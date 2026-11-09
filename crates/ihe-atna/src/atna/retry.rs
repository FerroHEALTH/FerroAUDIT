// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! How the [`Forwarder`](super::forwarder::Forwarder) waits between failed
//! deliveries.
//!
//! An audit sender stores a record it cannot deliver and sends it once it
//! can (ITI TF-2 §3.20.4.1.1), so the forwarder never gives a record up.
//! It retries in rounds. Inside a round the first wait is
//! [`Retry::initial_backoff`] and each wait after it doubles, up to
//! [`Retry::max_backoff`]. After [`Retry::max_attempts`] failed attempts the
//! round ends: the forwarder waits [`Retry::max_backoff`] and starts the next
//! round from [`Retry::initial_backoff`]. A delivery that succeeds starts a
//! fresh round. Each wait is jittered to between half of it and all of it, so
//! senders that failed together do not retry together. No specification
//! governs the schedule: our own design.

use std::time::Duration;

/// The retry policy of a forwarder: the attempts of one round and the waits
/// between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retry {
    max_attempts: u32,
    initial_backoff: Duration,
    max_backoff: Duration,
}

/// Why a [`Retry`] cannot be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum RetryError {
    /// `max_attempts` is 0, so a round would hold no attempt.
    #[error("retry.max_attempts is 0; 1 tries once per round")]
    NoAttempt,
    /// `initial_backoff` is zero, so the forwarder would retry at once and
    /// keep doing so.
    #[error("retry.initial_backoff is zero")]
    NoBackoff,
    /// `initial_backoff` is longer than `max_backoff`.
    #[error("retry.initial_backoff is longer than retry.max_backoff")]
    BackoffPastBound,
}

impl Retry {
    /// The policy of `max_attempts` attempts a round, waiting
    /// `initial_backoff` after the first failure of a round, twice as long
    /// after each next one up to `max_backoff`, and `max_backoff` after the
    /// last failure of a round.
    ///
    /// # Errors
    ///
    /// [`RetryError::NoAttempt`] when `max_attempts` is 0,
    /// [`RetryError::NoBackoff`] when `initial_backoff` is zero, and
    /// [`RetryError::BackoffPastBound`] when `initial_backoff` is longer than
    /// `max_backoff`.
    pub fn new(
        max_attempts: u32,
        initial_backoff: Duration,
        max_backoff: Duration,
    ) -> Result<Self, RetryError> {
        if max_attempts == 0 {
            return Err(RetryError::NoAttempt);
        }
        if initial_backoff.is_zero() {
            return Err(RetryError::NoBackoff);
        }
        if initial_backoff > max_backoff {
            return Err(RetryError::BackoffPastBound);
        }
        Ok(Self {
            max_attempts,
            initial_backoff,
            max_backoff,
        })
    }

    /// The policy of the constructors that take only the longest wait: a
    /// first wait of 250 ms, doubling up to `max_backoff` (raised to 250 ms
    /// when shorter), with no end to a round.
    pub(crate) fn capped_at(max_backoff: Duration) -> Self {
        let initial_backoff = Duration::from_millis(250);
        Self {
            max_attempts: u32::MAX,
            initial_backoff,
            max_backoff: max_backoff.max(initial_backoff),
        }
    }

    /// The failed attempts of one round; after the last of them the
    /// forwarder waits [`Retry::max_backoff`] and starts the next round.
    #[must_use]
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// The wait after the first failed attempt of a round.
    #[must_use]
    pub fn initial_backoff(&self) -> Duration {
        self.initial_backoff
    }

    /// The longest wait between two attempts, and the wait between two
    /// rounds.
    #[must_use]
    pub fn max_backoff(&self) -> Duration {
        self.max_backoff
    }
}

#[cfg(feature = "smart")]
impl TryFrom<ferrohealth_smart::config::endpoint::RetrySection> for Retry {
    type Error = RetryError;

    /// The `retry` table of an outbound endpoint as the forwarder's policy:
    /// `max_attempts` bounds a round, never the delivery.
    fn try_from(
        section: ferrohealth_smart::config::endpoint::RetrySection,
    ) -> Result<Self, Self::Error> {
        Self::new(
            section.max_attempts,
            section.initial_backoff.into(),
            section.max_backoff.into(),
        )
    }
}

/// Where a forwarder stands in its retry rounds.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Backoff {
    policy: Retry,
    next: Duration,
    failures: u32,
}

impl Backoff {
    /// The first round of `policy`, before any failure.
    pub(crate) fn new(policy: Retry) -> Self {
        Self {
            policy,
            next: policy.initial_backoff,
            failures: 0,
        }
    }

    /// Counts a failed attempt and returns the wait before the next one.
    pub(crate) fn failed(&mut self) -> Duration {
        self.failures = self.failures.saturating_add(1);
        if self.failures >= self.policy.max_attempts {
            self.reset();
            return self.policy.max_backoff;
        }
        let wait = self.next;
        self.next = self.next.saturating_mul(2).min(self.policy.max_backoff);
        wait
    }

    /// Starts a fresh round, after a delivery that succeeded.
    pub(crate) fn reset(&mut self) {
        self.next = self.policy.initial_backoff;
        self.failures = 0;
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Backoff, Retry, RetryError};

    const MS: Duration = Duration::from_millis(1);

    fn waits(policy: Retry, failures: usize) -> Vec<Duration> {
        let mut backoff = Backoff::new(policy);
        (0..failures).map(|_| backoff.failed()).collect()
    }

    #[test]
    fn initial_backoff_is_the_first_wait_of_a_round() {
        let policy = Retry::new(5, 30 * MS, 1000 * MS).expect("a policy");
        assert_eq!(Some(&(30 * MS)), waits(policy, 1).first());
    }

    #[test]
    fn the_waits_of_a_round_double_up_to_max_backoff() {
        let policy = Retry::new(6, 100 * MS, 300 * MS).expect("a policy");
        assert_eq!(
            vec![100 * MS, 200 * MS, 300 * MS, 300 * MS, 300 * MS],
            waits(policy, 5)
        );
    }

    #[test]
    fn max_attempts_ends_a_round_with_max_backoff_and_the_next_round_starts_over() {
        let policy = Retry::new(3, 10 * MS, 1000 * MS).expect("a policy");
        assert_eq!(
            vec![
                10 * MS,
                20 * MS,
                1000 * MS,
                10 * MS,
                20 * MS,
                1000 * MS,
                10 * MS
            ],
            waits(policy, 7),
            "rounds of three attempts, never an end to delivery"
        );
    }

    #[test]
    fn one_attempt_a_round_waits_max_backoff_after_every_failure() {
        let policy = Retry::new(1, 10 * MS, 500 * MS).expect("a policy");
        assert_eq!(vec![500 * MS; 4], waits(policy, 4));
    }

    #[test]
    fn a_success_starts_a_fresh_round() {
        let policy = Retry::new(3, 10 * MS, 1000 * MS).expect("a policy");
        let mut backoff = Backoff::new(policy);
        assert_eq!(10 * MS, backoff.failed());
        assert_eq!(20 * MS, backoff.failed());
        backoff.reset();
        assert_eq!(10 * MS, backoff.failed(), "the first wait again");
        assert_eq!(20 * MS, backoff.failed());
        assert_eq!(
            1000 * MS,
            backoff.failed(),
            "three failures since the success"
        );
    }

    #[test]
    fn the_longest_wait_alone_keeps_the_former_schedule() {
        let policy = Retry::capped_at(1000 * MS);
        assert_eq!(
            vec![250 * MS, 500 * MS, 1000 * MS, 1000 * MS],
            waits(policy, 4)
        );
        assert_eq!(
            250 * MS,
            Retry::capped_at(MS).max_backoff(),
            "raised to the first wait"
        );
    }

    #[test]
    fn a_policy_without_an_attempt_is_refused() {
        assert_eq!(Err(RetryError::NoAttempt), Retry::new(0, MS, MS));
    }

    #[test]
    fn a_policy_without_a_wait_is_refused() {
        assert_eq!(
            Err(RetryError::NoBackoff),
            Retry::new(3, Duration::ZERO, MS)
        );
    }

    #[test]
    fn a_first_wait_past_the_longest_is_refused() {
        assert_eq!(Err(RetryError::BackoffPastBound), Retry::new(3, 2 * MS, MS));
    }

    #[cfg(feature = "smart")]
    #[test]
    fn a_retry_section_becomes_the_policy_field_by_field() {
        use ferrohealth_smart::config::endpoint::RetrySection;

        let section = RetrySection {
            max_attempts: 4,
            initial_backoff: "150ms".parse().expect("a duration"),
            max_backoff: "7s".parse().expect("a duration"),
        };
        let policy = Retry::try_from(section).expect("a policy");
        assert_eq!(4, policy.max_attempts());
        assert_eq!(150 * MS, policy.initial_backoff());
        assert_eq!(Duration::from_secs(7), policy.max_backoff());
    }

    #[cfg(feature = "smart")]
    #[test]
    fn a_retry_section_with_no_attempt_is_refused() {
        use ferrohealth_smart::config::endpoint::RetrySection;

        let section = RetrySection {
            max_attempts: 0,
            ..RetrySection::default()
        };
        assert_eq!(Err(RetryError::NoAttempt), Retry::try_from(section));
    }
}
