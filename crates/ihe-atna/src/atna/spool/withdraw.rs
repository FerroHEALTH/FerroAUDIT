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

use secrecy::SecretSlice;
use tokio::sync::oneshot;

use super::{Place, Spool, SpoolError, lock};

impl Spool {
    /// Stores `message` in its `place`, or, when it is still queued at
    /// [`Bounds::write_timeout`](super::Bounds::write_timeout), withdraws it
    /// and queues `refusal` in its place. The place holds the larger of the
    /// two.
    pub(super) async fn store_or_withdraw(
        &self,
        message: SecretSlice<u8>,
        refusal: SecretSlice<u8>,
        place: Place,
    ) -> Result<(), SpoolError> {
        let bound = self.bounds.write_timeout;
        let (ticket, mut outcome) = self.enqueue(message, place);
        let turn = self.write_turn();
        drop(tokio::spawn(async move {
            if let Err(error) = turn.await {
                tracing::error!(target: crate::atna::targets::SPOOL,
                    error = %super::super::chain(&error),
                    "the task writing a batch of audit messages did not end"
                );
            }
        }));
        let Some(deadline) = tokio::time::Instant::now().checked_add(bound) else {
            return answered(outcome.await);
        };
        match tokio::time::timeout_at(deadline, &mut outcome).await {
            Ok(stored) => answered(stored),
            Err(_elapsed) if self.withdraw(ticket, refusal) => Err(SpoolError::Withdrawn { bound }),
            // NOTE: no specification governs this: our own design; a batch being written
            // cannot be withdrawn, so its caller answers what the write did.
            Err(_elapsed) => answered(outcome.await),
        }
    }

    /// Puts `refusal` in the place of the queued message `ticket`, keeping
    /// its place in the queue, or returns `false` when its batch has left
    /// the queue.
    fn withdraw(&self, ticket: u64, refusal: SecretSlice<u8>) -> bool {
        let (done, outcome) = oneshot::channel();
        {
            let mut waiting = lock(&self.waiting);
            let Some(queued) = waiting.iter_mut().find(|queued| queued.ticket == ticket) else {
                return false;
            };
            queued.message = refusal;
            queued.done = done;
        }
        drop(tokio::spawn(async move {
            match outcome.await {
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
        }));
        true
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
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use secrecy::{ExposeSecret as _, SecretSlice};

    use super::super::in_flight::tests::{BOUND, Logs, PATIENT, SLACK, Stall, naming_the_patient};
    use super::super::{Bounds, Depth, Spool, SpoolError, lock};

    /// The record of the refusal, as a caller writes it: one RFC 5425 frame.
    fn refusal() -> SecretSlice<u8> {
        let text = "<AuditMessage>refused</AuditMessage>";
        SecretSlice::from(format!("{} {text}", text.len()).into_bytes())
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
            .push_or_withdraw(naming_the_patient(), refusal())
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
        let turn = Arc::clone(&spool.turn).lock_owned().await;

        let asked = Instant::now();
        let withdrawn = spool
            .reserve(64)
            .expect("room")
            .push_or_withdraw(naming_the_patient(), refusal())
            .await;
        assert!(asked.elapsed() < BOUND + SLACK, "{:?}", asked.elapsed());
        assert!(
            matches!(withdrawn, Err(SpoolError::Withdrawn { bound }) if bound == BOUND),
            "{withdrawn:?}"
        );
        assert_eq!(Depth::default(), spool.depth(), "nothing is stored yet");
        assert_eq!(1, spool.queued(), "the refusal holds the place");

        drop(turn);
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
                .push_or_withdraw(naming_the_patient(), refusal())
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
                .push_or_withdraw(naming_the_patient(), refusal())
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
