// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The liveness probe a sender runs before each frame on a held connection:
//! it answers at once on a live connection, so it never caps the rate a
//! connection carries, and it still sees a connection the repository ended.
//!
//! Both certificates are minted for the run; none is committed.

use std::sync::Arc;
use std::time::{Duration, Instant};

use ihe_atna::atna::repository::{Repository, Timeouts, TlsSettings};
use rustls::pki_types::PrivateKeyDer;
use secrecy::SecretSlice;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_rustls::TlsAcceptor;
use url::Url;

use super::repository::{Pki, pki};
use crate::timing;

const TIMEOUTS: Timeouts = Timeouts {
    connect: Duration::from_secs(5),
    send: Duration::from_secs(5),
};

/// How many probe-then-send rounds the rate is measured over.
const ROUNDS: u32 = 2_000;

/// The RFC 5425 §4.3 frame of a short syslog message.
const FRAME: &[u8] = b"11 <13>1 - - - -";

/// A repository for `pki`'s leaf on a local port that accepts one
/// connection and reads it until `end` fires, then closes it with a TLS
/// `close_notify`.
async fn repository(pki: &Pki, end: oneshot::Receiver<()>) -> Repository {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("protocol versions")
        .with_no_client_auth()
        .with_single_cert(
            vec![pki.leaf.clone()],
            PrivateKeyDer::Pkcs8(pki.key.clone().into()),
        )
        .expect("server configuration");
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
    let port = listener.local_addr().expect("address").port();
    tokio::spawn(async move {
        let (tcp, _peer) = listener.accept().await.expect("accept");
        let mut tls = acceptor.accept(tcp).await.expect("handshake");
        let mut sink = vec![0_u8; 64 * 1024];
        tokio::select! {
            () = async { while matches!(tls.read(&mut sink).await, Ok(read) if read > 0) {} } => {}
            _ = end => {}
        }
        tls.shutdown().await.expect("close_notify");
    });
    Repository::tls(
        &Url::parse(&format!("tls://localhost:{port}")).expect("address"),
        &TlsSettings {
            roots: Some(pki.ca_pem.clone().into_bytes()),
            identity: None,
        },
        TIMEOUTS,
    )
    .expect("repository")
}

/// Probing before every frame leaves the connection's rate to the sends: the
/// probe takes well under a millisecond, where it used to wait for a timer
/// tick of about one (FerroHEALTH/FerroEHR#3954).
#[tokio::test]
async fn the_probe_answers_at_once_on_a_live_connection() {
    let pki = pki();
    let (_end, ended) = oneshot::channel();
    let mut connection = repository(&pki, ended)
        .await
        .connect()
        .await
        .expect("connect");
    let frame = SecretSlice::from(FRAME.to_vec());
    let mut probing = Duration::ZERO;
    let start = Instant::now();
    for _ in 0..ROUNDS {
        let probe = Instant::now();
        assert!(!connection.is_closed().await, "a live connection is open");
        probing += probe.elapsed();
        connection.send(&frame).await.expect("send");
    }
    let elapsed = start.elapsed();
    let per_probe = probing / ROUNDS;
    println!(
        "{ROUNDS} probe-then-send rounds in {elapsed:?}: {:.0} records/s, {per_probe:?} per probe",
        f64::from(ROUNDS) / elapsed.as_secs_f64()
    );
    assert!(
        per_probe < Duration::from_micros(500),
        "a probe took {per_probe:?} on average"
    );
    connection.close().await.expect("close");
}

/// A connection the repository ended is reported closed.
#[tokio::test]
async fn the_probe_sees_a_connection_the_repository_ended() {
    let pki = pki();
    let (end, ended) = oneshot::channel();
    let mut connection = repository(&pki, ended)
        .await
        .connect()
        .await
        .expect("connect");
    assert!(!connection.is_closed().await, "a live connection is open");
    end.send(()).expect("the repository waits");
    let deadline = timing::deadline(Duration::ZERO);
    while !connection.is_closed().await {
        assert!(
            Instant::now() < deadline,
            "the probe never saw the repository end the connection"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}
