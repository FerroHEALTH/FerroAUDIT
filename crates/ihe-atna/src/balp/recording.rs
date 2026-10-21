// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The time an audited exchange's record is stored within.
//!
//! An audited client records each exchange before it returns, and the
//! `timeout` its caller gives the exchange bounds that record too: a recorder
//! that has not accepted the record when the time is up leaves the exchange
//! with no stored record. An exchange that succeeded then fails with [`Late`]
//! as its audit failure, because an answer is used only once its record is
//! stored. An exchange that failed reports its own failure, which the late
//! record would only hide; the recorder goes on storing it, so the record is
//! not lost. No specification governs this bound: our own design, the one
//! ITI TF-2 §3.20.4.1.1 implies by asking a sender to keep every record.

use std::time::Duration;

/// Why an audited exchange failed: its audit record was not stored before
/// the exchange's time was up, so its answer is not used.
#[derive(Debug, Default, thiserror::Error)]
#[error("the audit record was not stored before the exchange's time was up")]
#[non_exhaustive]
pub struct Late;

impl Late {
    /// The failure of an exchange whose record was late, which an audited
    /// client returns as its audit error.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

/// What became of an exchange's audit record.
#[derive(Debug)]
pub enum Recorded<E> {
    /// The recorder accepted it.
    Accepted,
    /// The recorder refused it.
    Refused(E),
    /// The recorder had not accepted it when the exchange's time was up.
    Late,
}

/// The end of `timeout` from now, or `None` past what the clock holds.
#[must_use]
pub fn deadline(timeout: Duration) -> Option<tokio::time::Instant> {
    tokio::time::Instant::now().checked_add(timeout)
}

/// Awaits `record` until `deadline`, or for as long as it takes when there is
/// none.
pub async fn within<E>(
    deadline: Option<tokio::time::Instant>,
    record: impl Future<Output = Result<(), E>>,
) -> Recorded<E> {
    let outcome = match deadline {
        Some(deadline) => match tokio::time::timeout_at(deadline, record).await {
            Ok(outcome) => outcome,
            Err(_elapsed) => return Recorded::Late,
        },
        None => record.await,
    };
    match outcome {
        Ok(()) => Recorded::Accepted,
        Err(error) => Recorded::Refused(error),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{Recorded, deadline, within};

    #[tokio::test]
    async fn a_record_accepted_in_time_is_accepted() {
        let recorded = within(deadline(Duration::from_secs(5)), async { Ok::<(), ()>(()) }).await;
        assert!(matches!(recorded, Recorded::Accepted), "{recorded:?}");
    }

    #[tokio::test]
    async fn a_refusal_in_time_is_the_recorders_error() {
        let recorded = within(deadline(Duration::from_secs(5)), async {
            Err::<(), _>("full")
        })
        .await;
        assert!(
            matches!(recorded, Recorded::Refused("full")),
            "{recorded:?}"
        );
    }

    #[tokio::test]
    async fn a_record_not_accepted_when_the_time_is_up_is_late() {
        let recorded = within(deadline(Duration::from_millis(20)), async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok::<(), ()>(())
        })
        .await;
        assert!(matches!(recorded, Recorded::Late), "{recorded:?}");
    }

    #[tokio::test]
    async fn with_no_deadline_the_record_is_awaited() {
        let recorded = within(None, async {
            tokio::time::sleep(Duration::from_millis(20)).await;
            Ok::<(), ()>(())
        })
        .await;
        assert!(matches!(recorded, Recorded::Accepted), "{recorded:?}");
    }

    #[test]
    fn a_late_record_is_made_by_the_caller_that_reports_it() {
        assert_eq!(
            super::Late::new().to_string(),
            "the audit record was not stored before the exchange's time was up"
        );
    }

    #[test]
    fn a_timeout_past_the_clock_has_no_deadline() {
        assert!(
            deadline(Duration::MAX).is_none(),
            "no deadline past the clock"
        );
    }
}
