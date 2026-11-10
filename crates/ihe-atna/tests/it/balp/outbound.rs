// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The FHIR Feed repository the tests reach: `ferrohealth-smart`'s
//! `OutboundClient`, built from an endpoint table at a `wiremock` server
//! as a product builds it from `[audit.fhir_feed.endpoint]`, over clear text.

use std::sync::Arc;

use ferrohealth_config::tls::TlsClientSection;
use ferrohealth_secret::secret_url::SecretUrl;
use ferrohealth_smart::client::PresentedTls;
use ferrohealth_smart::client::outbound::OutboundClient;
use ferrohealth_smart::config::TlsBuilder;
use ferrohealth_smart::config::credentials::{CredentialsSection, OauthSection};
use ferrohealth_smart::config::endpoint::{EndpointSection, RetrySection};
use ihe_atna::atna::feed::FeedRepository;
use secrecy::SecretString;
use wiremock::MockServer;

/// The test's reader of TLS material: a client that trusts no root, since
/// every repository a test reaches speaks clear text.
#[derive(Debug)]
struct NoRoots;

impl TlsBuilder for NoRoots {
    fn build(
        &self,
        _key: &str,
        _section: &TlsClientSection,
    ) -> Result<PresentedTls, Box<dyn std::error::Error + Send + Sync>> {
        let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
        let config = rustls::ClientConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()?
            .with_root_certificates(rustls::RootCertStore::empty())
            .with_no_client_auth();
        Ok(PresentedTls::new(Arc::new(config), None))
    }
}

/// The `retry` table of `max_attempts` tries a round, waiting from
/// `initial_backoff` up to `max_backoff`, written as the configuration
/// writes them (`"400ms"`).
pub(crate) fn retry(max_attempts: u32, initial_backoff: &str, max_backoff: &str) -> RetrySection {
    RetrySection {
        max_attempts,
        initial_backoff: initial_backoff.parse().expect("a duration"),
        max_backoff: max_backoff.parse().expect("a duration"),
    }
}

/// The endpoint table of the repository at `server`'s `/arr/` base: one try
/// a round, a three-second call, and 400 ms between rounds, with no
/// credential.
pub(crate) fn table(server: &MockServer) -> EndpointSection {
    EndpointSection {
        url: SecretUrl::new(format!("{}/arr/", server.uri())),
        timeout: "3s".parse().expect("a duration"),
        retry: retry(1, "400ms", "400ms"),
        ..EndpointSection::default()
    }
}

/// A synthetic Basic password minted for one run, so no credential is
/// written in the source.
pub(crate) fn synthetic_password() -> String {
    use std::hash::BuildHasher as _;
    let minted = std::collections::hash_map::RandomState::new().hash_one(std::process::id());
    format!("Qz7-{minted:016x}")
}

/// The `credentials` table of HTTP Basic.
pub(crate) fn basic(user: &str, password: &str) -> CredentialsSection {
    CredentialsSection {
        user: user.to_owned(),
        password: Some(SecretString::from(password)),
        ..CredentialsSection::default()
    }
}

/// The `credentials` table of a client credentials grant at `server`'s token
/// endpoint.
pub(crate) fn grant(server: &MockServer) -> CredentialsSection {
    CredentialsSection {
        oauth: Some(OauthSection {
            token_endpoint: SecretUrl::new(format!("{}/token", server.uri())),
            client_id: "Qz7-audit-client".to_owned(),
            client_secret: Some(SecretString::from("Qz7-audit-secret")),
            ..OauthSection::default()
        }),
        ..CredentialsSection::default()
    }
}

/// The outbound client the endpoint table `table` builds.
pub(crate) fn client(table: &EndpointSection) -> OutboundClient {
    OutboundClient::build(table, "audit.fhir_feed.endpoint", &NoRoots)
        .expect("the outbound client builds")
}

/// The repository at `server`, its calls carrying what `credentials` names.
pub(crate) fn repository(server: &MockServer, credentials: CredentialsSection) -> FeedRepository {
    let table = EndpointSection {
        credentials,
        ..table(server)
    };
    FeedRepository::cleartext(client(&table)).expect("a repository")
}
