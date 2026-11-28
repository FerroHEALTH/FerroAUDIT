// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The post-quantum key exchange of both transports: a syslog sender and
//! the receiver over RFC 5425 TLS (ITI TF-2 §3.20.4.1), and the ATX FHIR
//! Feed's outbound client, each over the TLS configuration
//! `ferrohealth-smart`'s `ServerTls` builds from an outbound `tls` table,
//! agree on the TLS 1.3 hybrid group X25519MLKEM768 (RFC 10024), and a peer
//! that offers only a classical group still connects.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ferrohealth_config::tls::TlsClientSection;
use ferrohealth_secret::secret_url::SecretUrl;
use ferrohealth_smart::client::outbound::OutboundClient;
use ferrohealth_smart::config::endpoint::EndpointSection;
use ferrohealth_smart::config::{ServerTls, TlsBuilder as _};
use http::Method;
use ihe_atna::arr::AuditStore;
use ihe_atna::arr::syslog::{Limits, receive};
use ihe_atna::atna::forwarder::Forwarder;
use ihe_atna::atna::repository::{Repository, Timeouts};
use ihe_atna::atna::retry::Retry;
use ihe_atna::atna::spool::{Bounds, Spool};
use rustls::NamedGroup;
use rustls::pki_types::PrivateKeyDer;
use rustls::pki_types::pem::PemObject as _;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use url::Url;

use super::Memory;
use crate::atna::repository::{Pki, pki};
use crate::atna::until;

const BOUNDS: Bounds = Bounds {
    max_messages: 16,
    max_bytes: 1 << 20,
    write_timeout: Duration::from_secs(1),
};

/// The group the listener agreed on with its one connection.
type Agreed = Arc<Mutex<Option<NamedGroup>>>;

/// The `tls` table of an outbound connection that trusts `pki`'s CA, the CA
/// written to `dir`. The test CA publishes no revocation service, so it is
/// trusted with no revocation check.
fn table(pki: &Pki, dir: &tempfile::TempDir) -> TlsClientSection {
    let roots = dir.path().join("roots.pem");
    std::fs::write(&roots, &pki.ca_pem).expect("the roots are written");
    TlsClientSection {
        trust_roots_without_revocation_file: Some(roots),
        ..TlsClientSection::default()
    }
}

/// The configuration `ServerTls` builds from `table` at `key`, as a product
/// builds it from its configuration.
fn smart_tls(key: &str, table: &TlsClientSection) -> Arc<rustls::ClientConfig> {
    Arc::clone(
        ServerTls
            .build(key, table, &[])
            .expect("the table builds")
            .config(),
    )
}

/// A configuration of a peer that offers only X25519, trusting `pki`'s CA.
fn classical(pki: &Pki) -> Arc<rustls::ClientConfig> {
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(
            rustls::pki_types::CertificateDer::from_pem_slice(pki.ca_pem.as_bytes())
                .expect("the CA reads as PEM"),
        )
        .expect("the CA is a trust root");
    let provider = rustls::crypto::CryptoProvider {
        kx_groups: vec![rustls::crypto::aws_lc_rs::kx_group::X25519],
        ..rustls::crypto::aws_lc_rs::default_provider()
    };
    Arc::new(
        rustls::ClientConfig::builder_with_provider(Arc::new(provider))
            .with_safe_default_protocol_versions()
            .expect("protocol versions")
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

/// The listener's acceptor over `pki`'s leaf, on the provider rustls takes
/// by default, or offering X25519 alone when `classical`.
fn acceptor(pki: &Pki, classical: bool) -> TlsAcceptor {
    let mut provider = rustls::crypto::aws_lc_rs::default_provider();
    if classical {
        provider.kx_groups = vec![rustls::crypto::aws_lc_rs::kx_group::X25519];
    }
    let provider = Arc::new(provider);
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("protocol versions")
        .with_no_client_auth()
        .with_single_cert(
            vec![pki.leaf.clone()],
            PrivateKeyDer::Pkcs8(pki.key.clone().into()),
        )
        .expect("server configuration");
    TlsAcceptor::from(Arc::new(config))
}

/// Sends one ITI-20 message from this crate's syslog sender over `tls` to
/// this crate's receiver, and returns the group the receiver agreed on.
async fn syslog_over(pki: &Pki, tls: Arc<rustls::ClientConfig>) -> Option<NamedGroup> {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a listener");
    let port = listener.local_addr().expect("an address").port();
    let store = Arc::new(Memory::default());
    let retry = Retry::new(1, Duration::from_millis(20), Duration::from_millis(50))
        .expect("a retry policy");
    let kept: Arc<dyn AuditStore> = store.clone();
    let receiving = Forwarder::into_store(Spool::in_memory(BOUNDS), kept, retry);
    let delivering = tokio::spawn(Arc::clone(&receiving).run());
    let agreed: Agreed = Arc::default();
    let acceptor = acceptor(pki, false);
    let serving = {
        let (receiving, agreed) = (Arc::clone(&receiving), Arc::clone(&agreed));
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.expect("a connection");
            let mut tls = acceptor.accept(tcp).await.expect("the handshake");
            *agreed.lock().expect("the group") = tls
                .get_ref()
                .1
                .negotiated_key_exchange_group()
                .map(rustls::crypto::SupportedKxGroup::name);
            receive(&mut tls, &receiving, Limits::default()).await
        })
    };

    let sender = Repository::tls(
        &Url::parse(&format!("tls://localhost:{port}")).expect("a URL"),
        tls,
        Timeouts {
            connect: Duration::from_secs(5),
            send: Duration::from_secs(5),
        },
    )
    .expect("a repository");
    let sending = Forwarder::new(Spool::in_memory(BOUNDS), sender, Duration::from_millis(100));
    let running = tokio::spawn(Arc::clone(&sending).run());
    let xml = super::syslog::message()
        .to_xml()
        .expect("the message is written");
    sending
        .submit(super::syslog::frame(&xml))
        .await
        .expect("stored");
    let status = until(
        &receiving,
        Instant::now() + Duration::from_secs(10),
        |status| status.delivered == 1,
    )
    .await;
    assert_eq!(status.delivered, 1, "the message reaches the store");

    sending.stop();
    running.await.expect("the sender stops");
    receiving.stop();
    delivering.await.expect("the repository stops");
    serving.abort();
    *agreed.lock().expect("the group")
}

/// Posts one body with the FHIR Feed's outbound client, built by `ServerTls`
/// from `table`, to an HTTPS listener, offering X25519 alone when
/// `classical`, and returns the group the listener agreed on.
async fn feed_over(pki: &Pki, table: TlsClientSection, classical: bool) -> Option<NamedGroup> {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a listener");
    let port = listener.local_addr().expect("an address").port();
    let agreed: Agreed = Arc::default();
    let acceptor = acceptor(pki, classical);
    let serving = {
        let agreed = Arc::clone(&agreed);
        tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.expect("a connection");
            let mut tls = acceptor.accept(tcp).await.expect("the handshake");
            *agreed.lock().expect("the group") = tls
                .get_ref()
                .1
                .negotiated_key_exchange_group()
                .map(rustls::crypto::SupportedKxGroup::name);
            let mut request = vec![0_u8; 8192];
            let read = tls.read(&mut request).await.expect("the request");
            assert!(read > 0, "a request arrives");
            tls.write_all(
                b"HTTP/1.1 201 Created\r\ncontent-length: 0\r\nconnection: close\r\n\r\n",
            )
            .await
            .expect("the answer is written");
            tls.shutdown().await.expect("the connection closes");
        })
    };
    let endpoint = EndpointSection {
        url: SecretUrl::new(format!("https://localhost:{port}/arr/")),
        timeout: "5s".parse().expect("a duration"),
        tls: table,
        ..EndpointSection::default()
    };
    let client = OutboundClient::build(&endpoint, "audit.fhir_feed.endpoint", &ServerTls)
        .expect("the outbound client builds");
    let url = client.url().clone();
    let answer = client
        .send(&Method::POST, &url, |request| request.body(b"{}".to_vec()))
        .await
        .expect("the feed answers");
    assert_eq!(http::StatusCode::CREATED, answer.status());
    serving.await.expect("the listener answers");
    *agreed.lock().expect("the group")
}

// NOTE: RFC 10024 §3: X25519MLKEM768 is a TLS 1.3 group, which ferrohealth-server's TlsClient
// offers first, so a listener that supports it agrees on it.
#[tokio::test]
async fn the_syslog_transport_agrees_on_x25519mlkem768_and_a_classical_peer_still_connects() {
    let pki = pki();
    let dir = tempfile::tempdir().expect("a directory");
    let tls = smart_tls("audit.syslog.tls", &table(&pki, &dir));
    assert_eq!(
        Some(NamedGroup::X25519MLKEM768),
        syslog_over(&pki, tls).await
    );
    assert_eq!(
        Some(NamedGroup::X25519),
        syslog_over(&pki, classical(&pki)).await
    );
}

#[tokio::test]
async fn the_fhir_feed_agrees_on_x25519mlkem768_and_a_classical_listener_still_answers() {
    let pki = pki();
    let dir = tempfile::tempdir().expect("a directory");
    assert_eq!(
        Some(NamedGroup::X25519MLKEM768),
        feed_over(&pki, table(&pki, &dir), false).await
    );
    assert_eq!(
        Some(NamedGroup::X25519),
        feed_over(&pki, table(&pki, &dir), true).await
    );
}

// NOTE: RFC 9881 §3 and §4 define ML-DSA certificates; on Linux the platform verifier is webpki,
// which verifies them in TLS 1.3, and the macOS and Windows verifiers do not.
#[cfg(target_os = "linux")]
#[tokio::test]
async fn an_ml_dsa_certificate_authenticates_the_syslog_receiver_on_linux() {
    let pki = crate::atna::repository::pki_of(&rcgen::PKCS_ML_DSA_65);
    let dir = tempfile::tempdir().expect("a directory");
    let tls = smart_tls("audit.syslog.tls", &table(&pki, &dir));
    assert_eq!(
        Some(NamedGroup::X25519MLKEM768),
        syslog_over(&pki, tls).await
    );
}
