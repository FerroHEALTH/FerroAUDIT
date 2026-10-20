// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! A forwarder over a caller's own durable store: ITI TF-2 §3.20.4.1.1 has a
//! sender store a record and send it when it can, and where it stores it is
//! the sender's. The forwarder delivers oldest first, removes each delivered
//! message from the caller's store, and moves a refused one aside.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use ihe_atna::atna::forwarder::{Forwarder, Outbox, OutboxError};
#[cfg(feature = "atna")]
use ihe_atna::atna::repository::{Repository, Timeouts};
use ihe_atna::atna::spool::{Depth, Stored};
use secrecy::SecretSlice;
#[cfg(feature = "atna")]
use tokio::io::AsyncReadExt as _;
#[cfg(feature = "atna")]
use tokio::net::TcpListener;
use url::Url;

use crate::timing;

/// A caller's store: a queue in memory, each message by its sequence number,
/// with what the forwarder removed and moved aside.
#[derive(Debug, Default)]
pub(crate) struct Table {
    waiting: Mutex<VecDeque<(u64, Vec<u8>)>>,
    removed: Mutex<Vec<u64>>,
    quarantined: Mutex<Vec<u64>>,
}

fn held<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Table {
    pub(crate) fn holding(messages: &[&[u8]]) -> Arc<Self> {
        let table = Self::default();
        held(&table.waiting).extend(
            messages
                .iter()
                .zip(1_u64..)
                .map(|(bytes, sequence)| (sequence, bytes.to_vec())),
        );
        Arc::new(table)
    }

    pub(crate) fn removed(&self) -> Vec<u64> {
        held(&self.removed).clone()
    }

    #[cfg(feature = "balp")]
    pub(crate) fn quarantined(&self) -> Vec<u64> {
        held(&self.quarantined).clone()
    }
}

#[async_trait::async_trait]
impl Outbox for Table {
    async fn oldest(&self) -> Result<Option<Stored>, OutboxError> {
        Ok(held(&self.waiting).front().map(|(sequence, bytes)| Stored {
            sequence: *sequence,
            message: SecretSlice::from(bytes.clone()),
        }))
    }

    async fn stored(&self) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    async fn remove(&self, sequence: u64) -> Result<(), OutboxError> {
        held(&self.waiting).retain(|(kept, _)| *kept != sequence);
        held(&self.removed).push(sequence);
        Ok(())
    }

    async fn quarantine(&self, sequence: u64) -> Result<(), OutboxError> {
        held(&self.waiting).retain(|(kept, _)| *kept != sequence);
        held(&self.quarantined).push(sequence);
        Ok(())
    }

    fn depth(&self) -> Depth {
        Depth {
            messages: held(&self.waiting).len(),
            ..Depth::default()
        }
    }

    fn is_durable(&self) -> bool {
        true
    }
}

/// The RFC 5425 frame of `text`.
#[cfg(feature = "atna")]
fn framed(text: &str) -> Vec<u8> {
    format!("{} {text}", text.len()).into_bytes()
}

#[cfg(feature = "atna")]
#[tokio::test]
async fn a_callers_store_is_delivered_oldest_first_and_emptied() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a listener");
    let url = Url::parse(&format!(
        "tcp://{}",
        listener.local_addr().expect("an address")
    ))
    .expect("a URL");
    let timeouts = Timeouts {
        connect: timing::SLACK,
        send: timing::SLACK,
    };
    let repository = Repository::unencrypted_for_development(&url, timeouts).expect("a repository");
    let (first, second) = (framed("the first record"), framed("the second record"));
    let table = Table::holding(&[&first, &second]);
    let forwarder =
        Forwarder::syslog_from(Arc::clone(&table), repository, Duration::from_millis(400));
    assert!(forwarder.status().durable, "the caller's store says so");
    let running = tokio::spawn(Arc::clone(&forwarder).run());

    let (mut stream, _) = tokio::time::timeout(timing::SLACK, listener.accept())
        .await
        .expect("the forwarder connects")
        .expect("a connection");
    let mut received = vec![0_u8; first.len() + second.len()];
    tokio::time::timeout(timing::SLACK, stream.read_exact(&mut received))
        .await
        .expect("both records arrive")
        .expect("they read");
    assert_eq!(received, [first, second].concat(), "oldest first");

    let deadline = Instant::now() + timing::SLACK;
    while table.removed().len() < 2 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert_eq!(table.removed(), vec![1, 2], "each removed once delivered");
    assert_eq!(forwarder.status().delivered, 2);
    assert_eq!(forwarder.status().depth.messages, 0);
    running.abort();
}

/// A record the FHIR Feed repository refused is moved aside in the caller's
/// store and the drain goes on (the `RESTful` ATNA supplement §3.20.4.3.3
/// leaves the failure to the client).
#[cfg(feature = "balp")]
#[tokio::test]
async fn a_refused_record_is_moved_aside_in_the_callers_store() {
    use ihe_atna::atna::feed::FeedRepository;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400))
        .mount(&server)
        .await;
    let base = Url::parse(&format!("{}/arr/", server.uri())).expect("a base");
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("an HTTP client");
    let feed = FeedRepository::cleartext(base, http, timing::SLACK).expect("a repository");
    let table = Table::holding(&[br#"{"resourceType":"AuditEvent"}"#]);
    let forwarder = Forwarder::fhir_feed_from(Arc::clone(&table), feed, Duration::from_millis(400));
    let running = tokio::spawn(Arc::clone(&forwarder).run());
    let deadline = Instant::now() + timing::SLACK;
    while table.quarantined().is_empty() && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    running.abort();
    assert_eq!(table.quarantined(), vec![1], "moved aside");
    assert!(table.removed().is_empty(), "never counted as delivered");
}
