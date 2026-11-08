// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! Mutual TLS to an Audit Record Repository: the client identity a product
//! puts in the rustls configuration it hands `Repository::tls` is what the
//! repository authenticates the sender by (ITI TF-2 §3.20.4.1.2.1.1, RFC
//! 5425 §5.2), and a repository that asks for one refuses a sender without
//! it. Every certificate is minted for the run; none is committed.

use std::sync::Arc;
use std::time::Duration;

use ihe_atna::atna::repository::{Repository, Timeouts};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use secrecy::SecretSlice;
use tokio::io::AsyncReadExt as _;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_rustls::TlsAcceptor;
use url::Url;

const TIMEOUTS: Timeouts = Timeouts {
    connect: Duration::from_secs(5),
    send: Duration::from_secs(5),
};

/// A CA with a server leaf for `localhost` and a client leaf.
struct Pki {
    ca: CertificateDer<'static>,
    server: (CertificateDer<'static>, Vec<u8>),
    client: (CertificateDer<'static>, Vec<u8>),
}

fn pki() -> Pki {
    use rcgen::{
        BasicConstraints, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa, Issuer,
        KeyPair, KeyUsagePurpose,
    };
    let mut ca = CertificateParams::new(Vec::<String>::new()).expect("CA parameters");
    ca.distinguished_name
        .push(DnType::CommonName, "Qz7-mutual-ca");
    ca.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    ca.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
    let ca_key = KeyPair::generate().expect("CA key");
    let ca_der = ca
        .self_signed(&ca_key)
        .expect("CA certificate")
        .der()
        .clone();
    let issuer = Issuer::new(ca, ca_key);
    let leaf = |names: Vec<String>, purpose: ExtendedKeyUsagePurpose| {
        let mut params = CertificateParams::new(names).expect("leaf parameters");
        params
            .distinguished_name
            .push(DnType::CommonName, "Qz7-mutual-leaf");
        params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
        params.extended_key_usages = vec![purpose];
        let key = KeyPair::generate().expect("leaf key");
        let cert = params.signed_by(&key, &issuer).expect("leaf certificate");
        (cert.der().clone(), key.serialize_der())
    };
    Pki {
        server: leaf(
            vec!["localhost".to_owned()],
            ExtendedKeyUsagePurpose::ServerAuth,
        ),
        client: leaf(Vec::new(), ExtendedKeyUsagePurpose::ClientAuth),
        ca: ca_der,
    }
}

fn roots(pki: &Pki) -> rustls::RootCertStore {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(pki.ca.clone()).expect("the CA is a trust root");
    roots
}

/// A repository on a local port that requires a client certificate from
/// `pki`'s CA, completes one handshake and returns what it read, or the
/// handshake's failure.
async fn repository(pki: &Pki) -> (u16, JoinHandle<Result<Vec<u8>, std::io::Error>>) {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let verifier =
        WebPkiClientVerifier::builder_with_provider(Arc::new(roots(pki)), Arc::clone(&provider))
            .build()
            .expect("client verifier");
    let config = rustls::ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("protocol versions")
        .with_client_cert_verifier(verifier)
        .with_single_cert(
            vec![pki.server.0.clone()],
            PrivateKeyDer::Pkcs8(pki.server.1.clone().into()),
        )
        .expect("server configuration");
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
    let port = listener.local_addr().expect("address").port();
    let read = tokio::spawn(async move {
        let (tcp, _peer) = listener.accept().await.expect("accept");
        let mut tls = acceptor.accept(tcp).await?;
        let mut bytes = Vec::new();
        while let Ok(read) = tls.read_buf(&mut bytes).await {
            if read == 0 {
                break;
            }
        }
        Ok(bytes)
    });
    (port, read)
}

/// The configuration a product's `TlsClient` builds, with or without the
/// client identity.
fn client(pki: &Pki, identity: bool) -> Arc<rustls::ClientConfig> {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let builder = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("protocol versions")
        .with_root_certificates(roots(pki));
    Arc::new(if identity {
        builder
            .with_client_auth_cert(
                vec![pki.client.0.clone()],
                PrivateKeyDer::Pkcs8(pki.client.1.clone().into()),
            )
            .expect("client identity")
    } else {
        builder.with_no_client_auth()
    })
}

/// The RFC 5425 §4.3 frame of a short syslog message.
fn frame() -> Vec<u8> {
    let message = b"Qz7 synthetic audit message";
    let mut frame = format!("{} ", message.len()).into_bytes();
    frame.extend_from_slice(message);
    frame
}

#[tokio::test]
async fn a_sender_with_the_client_identity_is_taken() {
    let pki = pki();
    let (port, read) = repository(&pki).await;
    let repository = Repository::tls(
        &Url::parse(&format!("tls://localhost:{port}")).expect("address"),
        client(&pki, true),
        TIMEOUTS,
    )
    .expect("repository");
    let mut connection = repository.connect().await.expect("connect");
    connection
        .send(&SecretSlice::from(frame()))
        .await
        .expect("the frame");
    connection.close().await.expect("the connection closes");
    assert_eq!(
        read.await
            .expect("the repository ends")
            .expect("the handshake"),
        frame()
    );
}

#[tokio::test]
async fn a_sender_without_a_client_identity_is_refused() {
    let pki = pki();
    let (port, read) = repository(&pki).await;
    let repository = Repository::tls(
        &Url::parse(&format!("tls://localhost:{port}")).expect("address"),
        client(&pki, false),
        TIMEOUTS,
    )
    .expect("repository");
    let delivered = async {
        let mut connection = repository.connect().await.ok()?;
        connection.send(&SecretSlice::from(frame())).await.ok()?;
        connection.close().await.ok()
    }
    .await;
    let refused = read.await.expect("the repository ends");
    assert!(
        refused.is_err(),
        "the repository refuses the handshake without a client certificate"
    );
    assert!(delivered.is_none(), "the sender sees its frame undelivered");
}
