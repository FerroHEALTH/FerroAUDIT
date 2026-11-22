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
///
/// The refusal is held written ([`Withdrawable::new`]) or, for a BALP
/// record ([`Exchange::withdrawable`](crate::balp::Exchange::withdrawable)),
/// as its typed resource of a known length, written only when the message
/// is withdrawn.
pub struct Withdrawable {
    /// The message, the record of the exchange as it ended.
    message: SecretSlice<u8>,
    /// The record of the exchange refused.
    refusal: Refusal,
}

/// The record of a refused exchange, written or to be written.
enum Refusal {
    /// Written already.
    Written(SecretSlice<u8>),
    /// The typed resource, written on withdrawal into `size` bytes.
    #[cfg(feature = "balp")]
    Typed {
        /// The resource, its outcome the refusal's.
        resource: Box<crate::balp::AuditEventResource>,
        /// The length of its bytes.
        size: u64,
    },
}

impl Refusal {
    /// The length of the refusal's bytes.
    fn size(&self) -> u64 {
        match self {
            Self::Written(bytes) => length(bytes),
            #[cfg(feature = "balp")]
            Self::Typed { size, .. } => *size,
        }
    }

    /// The refusal's bytes, or `None` when they cannot be written within
    /// `room` bytes, which writing a typed resource to memory does not do.
    fn written(self, room: u64) -> Option<SecretSlice<u8>> {
        let bytes = match self {
            Self::Written(bytes) => bytes,
            #[cfg(feature = "balp")]
            Self::Typed { resource, .. } => match resource.to_record() {
                Ok(record) => record.into_bytes(),
                Err(error) => {
                    tracing::error!(target: crate::atna::targets::SPOOL,
                        error = %super::super::chain(&error),
                        "the record of a refused exchange could not be written, so its message is not withdrawn"
                    );
                    return None;
                }
            },
        };
        if length(&bytes) > room {
            tracing::error!(target: crate::atna::targets::SPOOL,
                "the record of a refused exchange is larger than its message's place, so its message is not withdrawn"
            );
            return None;
        }
        Some(bytes)
    }
}

/// The length of `bytes`.
fn length(bytes: &SecretSlice<u8>) -> u64 {
    u64::try_from(bytes.expose_secret().len()).unwrap_or(u64::MAX)
}

impl std::fmt::Debug for Withdrawable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Withdrawable")
            .field("message", &self.message.expose_secret().len())
            .field("refusal", &self.refusal.size())
            .finish()
    }
}

impl Withdrawable {
    /// The message `message`, the record of the exchange as it ended, and
    /// `refusal`, the record of the exchange refused, both written.
    #[must_use]
    pub fn new(message: SecretSlice<u8>, refusal: SecretSlice<u8>) -> Self {
        Self {
            message,
            refusal: Refusal::Written(refusal),
        }
    }

    /// The message and the typed refusal of `size` bytes, which is written
    /// only when the message is withdrawn.
    #[cfg(feature = "balp")]
    pub(crate) fn typed(
        message: SecretSlice<u8>,
        resource: crate::balp::AuditEventResource,
        size: u64,
    ) -> Self {
        Self {
            message,
            refusal: Refusal::Typed {
                resource: Box::new(resource),
                size,
            },
        }
    }

    /// The message, the record of the exchange as it ended.
    #[must_use]
    pub fn message(&self) -> &SecretSlice<u8> {
        &self.message
    }

    /// The record of the exchange refused, as it takes the message's place
    /// when the message is withdrawn; `None` when it cannot be written within
    /// that place, which writing to memory does not do.
    #[must_use]
    pub fn into_refusal(self) -> Option<SecretSlice<u8>> {
        let room = self.size();
        self.refusal.written(room)
    }

    /// The larger of the two, the bytes its place holds.
    pub(super) fn size(&self) -> u64 {
        length(&self.message).max(self.refusal.size())
    }
}

/// A message queued with its ticket, its refusal and its outcome.
struct Queued {
    ticket: u64,
    refusal: Option<Refusal>,
    room: u64,
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
                let room = place.size;
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
                    refusal: Some(record.refusal),
                    room,
                    outcome,
                }
            })
            .collect()
    }

    /// Puts each refusal of `queued` in the place of its message, keeping
    /// its place in the queue, or returns `false` when their batch has left
    /// the queue or a refusal could not be written.
    ///
    /// Every refusal is written before the queue is locked, so the lock is
    /// never held while one is written, and a message whose refusal could not
    /// be written is not withdrawn: its caller waits for its write instead.
    fn withdraw(&self, queued: &mut [Queued]) -> bool {
        let mut refusals = Vec::with_capacity(queued.len());
        for one in queued.iter_mut() {
            let Some(refusal) = one
                .refusal
                .take()
                .and_then(|refusal| refusal.written(one.room))
            else {
                return false;
            };
            refusals.push(refusal);
        }
        let mut withdrawn = Vec::with_capacity(queued.len());
        {
            let mut waiting = lock(&self.waiting);
            if !queued
                .iter()
                .all(|one| waiting.iter().any(|held| held.ticket == one.ticket))
            {
                return false;
            }
            for (one, refusal) in queued.iter().zip(refusals) {
                let Some(held) = waiting.iter_mut().find(|held| held.ticket == one.ticket) else {
                    continue;
                };
                let (done, outcome) = oneshot::channel();
                held.message = refusal;
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
        vec![Withdrawable::new(naming_the_patient(), refusal())]
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
            .map(|_| Withdrawable::new(naming_the_patient(), refusal()))
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
            .map(|_| Withdrawable::new(naming_the_patient(), refusal()))
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

    /// A system's read, its record written as BALP JSON.
    #[cfg(feature = "balp")]
    fn exchange() -> (crate::balp::Exchange, crate::balp::Observer) {
        use crate::balp::{
            DESTINATION_ROLE, Direction, EventKind, Exchange, NetworkAddress, Observer, Outcome,
            Peer, REST, SOURCE_ROLE,
        };
        let exchange = Exchange {
            kind: EventKind {
                profile: "",
                event_type: REST,
                subtypes: Vec::new(),
                action: "R",
                client: SOURCE_ROLE,
                server: DESTINATION_ROLE,
            },
            recorded: jiff::Timestamp::UNIX_EPOCH,
            outcome: Outcome::MajorFailure,
            outcome_description: None,
            direction: Direction::Sent {
                server: Peer {
                    who: "Qz7-server".to_owned(),
                    network: NetworkAddress::host("2001:db8::1"),
                },
            },
            on_behalf: crate::user::OnBehalfOf::System,
            entities: Vec::new(),
            security: Vec::new(),
        };
        let observer = Observer {
            source_id: "gateway.example.org".to_owned(),
            site: None,
            host: NetworkAddress::host("198.51.100.4"),
            source_types: Vec::new(),
        };
        (exchange, observer)
    }

    #[cfg(feature = "balp")]
    #[tokio::test]
    async fn a_typed_refusal_is_written_into_the_length_it_reserved_when_its_message_is_withdrawn()
    {
        let (exchange, observer) = exchange();
        let refused = crate::balp::Exchange {
            outcome: crate::balp::Outcome::SeriousFailure,
            ..exchange.clone()
        }
        .audit_event(&observer)
        .expect("the record")
        .into_bytes();
        let pair = exchange.withdrawable(&observer).expect("the records");
        assert_eq!(
            u64::try_from(refused.expose_secret().len()).expect("a length"),
            pair.refusal.size(),
            "the length reserved is the refusal's"
        );
        let directory = tempfile::tempdir().expect("a directory");
        let spool = Spool::open_for(
            &directory.path().join("spool"),
            Bounds {
                max_messages: 16,
                max_bytes: 1 << 20,
                write_timeout: BOUND,
            },
            super::super::Content::AuditEvents,
        )
        .expect("it opens");
        let turn = Stall::queue(&spool);
        let withdrawn = spool
            .reserve(64)
            .expect("room")
            .push_or_withdraw(vec![pair])
            .await;
        assert!(
            matches!(withdrawn, Err(SpoolError::Withdrawn { .. })),
            "{withdrawn:?}"
        );
        turn.release();
        holding(&spool, 1).await;
        let stored = spool.oldest().await.expect("it reads").expect("one");
        assert_eq!(
            refused.expose_secret(),
            stored.message.expose_secret(),
            "the record says the exchange was refused"
        );
    }

    #[cfg(feature = "balp")]
    #[tokio::test]
    async fn a_message_whose_refusal_does_not_fit_its_place_is_not_withdrawn_and_is_stored() {
        let (exchange, observer) = exchange();
        let resource = exchange.audit_event_resource(&observer);
        let message = naming_the_patient();
        let pair = Withdrawable {
            message,
            refusal: super::Refusal::Typed {
                resource: Box::new(resource),
                size: 1,
            },
        };
        let directory = tempfile::tempdir().expect("a directory");
        let spool = spool(&directory);
        let turn = Stall::queue(&spool);
        let releasing = tokio::spawn(async move {
            tokio::time::sleep(BOUND + BOUND).await;
            turn.release();
        });
        let stored = spool
            .reserve(64)
            .expect("room")
            .push_or_withdraw(vec![pair])
            .await;
        releasing.await.expect("the release runs");
        assert!(stored.is_ok(), "the message is waited for: {stored:?}");
        let stored = spool.oldest().await.expect("it reads").expect("one");
        assert_eq!(
            naming_the_patient().expose_secret(),
            stored.message.expose_secret(),
            "the record of the exchange as it ended is stored"
        );
    }
}
