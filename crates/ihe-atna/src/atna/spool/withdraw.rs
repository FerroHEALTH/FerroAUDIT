// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! A write whose caller refuses its exchange when the write is late, so the
//! trail never holds a record of an exchange its client saw refused.
//!
//! On the timeout, a message still queued is withdrawn and the record of
//! the refusal takes its place in the queue, so the access is recorded once,
//! as refused. A message whose batch is already being written is waited
//! for, and the caller answers what the write did. No specification governs
//! the timing: our own design.

use secrecy::{ExposeSecret, SecretSlice};
use tokio::sync::oneshot;

use super::{Place, Spool, SpoolError, Waiting, lock};

/// One message and the record of its refusal, which takes the message's
/// place when it is withdrawn.
pub struct Withdrawable {
    /// The message, the record of the exchange as it ended.
    pub message: SecretSlice<u8>,
    /// The record of the exchange refused.
    pub refusal: SecretSlice<u8>,
}

impl std::fmt::Debug for Withdrawable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Withdrawable")
            .field("message", &self.message.expose_secret().len())
            .field("refusal", &self.refusal.expose_secret().len())
            .finish()
    }
}

impl Withdrawable {
    /// The larger of the two, the bytes its place holds.
    pub(super) fn size(&self) -> u64 {
        let size = |bytes: &SecretSlice<u8>| {
            u64::try_from(bytes.expose_secret().len()).unwrap_or(u64::MAX)
        };
        size(&self.message).max(size(&self.refusal))
    }
}

/// A message queued with its ticket, its refusal and its outcome.
struct Queued {
    ticket: u64,
    refusal: SecretSlice<u8>,
    outcome: oneshot::Receiver<Result<u64, SpoolError>>,
}

impl Spool {
    /// Stores the messages of `records`, each in its place of `places`, as
    /// one batch, or, when they are still queued at
    /// [`Bounds::write_timeout`](super::Bounds::write_timeout), withdraws
    /// every one and queues each refusal in its place.
    ///
    /// The messages join the queue under one lock and the write drains the
    /// queue under the same lock, so they are queued together or written
    /// together, never some of each.
    pub(super) async fn store_or_withdraw(
        &self,
        records: Vec<Withdrawable>,
        places: Vec<Place>,
    ) -> Result<(), SpoolError> {
        let bound = self.bounds.write_timeout;
        let mut queued = self.enqueue_all(records, places);
        self.write();
        let mut answers: Vec<Option<Result<(), SpoolError>>> =
            std::iter::repeat_with(|| None).take(queued.len()).collect();
        let Some(deadline) = tokio::time::Instant::now().checked_add(bound) else {
            return waited(&mut queued, &mut answers).await;
        };
        let timed = tokio::time::timeout_at(deadline, waited(&mut queued, &mut answers)).await;
        match timed {
            Ok(answer) => answer,
            Err(_elapsed) if self.withdraw(&mut queued) => Err(SpoolError::Withdrawn { bound }),
            // NOTE: no specification governs this: our own design; a batch being written
            // cannot be withdrawn, so its caller answers what the write did.
            Err(_elapsed) => waited(&mut queued, &mut answers).await,
        }
    }

    /// Queues every message of `records` in its place of `places`, under one
    /// lock, so no batch takes some of them and leaves the rest.
    fn enqueue_all(&self, records: Vec<Withdrawable>, places: Vec<Place>) -> Vec<Queued> {
        let mut waiting = lock(&self.waiting);
        records
            .into_iter()
            .zip(places)
            .map(|(record, place)| {
                let (done, outcome) = oneshot::channel();
                let ticket = self
                    .tickets
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                waiting.push_back(Waiting {
                    ticket,
                    message: record.message,
                    place,
                    done,
                });
                Queued {
                    ticket,
                    refusal: record.refusal,
                    outcome,
                }
            })
            .collect()
    }

    /// Puts each refusal of `queued` in the place of its message, keeping
    /// its place in the queue, or returns `false` when their batch has left
    /// the queue.
    fn withdraw(&self, queued: &mut [Queued]) -> bool {
        let mut withdrawn = Vec::with_capacity(queued.len());
        {
            let mut waiting = lock(&self.waiting);
            for one in queued.iter_mut() {
                let Some(held) = waiting.iter_mut().find(|held| held.ticket == one.ticket) else {
                    continue;
                };
                let (done, outcome) = oneshot::channel();
                held.message = std::mem::replace(&mut one.refusal, SecretSlice::from(Vec::new()));
                held.done = done;
                withdrawn.push(outcome);
            }
        }
        if withdrawn.is_empty() {
            return false;
        }
        drop(tokio::spawn(async move {
            for outcome in withdrawn {
                logged(outcome.await);
            }
        }));
        true
    }
}

/// Waits for the outcome of every message of `queued` not yet in `answers`,
/// and answers the first failure, or success.
async fn waited(
    queued: &mut [Queued],
    answers: &mut [Option<Result<(), SpoolError>>],
) -> Result<(), SpoolError> {
    for (one, answer) in queued.iter_mut().zip(answers.iter_mut()) {
        if answer.is_none() {
            *answer = Some(answered((&mut one.outcome).await));
        }
    }
    answers
        .iter_mut()
        .find_map(|answer| match answer.take() {
            Some(Err(error)) => Some(Err(error)),
            _ => None,
        })
        .unwrap_or(Ok(()))
}

/// Logs how the record of a refusal queued in place of a withdrawn message
/// ended.
fn logged(outcome: Result<Result<u64, SpoolError>, oneshot::error::RecvError>) {
    match outcome {
        Ok(Ok(sequence)) => tracing::warn!(target: crate::atna::targets::SPOOL,
            sequence,
            "an audit message missed its write timeout and was withdrawn; the record of its refusal was stored in its place"
        ),
        Ok(Err(error)) => tracing::error!(target: crate::atna::targets::SPOOL,
            error = %super::super::chain(&error),
            "the record of a refused exchange, queued in place of a withdrawn audit message, could not be stored"
        ),
        Err(_unfinished) => tracing::error!(target: crate::atna::targets::SPOOL,
            "the write of the record of a refused exchange, queued in place of a withdrawn audit message, did not finish"
        ),
    }
}

/// The caller's answer from the outcome of its message's write.
fn answered(
    outcome: Result<Result<u64, SpoolError>, oneshot::error::RecvError>,
) -> Result<(), SpoolError> {
    outcome
        .map_err(|_unfinished| SpoolError::Unfinished)?
        .map(|_sequence| ())
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use secrecy::{ExposeSecret as _, SecretSlice};

    use super::super::in_flight::tests::{BOUND, Logs, PATIENT, SLACK, Stall, naming_the_patient};
    use super::super::{Bounds, Depth, Spool, SpoolError, lock};
    use super::Withdrawable;

    /// The record of the refusal, as a caller writes it: one RFC 5425 frame.
    fn refusal() -> SecretSlice<u8> {
        let text = "<AuditMessage>refused</AuditMessage>";
        SecretSlice::from(format!("{} {text}", text.len()).into_bytes())
    }

    /// One record that names the patient, with its refusal.
    fn one() -> Vec<Withdrawable> {
        vec![Withdrawable {
            message: naming_the_patient(),
            refusal: refusal(),
        }]
    }

    fn spool(directory: &tempfile::TempDir) -> Spool {
        let bounds = Bounds {
            max_messages: 16,
            max_bytes: 1 << 20,
            write_timeout: BOUND,
        };
        Spool::open(&directory.path().join("spool"), bounds).expect("it opens")
    }

    /// Waits at most [`SLACK`] until `spool` holds `messages`.
    async fn holding(spool: &Spool, messages: usize) {
        let until = Instant::now() + SLACK;
        while spool.depth().messages < messages && Instant::now() < until {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    #[tokio::test]
    async fn a_message_stored_in_time_is_stored_alone() {
        let directory = tempfile::tempdir().expect("a directory");
        let spool = spool(&directory);
        spool
            .reserve(64)
            .expect("room")
            .push_or_withdraw(one())
            .await
            .expect("stored in time");
        assert_eq!(1, spool.depth().messages);
        assert_eq!(0, spool.queued(), "the place is given back");
        let stored = spool.oldest().await.expect("it reads").expect("one");
        assert!(String::from_utf8_lossy(stored.message.expose_secret()).contains(PATIENT));
    }

    #[tokio::test]
    async fn a_message_still_queued_at_its_bound_is_withdrawn_and_its_refusal_stored_in_its_place()
    {
        let logs = Logs::default();
        let _logging = tracing::subscriber::set_default(logs.subscriber());
        let directory = tempfile::tempdir().expect("a directory");
        let spool = spool(&directory);
        let turn = Stall::queue(&spool);

        let asked = Instant::now();
        let withdrawn = spool
            .reserve(64)
            .expect("room")
            .push_or_withdraw(one())
            .await;
        assert!(asked.elapsed() < BOUND + SLACK, "{:?}", asked.elapsed());
        assert!(
            matches!(withdrawn, Err(SpoolError::Withdrawn { bound }) if bound == BOUND),
            "{withdrawn:?}"
        );
        assert_eq!(Depth::default(), spool.depth(), "nothing is stored yet");
        assert_eq!(1, spool.queued(), "the refusal holds the place");

        turn.release();
        holding(&spool, 1).await;
        assert_eq!(1, spool.depth().messages, "one record of the exchange");
        assert_eq!(0, spool.queued());
        let stored = spool.oldest().await.expect("it reads").expect("one");
        assert_eq!(
            refusal().expose_secret(),
            stored.message.expose_secret(),
            "the record says the exchange was refused"
        );
        spool.remove(stored.sequence).await.expect("it removes");
        assert!(
            spool.oldest().await.expect("it reads").is_none(),
            "the withdrawn message is never stored"
        );

        let log = logs.wait_for("was withdrawn").await;
        assert!(log.contains("was withdrawn"), "{log}");
        assert!(!log.contains(PATIENT), "no log names the patient: {log}");
    }

    #[tokio::test]
    async fn a_message_whose_batch_is_being_written_is_waited_for_and_answered_by_its_write() {
        let directory = tempfile::tempdir().expect("a directory");
        let spool = spool(&directory);
        let stall = Stall::of(&spool);

        let pushing = spool.clone();
        let push = tokio::spawn(async move {
            pushing
                .reserve(64)
                .expect("room")
                .push_or_withdraw(one())
                .await
        });
        let until = Instant::now() + SLACK;
        while !lock(&spool.waiting).is_empty() && Instant::now() < until {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        tokio::time::sleep(BOUND + BOUND).await;
        assert!(
            !push.is_finished(),
            "past its bound, the caller waits for the batch being written"
        );

        stall.release();
        push.await
            .expect("the push ends")
            .expect("its write stored it, so the exchange stands");
        assert_eq!(1, spool.depth().messages);
        let stored = spool.oldest().await.expect("it reads").expect("one");
        assert!(
            String::from_utf8_lossy(stored.message.expose_secret()).contains(PATIENT),
            "the record of the exchange, never its refusal"
        );
        spool.remove(stored.sequence).await.expect("it removes");
        assert!(spool.oldest().await.expect("it reads").is_none());
    }

    #[tokio::test]
    async fn several_messages_still_queued_are_withdrawn_together() {
        let directory = tempfile::tempdir().expect("a directory");
        let spool = spool(&directory);
        let turn = Stall::queue(&spool);

        let three = (0..3)
            .map(|_| Withdrawable {
                message: naming_the_patient(),
                refusal: refusal(),
            })
            .collect();
        let withdrawn = spool
            .reserve(64)
            .expect("room")
            .push_or_withdraw(three)
            .await;
        assert!(
            matches!(withdrawn, Err(SpoolError::Withdrawn { .. })),
            "{withdrawn:?}"
        );
        assert_eq!(3, spool.queued(), "each refusal holds its message's place");

        turn.release();
        holding(&spool, 3).await;
        for _ in 0..3 {
            let stored = spool.oldest().await.expect("it reads").expect("one");
            assert_eq!(
                refusal().expose_secret(),
                stored.message.expose_secret(),
                "every record says the exchange was refused"
            );
            spool.remove(stored.sequence).await.expect("it removes");
        }
        assert!(spool.oldest().await.expect("it reads").is_none());
    }

    #[tokio::test]
    async fn records_the_bounds_have_no_room_for_are_refused_before_any_is_queued() {
        let directory = tempfile::tempdir().expect("a directory");
        let bounds = Bounds {
            max_messages: 2,
            max_bytes: 1 << 20,
            write_timeout: BOUND,
        };
        let spool = Spool::open(&directory.path().join("spool"), bounds).expect("it opens");
        let three = (0..3)
            .map(|_| Withdrawable {
                message: naming_the_patient(),
                refusal: refusal(),
            })
            .collect();
        let refused = spool
            .reserve(64)
            .expect("room")
            .push_or_withdraw(three)
            .await;
        assert!(
            matches!(refused, Err(SpoolError::Full { .. })),
            "{refused:?}"
        );
        assert_eq!(0, spool.queued(), "every place is given back");
        assert_eq!(Depth::default(), spool.depth(), "nothing is stored");
    }

    #[tokio::test]
    async fn no_records_give_the_place_back() {
        let directory = tempfile::tempdir().expect("a directory");
        let spool = spool(&directory);
        spool
            .reserve(64)
            .expect("room")
            .push_or_withdraw(Vec::new())
            .await
            .expect("nothing to store");
        assert_eq!(0, spool.queued());
        assert_eq!(Depth::default(), spool.depth());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_message_whose_batch_fails_is_answered_by_the_failure() {
        use std::os::unix::fs::PermissionsExt as _;

        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("spool");
        let spool = spool(&directory);
        let stall = Stall::of(&spool);

        let pushing = spool.clone();
        let push = tokio::spawn(async move {
            pushing
                .reserve(64)
                .expect("room")
                .push_or_withdraw(one())
                .await
        });
        tokio::time::sleep(BOUND + BOUND).await;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o500)).expect("chmod");
        stall.release();
        let failed = push.await.expect("the push ends");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).expect("chmod");
        assert!(matches!(failed, Err(SpoolError::Io { .. })), "{failed:?}");
        assert_eq!(Depth::default(), spool.depth(), "nothing is stored");
        assert_eq!(0, spool.queued());
    }
}
