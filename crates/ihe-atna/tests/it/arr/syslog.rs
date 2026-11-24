// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! ITI-20 received over RFC 5425 TLS (ITI TF-2 §3.20.4.1): the messages this
//! crate's syslog forwarder sends arrive in the store as the `AuditEvent`
//! of `RESTful` ATNA Table 3.81.4.2.2.1-1, a frame stays in the repository's
//! spool while the store is down, a message the mapping cannot carry is
//! quarantined, and a connection that sends no frame is ended. Both
//! certificates are minted for the run.

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use ihe_atna::arr::AuditStore;
use ihe_atna::arr::syslog::{Limits, ReceiveError, receive, serve};
use ihe_atna::atna::codes;
use ihe_atna::atna::forwarder::Forwarder;
use ihe_atna::atna::message::{
    ActiveParticipant, AuditMessage, AuditSource, CodedValue, EventIdentification,
    ParticipantObject,
};
use ihe_atna::atna::repository::{Repository, Timeouts};
use ihe_atna::atna::retry::Retry;
use ihe_atna::atna::spool::{Bounds, Spool};
use ihe_atna::atna::syslog::Sender;
use rustls::pki_types::PrivateKeyDer;
use secrecy::{SecretSlice, SecretString};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use url::Url;

use super::Memory;
use crate::atna::repository::{client, pki};
use crate::atna::until;

const BOUNDS: Bounds = Bounds {
    max_messages: 16,
    max_bytes: 1 << 20,
    write_timeout: Duration::from_secs(1),
};

fn retry() -> Retry {
    Retry::new(1, Duration::from_millis(20), Duration::from_millis(50)).expect("a retry policy")
}

/// A DICOM audit message of a patient record read by `Qz7-user`.
fn message() -> AuditMessage {
    AuditMessage {
        event: EventIdentification {
            event_id: CodedValue::of(codes::EVENT_PATIENT_RECORD),
            action: "R",
            date_time: "2026-05-19T08:30:00Z".parse().expect("a time"),
            outcome: "0",
            type_codes: Vec::new(),
            outcome_description: None,
            purposes_of_use: Vec::new(),
        },
        participants: vec![ActiveParticipant {
            user_id: "Qz7-user".to_owned(),
            alternative_user_id: None,
            user_name: None,
            user_is_requestor: true,
            role_id_codes: vec![CodedValue::of(codes::ROLE_SOURCE_ROLE_ID)],
            access_point: None,
        }],
        source: AuditSource {
            id: "cdr.example.org".to_owned(),
            enterprise_site: None,
            type_codes: Vec::new(),
        },
        objects: vec![ParticipantObject {
            id: SecretString::from("Qz7-patient"),
            type_code: Some("1"),
            type_code_role: Some("1"),
            data_life_cycle: None,
            id_type_code: CodedValue::of(codes::OBJECT_ID_PATIENT_NUMBER),
            name: None,
            query: None,
            details: Vec::new(),
        }],
    }
}

fn frame(xml: &SecretSlice<u8>) -> SecretSlice<u8> {
    Sender::new("cdr.example.org", "ihe-atna", "4242")
        .expect("a sender")
        .frame(jiff::Timestamp::UNIX_EPOCH, xml)
}

/// The repository's receiving forwarder over an in-memory spool into
/// `store`.
fn repository(store: &Arc<Memory>) -> (Spool, Arc<Forwarder>) {
    let spool = Spool::in_memory(BOUNDS);
    let store: Arc<dyn AuditStore> = store.clone();
    let forwarder = Forwarder::into_store(spool.clone(), store, retry());
    (spool, forwarder)
}

/// ITI TF-2 §3.20.4.1: the messages this crate's syslog forwarder sends over
/// TLS are kept as the `AuditEvent` the mapping gives.
#[tokio::test]
async fn the_syslog_forwarders_messages_are_kept_as_audit_events() {
    let pki = pki();
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
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("a listener");
    let port = listener.local_addr().expect("an address").port();

    let store = Arc::new(Memory::default());
    let (_spool, receiving) = repository(&store);
    let delivering = tokio::spawn(Arc::clone(&receiving).run());
    let serving = tokio::spawn(serve(
        listener,
        TlsAcceptor::from(Arc::new(config)),
        Arc::clone(&receiving),
        Limits::default(),
    ));

    let sender = Repository::tls(
        &Url::parse(&format!("tls://localhost:{port}")).expect("a URL"),
        client(&pki),
        Timeouts {
            connect: Duration::from_secs(5),
            send: Duration::from_secs(5),
        },
    )
    .expect("a repository");
    let sending = Forwarder::new(Spool::in_memory(BOUNDS), sender, Duration::from_millis(100));
    let running = tokio::spawn(Arc::clone(&sending).run());
    let xml = message().to_xml().expect("the message is written");
    for _ in 0..2 {
        sending.submit(frame(&xml)).await.expect("stored");
    }
    let status = until(
        &receiving,
        Instant::now() + Duration::from_secs(10),
        |status| status.delivered == 2,
    )
    .await;
    assert_eq!(status.delivered, 2, "both messages reach the store");
    let kept = store.kept();
    assert_eq!(kept.len(), 2);
    let record = kept.first().expect("a record");
    assert_eq!(
        record.r#type.code.as_ref().and_then(|c| c.value.as_deref()),
        Some("110110")
    );
    assert_eq!(
        record.recorded.value.as_deref(),
        Some("2026-05-19T08:30:00Z")
    );
    assert_eq!(record.entity.len(), 1);

    sending.stop();
    running.await.expect("the sender stops");
    receiving.stop();
    delivering.await.expect("the repository stops");
    serving.abort();
}

/// ITI TF-2 §3.20.4.1.1, read for the receiving side: a frame received
/// while the store is down stays in the repository's spool and reaches the
/// store once it is back.
#[tokio::test]
async fn a_frame_received_while_the_store_is_down_is_kept_later() {
    let store = Arc::new(Memory::default());
    store.unavailable.store(true, Ordering::SeqCst);
    let (_spool, receiving) = repository(&store);
    let delivering = tokio::spawn(Arc::clone(&receiving).run());
    let xml = message().to_xml().expect("written");
    let bytes = frame(&xml);
    let mut stream: &[u8] = secrecy::ExposeSecret::expose_secret(&bytes);
    let stored = receive(&mut stream, &receiving, Limits::default())
        .await
        .expect("received");
    assert_eq!(stored, 1);
    tokio::time::sleep(Duration::from_millis(150)).await;
    assert_eq!(store.kept().len(), 0);
    assert_eq!(
        receiving.status().depth.messages,
        1,
        "the frame stays spooled"
    );
    store.unavailable.store(false, Ordering::SeqCst);
    let status = until(
        &receiving,
        Instant::now() + Duration::from_secs(5),
        |status| status.delivered == 1,
    )
    .await;
    assert_eq!(status.delivered, 1);
    assert_eq!(store.kept().len(), 1);
    receiving.stop();
    delivering.await.expect("stops");
}

/// `RESTful` ATNA Table 3.81.4.2.2.1-1: a message holding an element the
/// mapping does not carry is quarantined for an operator, never dropped.
#[tokio::test]
async fn a_message_the_mapping_cannot_carry_is_quarantined() {
    let store = Arc::new(Memory::default());
    let (spool, receiving) = repository(&store);
    let delivering = tokio::spawn(Arc::clone(&receiving).run());
    let xml = SecretSlice::from(b"<AuditMessage><Unknown/></AuditMessage>".to_vec());
    let bytes = frame(&xml);
    let mut stream: &[u8] = secrecy::ExposeSecret::expose_secret(&bytes);
    receive(&mut stream, &receiving, Limits::default())
        .await
        .expect("received");
    let deadline = Instant::now() + Duration::from_secs(5);
    while spool.depth().quarantined == 0 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(spool.depth().quarantined, 1, "kept aside");
    assert_eq!(store.kept().len(), 0);
    receiving.stop();
    delivering.await.expect("stops");
}

/// RFC 5425 §4.3: bytes that are no octet-counted frame end the connection,
/// and nothing of them is stored.
#[tokio::test]
async fn bytes_that_are_no_frame_end_the_connection() {
    let store = Arc::new(Memory::default());
    let (spool, receiving) = repository(&store);
    for bytes in [
        &b"<85>1 no length"[..],
        &b"20 <85>1 short"[..],
        &b"5 hello"[..],
    ] {
        let mut stream = bytes;
        let received = receive(&mut stream, &receiving, Limits::default()).await;
        assert!(
            matches!(received, Err(ReceiveError::Frame(_))),
            "{received:?}"
        );
    }
    assert_eq!(spool.depth().messages, 0);
}
