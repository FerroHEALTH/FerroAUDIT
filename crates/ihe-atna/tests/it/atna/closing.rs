// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The forwarder ends every syslog connection it leaves with a TLS
//! `close_notify` (RFC 5425 §4.4): the connection open when it is stopped,
//! after which the frames it wrote last still arrive, and a connection the
//! repository ended, before it reconnects.
//!
//! The listener tells a `close_notify` from an abrupt end: tokio-rustls reads
//! the first as the end of the stream and the second as an error. Both
//! certificates are minted for the run; none is committed.

use std::sync::Arc;
use std::time::Duration;

use ihe_atna::atna::forwarder::Forwarder;
use ihe_atna::atna::repository::{Repository, Timeouts, TlsSettings};
use ihe_atna::atna::spool::{Bounds, Spool};
use rustls::pki_types::PrivateKeyDer;
use secrecy::SecretSlice;
use tokio::io::{AsyncRead, AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpSocket;
use tokio::sync::{mpsc, oneshot};
use tokio_rustls::TlsAcceptor;
use url::Url;

use super::repository::{Pki, pki};
use super::until;
use crate::timing;

const TIMEOUTS: Timeouts = Timeouts {
    connect: Duration::from_secs(5),
    send: Duration::from_secs(5),
};

const BOUNDS: Bounds = Bounds {
    max_messages: 16,
    max_bytes: 64 << 20,
    write_timeout: Duration::from_secs(10),
};

const RETRY_MAX: Duration = Duration::from_millis(400);

/// The octets of each large syslog message: more than the listener's receive
/// window holds, so a frame is still queued when the connection ends.
const MESSAGE: usize = 6 * 1024;

/// The receive buffer the listener asks for.
const RECEIVE_BUFFER: u32 = 4096;

/// How a sender ended a connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ending {
    /// With a TLS `close_notify`.
    CloseNotify,
    /// Without one: dropped, reset or failed.
    Abrupt,
}

/// The RFC 5425 §4.3 frame of a syslog message of `octets` octets of `fill`.
fn frame(octets: usize, fill: u8) -> Vec<u8> {
    let mut frame = format!("{octets} ").into_bytes();
    frame.resize(frame.len() + octets, fill);
    frame
}

/// A TLS acceptor for `pki`'s leaf.
fn acceptor(pki: &Pki) -> TlsAcceptor {
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
    TlsAcceptor::from(Arc::new(config))
}

/// The TLS repository at `port`, trusting `pki`'s CA.
fn repository(pki: &Pki, port: u16) -> Repository {
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

/// Reads `stream` to its end, and returns what it read and how it ended.
async fn read_to_end<S: AsyncRead + Unpin>(
    stream: &mut S,
    mut bytes: Vec<u8>,
) -> (Vec<u8>, Ending) {
    loop {
        match stream.read_buf(&mut bytes).await {
            Ok(0) => return (bytes, Ending::CloseNotify),
            Ok(_) => {}
            Err(_) => return (bytes, Ending::Abrupt),
        }
    }
}

/// Reads from `stream` until `bytes` holds `octets` octets or more.
async fn read_at_least<S: AsyncRead + Unpin>(stream: &mut S, octets: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    while bytes.len() < octets {
        let read = stream.read_buf(&mut bytes).await.expect("a read");
        assert_ne!(0, read, "the stream ended early");
    }
    bytes
}

#[tokio::test]
async fn a_stopped_forwarder_closes_its_connection_and_the_frames_it_wrote_last_arrive() {
    let pki = pki();
    let acceptor = acceptor(&pki);
    let socket = TcpSocket::new_v4().expect("socket");
    socket
        .set_recv_buffer_size(RECEIVE_BUFFER)
        .expect("receive buffer");
    socket.bind(([127, 0, 0, 1], 0).into()).expect("bind");
    let listener = socket.listen(1).expect("listen");
    let port = listener.local_addr().expect("address").port();
    let (release, released) = oneshot::channel::<()>();
    let read = tokio::spawn(async move {
        let (tcp, _peer) = listener.accept().await.expect("accept");
        let mut tls = acceptor.accept(tcp).await.expect("handshake");
        released.await.expect("the test releases the listener");
        read_to_end(&mut tls, Vec::new()).await
    });

    let forwarder = Forwarder::new(Spool::in_memory(BOUNDS), repository(&pki, port), RETRY_MAX);
    let (first, second) = (frame(MESSAGE, b'a'), frame(MESSAGE, b'b'));
    for sent in [&first, &second] {
        forwarder
            .submit(SecretSlice::from(sent.clone()))
            .await
            .expect("stored");
    }
    let running = tokio::spawn(Arc::clone(&forwarder).run());
    let status = until(&forwarder, timing::deadline(TIMEOUTS.send), |status| {
        status.delivered == 2
    })
    .await;
    assert_eq!(2, status.delivered, "both frames are written");

    forwarder.stop();
    release.send(()).expect("the listener waits");
    let (bytes, ending) = read.await.expect("the listener ends");
    tokio::time::timeout(timing::SLACK.saturating_add(TIMEOUTS.send), running)
        .await
        .expect("the stopped forwarder returns")
        .expect("the delivery task ends without a panic");

    assert!(
        bytes == [first, second].concat(),
        "the listener received {} of the {} octets sent",
        bytes.len(),
        2 * frame(MESSAGE, b'a').len()
    );
    assert_eq!(
        Ending::CloseNotify,
        ending,
        "the forwarder ended its connection without a close_notify"
    );
}

#[tokio::test]
async fn a_connection_the_repository_ended_is_closed_with_close_notify_before_the_reconnect() {
    let pki = pki();
    let acceptor = acceptor(&pki);
    let socket = TcpSocket::new_v4().expect("socket");
    socket.bind(([127, 0, 0, 1], 0).into()).expect("bind");
    let listener = socket.listen(2).expect("listen");
    let port = listener.local_addr().expect("address").port();
    let (first, second) = (frame(16, b'a'), frame(16, b'b'));
    let octets = first.len();
    let (shut_tx, mut shut) = mpsc::unbounded_channel::<()>();
    let (endings_tx, mut endings) = mpsc::unbounded_channel::<(Vec<u8>, Ending)>();
    tokio::spawn(async move {
        let (tcp, _peer) = listener.accept().await.expect("the first accept");
        let mut tls = acceptor
            .clone()
            .accept(tcp)
            .await
            .expect("the first handshake");
        let read = read_at_least(&mut tls, octets).await;
        tls.shutdown().await.expect("the repository ends its side");
        shut_tx.send(()).expect("the test waits for the end");
        let (rest, ending) = read_to_end(&mut tls, Vec::new()).await;
        endings_tx
            .send(([read, rest].concat(), ending))
            .expect("the test reads the first ending");
        let (tcp, _peer) = listener.accept().await.expect("the second accept");
        let mut tls = acceptor.accept(tcp).await.expect("the second handshake");
        let read = read_at_least(&mut tls, octets).await;
        endings_tx
            .send((read, Ending::CloseNotify))
            .expect("the test reads the second connection");
    });

    let forwarder = Forwarder::new(Spool::in_memory(BOUNDS), repository(&pki, port), RETRY_MAX);
    let running = tokio::spawn(Arc::clone(&forwarder).run());
    forwarder
        .submit(SecretSlice::from(first.clone()))
        .await
        .expect("stored");
    tokio::time::timeout(timing::SLACK.saturating_add(TIMEOUTS.send), shut.recv())
        .await
        .expect("the repository ends the first connection")
        .expect("the listener says so");
    // NOTE: the repository's close_notify must reach the sender before the
    // next frame, so its probe finds the connection ended.
    tokio::time::sleep(Duration::from_millis(300)).await;
    forwarder
        .submit(SecretSlice::from(second.clone()))
        .await
        .expect("stored");

    let wait = timing::SLACK.saturating_add(TIMEOUTS.send);
    let (on_first, ending) = tokio::time::timeout(wait, endings.recv())
        .await
        .expect("the first connection ends")
        .expect("the listener reports it");
    let (on_second, _) = tokio::time::timeout(wait, endings.recv())
        .await
        .expect("the second connection carries a frame")
        .expect("the listener reports it");
    forwarder.stop();
    tokio::time::timeout(wait, running)
        .await
        .expect("the stopped forwarder returns")
        .expect("the delivery task ends without a panic");

    assert_eq!(
        first, on_first,
        "the first connection carries the first frame alone"
    );
    assert_eq!(
        Ending::CloseNotify,
        ending,
        "the forwarder dropped the ended connection without a close_notify"
    );
    assert_eq!(
        second, on_second,
        "the second frame goes over a new connection"
    );
}

#[tokio::test]
async fn a_forwarder_stopped_while_it_backs_off_returns_with_its_messages_stored() {
    let url = Url::parse("tcp://127.0.0.1:0").expect("a URL");
    let repository = Repository::unencrypted_for_development(&url, TIMEOUTS).expect("a repository");
    let forwarder = Forwarder::new(
        Spool::in_memory(BOUNDS),
        repository,
        Duration::from_secs(60),
    );
    forwarder
        .submit(SecretSlice::from(frame(4, b'k')))
        .await
        .expect("stored");
    let running = tokio::spawn(Arc::clone(&forwarder).run());
    let status = until(&forwarder, timing::deadline(TIMEOUTS.connect), |status| {
        status.retries >= 1
    })
    .await;
    assert!(status.retries >= 1, "the first attempt fails");
    forwarder.stop();
    tokio::time::timeout(timing::SLACK, running)
        .await
        .expect("a stop ends the backoff")
        .expect("the delivery task ends without a panic");
    assert_eq!(
        1,
        forwarder.status().depth.messages,
        "the message stays stored"
    );
}
