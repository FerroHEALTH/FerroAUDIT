// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The FHIR Feed's credential source: every request carries the value it
//! gives, a `401` is answered with a fresh one once, as RFC 6750 §3.1 has a
//! client do on an invalid token, and a `401` is the credential's fault,
//! never the record's, so the record is never quarantined for it. The
//! cleartext constructor reaches an `http` base by name; `new` refuses it.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use ihe_atna::atna::feed::{
    Credentials, CredentialsError, FeedAddressError, FeedError, FeedRepository,
};
use ihe_atna::atna::forwarder::Forwarder;
use ihe_atna::atna::spool::{Bounds, Content, Spool};
use ihe_atna::balp::{
    DESTINATION_ROLE, Direction, EventKind, Exchange, Outcome, Peer, REST, SEARCH, SOURCE_ROLE,
};
use ihe_atna::user::OnBehalfOf;
use secrecy::{SecretSlice, SecretString};
use url::Url;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::observer;
use crate::timing;

/// A token cache: the value `Bearer Qz7-token-<n>`, renewed each time the
/// repository refuses it.
#[derive(Debug, Default)]
struct Tokens {
    issued: AtomicUsize,
    refused: AtomicUsize,
}

#[async_trait::async_trait]
impl Credentials for Tokens {
    async fn authorization(&self) -> Result<SecretString, CredentialsError> {
        let current = self.refused.load(Ordering::SeqCst);
        self.issued.fetch_add(1, Ordering::SeqCst);
        Ok(SecretString::from(format!("Bearer Qz7-token-{current}")))
    }

    async fn refused(&self, _refused: &SecretString) {
        self.refused.fetch_add(1, Ordering::SeqCst);
    }
}

/// A source that can give no value.
#[derive(Debug)]
struct Unavailable;

#[derive(Debug, thiserror::Error)]
#[error("the authorization server cannot be reached")]
struct Down;

#[async_trait::async_trait]
impl Credentials for Unavailable {
    async fn authorization(&self) -> Result<SecretString, CredentialsError> {
        Err(CredentialsError(Box::new(Down)))
    }

    async fn refused(&self, _refused: &SecretString) {}
}

fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("an HTTP client")
}

pub(crate) fn repository(server: &MockServer) -> FeedRepository {
    let base = Url::parse(&format!("{}/arr/", server.uri())).expect("a base");
    FeedRepository::cleartext(base, http(), timing::SLACK).expect("a repository")
}

/// One synthetic record of an exchange the system made on its own behalf.
pub(crate) fn record() -> SecretSlice<u8> {
    Exchange {
        kind: EventKind {
            profile: "",
            event_type: REST,
            subtypes: vec![SEARCH],
            action: "E",
            client: SOURCE_ROLE,
            server: DESTINATION_ROLE,
        },
        recorded: jiff::Timestamp::UNIX_EPOCH,
        outcome: Outcome::Success,
        outcome_description: None,
        direction: Direction::Sent {
            server: Peer::server(&Url::parse("https://pix.example.org/fhir/").expect("a URL")),
        },
        on_behalf: OnBehalfOf::System,
        entities: Vec::new(),
        security: Vec::new(),
    }
    .audit_event(&observer())
    .expect("a record")
    .into_bytes()
}

/// The repository refuses the first token and takes the renewed one.
#[tokio::test]
async fn a_refused_token_is_renewed_once_and_the_record_sent_with_it() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/arr/AuditEvent"))
        .and(header("authorization", "Bearer Qz7-token-0"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/arr/AuditEvent"))
        .and(header("authorization", "Bearer Qz7-token-1"))
        .respond_with(ResponseTemplate::new(201))
        .mount(&server)
        .await;
    let tokens = Arc::new(Tokens::default());
    let source: Arc<dyn Credentials> = tokens.clone();
    let feed = repository(&server).with_credentials(source);
    feed.send(&record())
        .await
        .expect("the renewed token is taken");
    assert_eq!(tokens.refused.load(Ordering::SeqCst), 1, "renewed once");
    assert_eq!(tokens.issued.load(Ordering::SeqCst), 2, "asked twice");
}

/// A token refused twice is `Unauthorized`, and so is a `401` with no source.
#[tokio::test]
async fn a_token_refused_twice_is_unauthorized_and_asked_for_no_third_time() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let tokens = Arc::new(Tokens::default());
    let source: Arc<dyn Credentials> = tokens.clone();
    let feed = repository(&server).with_credentials(source);
    let refused = feed.send(&record()).await;
    assert!(
        matches!(refused, Err(FeedError::Unauthorized)),
        "{refused:?}"
    );
    assert_eq!(
        tokens.issued.load(Ordering::SeqCst),
        2,
        "one retry, no more"
    );
    let bare = repository(&server).send(&record()).await;
    assert!(matches!(bare, Err(FeedError::Unauthorized)), "{bare:?}");
}

#[tokio::test]
async fn a_source_that_gives_no_value_fails_the_send_before_it_is_made() {
    let server = MockServer::start().await;
    let feed = repository(&server).with_credentials(Arc::new(Unavailable));
    let failed = feed.send(&record()).await;
    assert!(
        matches!(failed, Err(FeedError::Credentials(_))),
        "{failed:?}"
    );
    assert!(
        server
            .received_requests()
            .await
            .unwrap_or_default()
            .is_empty(),
        "nothing is sent without a credential"
    );
}

/// A `401` leaves the record stored and retried, never quarantined.
#[tokio::test]
async fn an_unauthorized_record_stays_spooled_and_is_never_quarantined() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let directory = tempfile::tempdir().expect("a directory");
    let bounds = Bounds {
        max_messages: 8,
        max_bytes: 1 << 20,
        write_timeout: Duration::from_secs(10),
    };
    let spool = Spool::open_for(
        &directory.path().join("spool"),
        bounds,
        Content::AuditEvents,
    )
    .expect("it opens");
    let forwarder = Forwarder::fhir_feed(spool, repository(&server), Duration::from_millis(300));
    forwarder.submit(record()).await.expect("stored");
    let running = tokio::spawn(Arc::clone(&forwarder).run());
    let deadline = Instant::now() + timing::SLACK;
    while forwarder.status().retries < 2 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    running.abort();
    let status = forwarder.status();
    assert!(status.retries >= 2, "retried: {status:?}");
    assert_eq!(status.depth.quarantined, 0, "never quarantined");
    assert_eq!(status.depth.messages, 1, "still stored");
}

/// `new` refuses an `http` base, `cleartext` reaches it by name, and the
/// repository's `Debug` shows that it has a credential source and no value.
#[test]
fn the_cleartext_constructor_reaches_http_and_new_refuses_it() {
    let base = Url::parse("http://arr.example.org/fhir/").expect("a URL");
    assert_eq!(
        FeedRepository::new(base.clone(), http(), Duration::from_secs(1)).err(),
        Some(FeedAddressError::Cleartext)
    );
    let accepted = FeedRepository::cleartext(base, http(), Duration::from_secs(1))
        .expect("an http base accepted by name");
    assert_eq!(
        accepted.endpoint().as_str(),
        "http://arr.example.org/fhir/AuditEvent"
    );
    let shown = format!(
        "{:?}",
        accepted.with_credentials(Arc::new(Tokens::default()))
    );
    assert!(!shown.contains("Qz7-token"), "{shown}");
    assert!(shown.contains("credentials: true"), "{shown}");
}
