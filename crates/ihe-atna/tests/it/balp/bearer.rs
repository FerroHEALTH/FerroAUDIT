// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The client credentials token of `ferrohealth-smart` as the Feed's
//! credential source: each request carries the cached token, and a `401`
//! drops it so the record is sent once more under a token fetched anew (RFC
//! 6750 §3.1).

use std::sync::Arc;

use ferrohealth_smart::client::cache::TokenSource;
use ferrohealth_smart::client::{ClientCredentials, Endpoint};
use ihe_atna::atna::bearer::Bearer;
use ihe_atna::atna::feed::{Credentials, FeedError};
use secrecy::SecretString;
use url::Url;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::credentials::{record, repository};

/// A token response of RFC 6749 §5.1 with the synthetic token `token`.
fn token_response(token: &str) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(serde_json::json!({
        "access_token": token,
        "token_type": "Bearer",
        "expires_in": 300
    }))
}

/// The bearer source of a grant at `server`'s token endpoint.
fn bearer(server: &MockServer) -> Arc<dyn Credentials> {
    let endpoint = Url::parse(&format!("{}/token", server.uri())).expect("a token endpoint URL");
    let grant = ClientCredentials::new(
        Endpoint::Token(endpoint),
        "Qz7-audit-client".to_owned(),
        SecretString::from("Qz7-audit-secret"),
    );
    Arc::new(Bearer::from(
        TokenSource::new(grant).expect("a token source"),
    ))
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
    let feed = repository(&server).with_credentials(bearer(&server));
    feed.send(&record())
        .await
        .expect("the token fetched anew is taken");
}

/// A token endpoint that refuses the grant leaves no credential, and the
/// record is not sent.
#[tokio::test]
async fn a_refused_grant_is_a_credentials_error_and_sends_nothing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
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
    let feed = repository(&server).with_credentials(bearer(&server));
    let refused = feed.send(&record()).await;
    assert!(
        matches!(refused, Err(FeedError::Credentials(_))),
        "{refused:?}"
    );
}
