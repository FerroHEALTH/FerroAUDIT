// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The FHIR Feed's credential, which `ferrohealth-smart`'s outbound client
//! carries as its endpoint table names it: every request carries HTTP Basic
//! or the client credentials token, a `401` is answered with a token fetched
//! anew once, as RFC 6750 §3.1 has a client do on an invalid token, and a
//! `401` is the credential's fault, never the record's, so the record is
//! never quarantined for it. The cleartext constructor reaches an `http`
//! base by name; `new` refuses it.

use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine as _;
use ferrohealth_smart::config::credentials::CredentialsSection;
use ferrohealth_smart::config::endpoint::EndpointSection;
use ihe_atna::atna::feed::{FeedConfigError, FeedError, FeedRepository};
use ihe_atna::atna::forwarder::Forwarder;
use ihe_atna::atna::retry::RetryError;
use ihe_atna::atna::spool::{Bounds, Content, Spool};
use ihe_atna::balp::{
    DESTINATION_ROLE, Direction, EventKind, Exchange, Outcome, Peer, REST, SEARCH, SOURCE_ROLE,
};
use ihe_atna::user::OnBehalfOf;
use secrecy::SecretSlice;
use serde_json::json;
use url::Url;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::observer;
use super::outbound::{basic, client, grant, repository, retry, synthetic_password, table};
use crate::timing;

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

/// A token response of RFC 6749 §5.1 with the synthetic token `token`.
fn token_response(token: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({
        "access_token": token,
        "token_type": "Bearer",
        "expires_in": 300
    }))
}

/// Every request carries the HTTP Basic credential the table names (RFC
/// 7617 §2).
#[tokio::test]
async fn a_basic_credential_is_carried_on_each_request() {
    let server = MockServer::start().await;
    let password = synthetic_password();
    let expected = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!("Qz7-audit-user:{password}"))
    );
    Mock::given(method("POST"))
        .and(path("/arr/AuditEvent"))
        .and(header("authorization", expected.as_str()))
        .respond_with(ResponseTemplate::new(201))
        .expect(2)
        .mount(&server)
        .await;
    let feed = repository(&server, basic("Qz7-audit-user", &password));
    for _ in 0..2 {
        feed.send(&record()).await.expect("the credential is taken");
    }
}

/// The repository refuses the first token; the source fetches a second,
/// which the repository takes.
#[tokio::test]
async fn a_refused_token_is_fetched_anew_and_the_record_sent_with_it() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(token_response("Qz7-first"))
        .up_to_n_times(1)
        .with_priority(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(token_response("Qz7-second"))
        .with_priority(2)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/arr/AuditEvent"))
        .and(header("authorization", "Bearer Qz7-first"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/arr/AuditEvent"))
        .and(header("authorization", "Bearer Qz7-second"))
        .respond_with(ResponseTemplate::new(201))
        .expect(1)
        .mount(&server)
        .await;
    let feed = repository(&server, grant(&server));
    feed.send(&record())
        .await
        .expect("the token fetched anew is taken");
}

/// A token refused twice is `Unauthorized`, with no third token asked for,
/// and so is a `401` to a repository with no credential.
#[tokio::test]
async fn a_token_refused_twice_is_unauthorized_and_asked_for_no_third_time() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(token_response("Qz7-refused"))
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/arr/AuditEvent"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&server)
        .await;
    let feed = repository(&server, grant(&server));
    let refused = feed.send(&record()).await;
    assert!(
        matches!(refused, Err(FeedError::Unauthorized)),
        "{refused:?}"
    );
    let bare = repository(&server, CredentialsSection::default())
        .send(&record())
        .await;
    assert!(matches!(bare, Err(FeedError::Unauthorized)), "{bare:?}");
}

/// A token endpoint that refuses the grant leaves no credential, and the
/// record is not sent.
#[tokio::test]
async fn a_refused_grant_is_a_credentials_error_and_sends_nothing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({
            "error": "invalid_client"
        })))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/arr/AuditEvent"))
        .respond_with(ResponseTemplate::new(201))
        .expect(0)
        .mount(&server)
        .await;
    let feed = repository(&server, grant(&server));
    let refused = feed.send(&record()).await;
    assert!(
        matches!(refused, Err(FeedError::Credentials(_))),
        "{refused:?}"
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
    let forwarder = Forwarder::fhir_feed(spool, repository(&server, CredentialsSection::default()));
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
/// repository's `Debug` shows that it carries an authorization and not its
/// value.
#[tokio::test]
async fn the_cleartext_constructor_reaches_http_and_new_refuses_it() {
    let server = MockServer::start().await;
    let password = synthetic_password();
    let with_basic = EndpointSection {
        credentials: basic("Qz7-audit-user", &password),
        ..table(&server)
    };
    assert_eq!(
        FeedRepository::new(client(&with_basic)).err(),
        Some(FeedConfigError::Cleartext)
    );
    let accepted =
        FeedRepository::cleartext(client(&with_basic)).expect("an http base accepted by name");
    assert_eq!(
        accepted.endpoint().as_str(),
        format!("{}/arr/AuditEvent", server.uri())
    );
    let shown = format!("{accepted:?}");
    assert!(!shown.contains("Qz7-audit"), "{shown}");
    assert!(!shown.contains(&password), "{shown}");
    assert!(shown.contains("authorization: true"), "{shown}");
}

/// An endpoint whose `retry` table leaves no wait between two rounds is
/// refused, so a failing repository is never asked again at once, forever.
#[tokio::test]
async fn an_endpoint_without_a_wait_between_rounds_is_refused() {
    let server = MockServer::start().await;
    let immediate = EndpointSection {
        retry: retry(3, "0ms", "0ms"),
        ..table(&server)
    };
    assert_eq!(
        FeedRepository::cleartext(client(&immediate)).err(),
        Some(FeedConfigError::Retry(RetryError::NoBackoff))
    );
}
