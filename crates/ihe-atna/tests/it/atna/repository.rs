// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The TLS connection to an Audit Record Repository: a sender that ends a
//! connection after its last frame still delivers every frame it sent (RFC
//! 5425 §4.4).
//!
//! The listener holds its reads behind a small receive buffer, so the second
//! frame is still in the sender's queue when the connection ends. Both
//! certificates are minted for the run; none is committed.

use std::sync::Arc;
use std::time::Duration;

use ihe_atna::atna::repository::{Repository, Timeouts};
use rustls::pki_types::pem::PemObject as _;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use secrecy::SecretSlice;
use tokio::io::AsyncReadExt as _;
use tokio::net::TcpSocket;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;
use url::Url;

const TIMEOUTS: Timeouts = Timeouts {
    connect: Duration::from_secs(5),
    send: Duration::from_secs(5),
};

/// The octets of each syslog message: small, yet more than the listener's
/// receive window holds.
const MESSAGE: usize = 6 * 1024;

/// The receive buffer the listener asks for.
const RECEIVE_BUFFER: u32 = 4096;

/// A CA and the leaf it signed for `localhost`, valid from yesterday to a
/// week from today.
pub(super) struct Pki {
    pub(super) ca_pem: String,
    pub(super) leaf: CertificateDer<'static>,
    pub(super) key: Vec<u8>,
}

pub(super) fn pki() -> Pki {
    use rcgen::{
        BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer,
        KeyPair, KeyUsagePurpose,
    };
    let today = jiff::Timestamp::now()
        .to_zoned(jiff::tz::TimeZone::UTC)
        .date();
    let stamp = |date: jiff::civil::Date| {
        rcgen::date_time_ymd(
            i32::from(date.year()),
            u8::try_from(date.month()).expect("a month fits a u8"),
            u8::try_from(date.day()).expect("a day fits a u8"),
        )
    };
    let first = stamp(today.yesterday().expect("yesterday"));
    let last = stamp(
        today
            .checked_add(jiff::Span::new().days(7))
            .expect("a week from today"),
    );

    let mut ca = CertificateParams::new(Vec::<String>::new()).expect("CA parameters");
    ca.distinguished_name
        .push(DnType::CommonName, "Qz7-test-ca");
    ca.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    ca.not_before = first;
    ca.not_after = last;
    let ca_key = KeyPair::generate().expect("CA key");
    let ca_pem = ca.self_signed(&ca_key).expect("CA certificate").pem();

    let mut leaf = CertificateParams::new(vec!["localhost".to_owned()]).expect("leaf parameters");
    leaf.distinguished_name
        .push(DnType::CommonName, "localhost");
    leaf.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    leaf.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    leaf.not_before = first;
    leaf.not_after = last;
    let key = KeyPair::generate().expect("leaf key");
    let leaf = leaf
        .signed_by(&key, &Issuer::new(ca, ca_key))
        .expect("leaf certificate");
    Pki {
        ca_pem,
        leaf: leaf.der().clone(),
        key: key.serialize_der(),
    }
}

/// The client configuration a product's `TlsClient` builds: TLS 1.2 or
/// later, trusting `pki`'s CA.
pub(super) fn client(pki: &Pki) -> Arc<rustls::ClientConfig> {
    client_at(pki, rustls::DEFAULT_VERSIONS)
}

/// The client configuration trusting `pki`'s CA at `versions` alone.
fn client_at(
    pki: &Pki,
    versions: &[&'static rustls::SupportedProtocolVersion],
) -> Arc<rustls::ClientConfig> {
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(CertificateDer::from_pem_slice(pki.ca_pem.as_bytes()).expect("the CA reads as PEM"))
        .expect("the CA is a trust root");
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    Arc::new(
        rustls::ClientConfig::builder_with_provider(provider)
            .with_protocol_versions(versions)
            .expect("protocol versions")
            .with_root_certificates(roots)
            .with_no_client_auth(),
    )
}

/// A listener that completes one TLS handshake, says so on `ready`, reads
/// nothing until `release`, then reads to the end of the stream and returns
/// what it read.
struct Held {
    port: u16,
    ready: oneshot::Receiver<()>,
    release: oneshot::Sender<()>,
    read: JoinHandle<Vec<u8>>,
}

fn held(pki: &Pki) -> Held {
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
    let socket = TcpSocket::new_v4().expect("socket");
    socket
        .set_recv_buffer_size(RECEIVE_BUFFER)
        .expect("receive buffer");
    socket.bind(([127, 0, 0, 1], 0).into()).expect("bind");
    let listener = socket.listen(1).expect("listen");
    let port = listener.local_addr().expect("address").port();
    let (ready_tx, ready) = oneshot::channel();
    let (release, released) = oneshot::channel::<()>();
    let read = tokio::spawn(async move {
        let (tcp, _peer) = listener.accept().await.expect("accept");
        let mut tls = acceptor.accept(tcp).await.expect("handshake");
        ready_tx.send(()).expect("the test waits for the handshake");
        released.await.expect("the test releases the listener");
        let mut bytes = Vec::new();
        // NOTE: a reset ends the read as the end of the stream does; what
        // arrived before it is the outcome under test.
        while let Ok(read) = tls.read_buf(&mut bytes).await {
            if read == 0 {
                break;
            }
        }
        bytes
    });
    Held {
        port,
        ready,
        release,
        read,
    }
}

/// The RFC 5425 §4.3 frame of a syslog message of [`MESSAGE`] octets.
fn frame(fill: u8) -> Vec<u8> {
    let mut frame = format!("{MESSAGE} ").into_bytes();
    frame.resize(frame.len() + MESSAGE, fill);
    frame
}

#[tokio::test]
async fn a_connection_ended_right_after_two_frames_delivers_both() {
    let pki = pki();
    let Held {
        port,
        ready,
        release,
        read,
    } = held(&pki);
    let repository = Repository::tls(
        &Url::parse(&format!("tls://localhost:{port}")).expect("address"),
        client(&pki),
        TIMEOUTS,
    )
    .expect("repository");
    let mut connection = repository.connect().await.expect("connect");
    ready.await.expect("the listener finished the handshake");
    let (first, second) = (frame(b'a'), frame(b'b'));
    connection
        .send(&SecretSlice::from(first.clone()))
        .await
        .expect("first frame");
    connection
        .send(&SecretSlice::from(second.clone()))
        .await
        .expect("second frame");

    let closed = tokio::spawn(connection.close());
    release.send(()).expect("the listener waits");
    let bytes = read.await.expect("the listener ends");
    closed
        .await
        .expect("the close ends")
        .expect("the connection closes");

    assert!(
        bytes == [first, second].concat(),
        "the listener received {} of the {} octets sent",
        bytes.len(),
        2 * frame(b'a').len()
    );
}

/// TLS 1.2 and TLS 1.3 each meet the floor of ITI TF-2 §3.20.4.1.2.1.1, and a
/// repository reached at either version takes a frame.
#[tokio::test]
async fn a_configuration_at_tls_1_2_or_1_3_alone_is_taken_and_connects() {
    for version in [&rustls::version::TLS12, &rustls::version::TLS13] {
        let pki = pki();
        let Held {
            port,
            ready,
            release,
            read,
        } = held(&pki);
        let repository = Repository::tls(
            &Url::parse(&format!("tls://localhost:{port}")).expect("address"),
            client_at(&pki, &[version]),
            TIMEOUTS,
        )
        .expect("a configuration at TLS 1.2 or later is taken");
        let mut connection = repository.connect().await.expect("connect");
        ready.await.expect("the listener finished the handshake");
        let sent = frame(b'v');
        connection
            .send(&SecretSlice::from(sent.clone()))
            .await
            .expect("the frame");
        let closed = tokio::spawn(connection.close());
        release.send(()).expect("the listener waits");
        assert_eq!(read.await.expect("the listener ends"), sent, "{version:?}");
        closed
            .await
            .expect("the close ends")
            .expect("the connection closes");
    }
}

/// A repository that trusts nothing the listener presents fails the
/// handshake, so the caller's roots are the ones verified against.
#[tokio::test]
async fn the_callers_roots_are_the_ones_verified_against() {
    let other = pki();
    let listener = pki();
    let Held { port, .. } = held(&listener);
    let repository = Repository::tls(
        &Url::parse(&format!("tls://localhost:{port}")).expect("address"),
        client(&other),
        TIMEOUTS,
    )
    .expect("repository");
    assert!(
        repository.connect().await.is_err(),
        "a leaf from another CA is refused"
    );
}
