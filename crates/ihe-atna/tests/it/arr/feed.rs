// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The ATX: FHIR Feed received (ITI TF-2 §3.20.4.2 to §3.20.4.5): a
//! `create` in JSON and in XML, a `batch` answered entry by entry, each
//! malformed request refused with its status, and the records of this
//! crate's FHIR Feed forwarder kept, none lost while the store is down.

#[cfg(feature = "feed")]
use std::sync::Arc;
use std::sync::atomic::Ordering;
#[cfg(feature = "feed")]
use std::time::{Duration, Instant};

use fhir_types::r4::resource::Resource;
use http::StatusCode;
use ihe_atna::arr::feed::{Format, receive};
#[cfg(feature = "feed")]
use ihe_atna::atna::feed::{FeedAnswer, FeedClient, FeedRepository, FeedSendError};
#[cfg(feature = "feed")]
use ihe_atna::atna::forwarder::Forwarder;
#[cfg(feature = "feed")]
use ihe_atna::atna::spool::{Bounds, Spool};
#[cfg(feature = "feed")]
use secrecy::SecretSlice;
#[cfg(feature = "feed")]
use url::Url;

use super::Memory;

const RECORD: &str = r#"{"resourceType":"AuditEvent","type":{"system":"http://dicom.nema.org/resources/ontology/DCM","code":"110110"},"recorded":"2026-05-19T08:30:00Z","agent":[{"who":{"identifier":{"value":"Qz7-user"}},"requestor":true}],"source":{"observer":{"identifier":{"value":"cdr.example.org"}}}}"#;

const RECORD_XML: &str = r#"<AuditEvent xmlns="http://hl7.org/fhir"><type><system value="http://dicom.nema.org/resources/ontology/DCM"/><code value="110110"/></type><recorded value="2026-05-19T08:30:00Z"/><agent><who><identifier><value value="Qz7-user"/></identifier></who><requestor value="true"/></agent><source><observer><identifier><value value="cdr.example.org"/></identifier></observer></source></AuditEvent>"#;

/// ITI TF-2 §3.20.4.2.3, §3.20.4.3.2: a JSON `create` the store keeps is a
/// `201` with the record's `Location`.
#[tokio::test]
async fn a_json_create_is_kept_and_answered_201() {
    let store = Memory::default();
    let answer = receive(&store, Format::Json, RECORD.as_bytes()).await;
    assert_eq!(answer.status, StatusCode::CREATED);
    assert_eq!(answer.location.as_deref(), Some("AuditEvent/1"));
    assert_eq!(store.kept().len(), 1);
}

/// ITI TF-2 §3.20.4.2.3 and Appendix Z.6: the repository takes XML too.
#[tokio::test]
async fn an_xml_create_is_kept_as_the_same_record() {
    let store = Memory::default();
    let answer = receive(&store, Format::Xml, RECORD_XML.as_bytes()).await;
    assert_eq!(answer.status, StatusCode::CREATED, "{:?}", answer.body);
    let json = Memory::default();
    let _answer = receive(&json, Format::Json, RECORD.as_bytes()).await;
    assert_eq!(store.kept(), json.kept());
}

/// Appendix Z.6: the media types the repository reads.
#[test]
fn the_media_types_name_their_format() {
    assert_eq!(
        Format::of("application/fhir+json; charset=utf-8"),
        Some(Format::Json)
    );
    assert_eq!(Format::of("application/fhir+xml"), Some(Format::Xml));
    assert_eq!(Format::of("text/plain"), None);
}

/// FHIR R4 http.html#create: a body that is no resource is a `400`.
#[tokio::test]
async fn a_body_that_is_no_resource_is_a_bad_request() {
    let store = Memory::default();
    for (format, body) in [
        (Format::Json, &b"{not json"[..]),
        (Format::Xml, &b"<AuditEvent"[..]),
        (
            Format::Json,
            &b"{\"resourceType\":\"AuditEvent\",\"bogus\":1}"[..],
        ),
    ] {
        let answer = receive(&store, format, body).await;
        assert_eq!(answer.status, StatusCode::BAD_REQUEST, "{format:?}");
        assert!(matches!(answer.body, Some(Resource::OperationOutcome(_))));
    }
    assert_eq!(store.kept().len(), 0);
}

/// ITI TF-2 §3.20.4.2.2: only an `AuditEvent` or a Bundle is taken; any
/// other resource is a `422`, and an `AuditEvent` without the agent FHIR R4
/// requires (1..*) is no `AuditEvent`, a `400`.
#[tokio::test]
async fn another_resource_or_an_agentless_record_is_refused() {
    let store = Memory::default();
    let answer = receive(&store, Format::Json, br#"{"resourceType":"Patient"}"#).await;
    assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
    let agentless = RECORD.replace(
        r#""agent":[{"who":{"identifier":{"value":"Qz7-user"}},"requestor":true}],"#,
        "",
    );
    let answer = receive(&store, Format::Json, agentless.as_bytes()).await;
    assert_eq!(answer.status, StatusCode::BAD_REQUEST);
    assert_eq!(store.kept().len(), 0);
}

/// ITI TF-2 §3.20.4.3.2: a store that cannot keep the record now is a
/// `503`, which the sender retries; one that never will, a `422`.
#[tokio::test]
async fn a_store_that_cannot_keep_the_record_is_answered_so() {
    let store = Memory::default();
    store.unavailable.store(true, Ordering::SeqCst);
    let answer = receive(&store, Format::Json, RECORD.as_bytes()).await;
    assert_eq!(answer.status, StatusCode::SERVICE_UNAVAILABLE);
    store.unavailable.store(false, Ordering::SeqCst);
    store.refusing.store(true, Ordering::SeqCst);
    let answer = receive(&store, Format::Json, RECORD.as_bytes()).await;
    assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
}

/// ITI TF-2 §3.20.4.4.2, §3.20.4.5.2: a `batch` is answered with a
/// `batch-response` holding one status per entry; an entry that is no POST
/// of an `AuditEvent` is refused alone.
#[tokio::test]
async fn a_batch_is_answered_entry_by_entry() {
    let store = Memory::default();
    let bundle = format!(
        r#"{{"resourceType":"Bundle","type":"batch","entry":[{{"resource":{RECORD},"request":{{"method":"POST","url":"AuditEvent"}}}},{{"resource":{RECORD},"request":{{"method":"PUT","url":"AuditEvent/1"}}}}]}}"#
    );
    let answer = receive(&store, Format::Json, bundle.as_bytes()).await;
    assert_eq!(answer.status, StatusCode::OK);
    let Some(Resource::Bundle(response)) = answer.body else {
        panic!("a batch-response, got {:?}", answer.body);
    };
    assert_eq!(response.r#type.value.as_deref(), Some("batch-response"));
    let statuses: Vec<_> = response
        .entry
        .iter()
        .map(|entry| entry.response.as_ref().and_then(|r| r.status.value.clone()))
        .collect();
    assert_eq!(
        statuses,
        [
            Some("201 Created".to_owned()),
            Some("422 Unprocessable Entity".to_owned())
        ]
    );
    assert_eq!(store.kept().len(), 1);
}

/// Table 3.20.4.4.2-1: a Bundle of another type, or with no entry, is
/// refused whole.
#[tokio::test]
async fn a_bundle_that_is_no_batch_is_refused() {
    let store = Memory::default();
    for bundle in [
        format!(
            r#"{{"resourceType":"Bundle","type":"transaction","entry":[{{"resource":{RECORD},"request":{{"method":"POST","url":"AuditEvent"}}}}]}}"#
        ),
        r#"{"resourceType":"Bundle","type":"batch"}"#.to_owned(),
    ] {
        let answer = receive(&store, Format::Json, bundle.as_bytes()).await;
        assert_eq!(answer.status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    assert_eq!(store.kept().len(), 0);
}

#[cfg(feature = "feed")]
/// The HTTP client a product writes, here handing each request to the
/// repository in the same process.
struct InProcess(Arc<Memory>);

#[cfg(feature = "feed")]
#[async_trait::async_trait]
impl FeedClient for InProcess {
    fn authorizes(&self) -> bool {
        false
    }

    async fn post(
        &self,
        _url: &Url,
        content_type: &'static str,
        body: &[u8],
    ) -> Result<FeedAnswer, FeedSendError> {
        let format = Format::of(content_type).expect("the forwarder names a FHIR media type");
        let answer = receive(self.0.as_ref(), format, body).await;
        let body = match answer.body {
            Some(resource) => serde_json::to_vec(&resource).expect("the answer is written"),
            None => Vec::new(),
        };
        Ok(FeedAnswer::new(answer.status, body))
    }
}

#[cfg(feature = "feed")]
/// ITI TF-2 §3.20.4.2.3: the records this crate's FHIR Feed forwarder sends
/// are kept, and those sent while the store is down stay in the sender's
/// spool and arrive once it is back, so none is lost.
#[tokio::test]
async fn the_feed_forwarders_records_are_kept_none_lost_while_the_store_is_down() {
    let store = Arc::new(Memory::default());
    store.unavailable.store(true, Ordering::SeqCst);
    let feed = FeedRepository::new(
        &Url::parse("https://arr.example.org/fhir/").expect("a URL"),
        Arc::new(InProcess(Arc::clone(&store))),
        Duration::from_millis(50),
    )
    .expect("a repository");
    let spool = Spool::in_memory(Bounds {
        max_messages: 16,
        max_bytes: 1 << 20,
        write_timeout: Duration::from_secs(1),
    });
    let forwarder = Forwarder::fhir_feed(spool, feed);
    for _ in 0..3 {
        forwarder
            .submit(SecretSlice::from(RECORD.as_bytes().to_vec()))
            .await
            .expect("stored");
    }
    let running = tokio::spawn(Arc::clone(&forwarder).run());
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        store.kept().len(),
        0,
        "nothing kept while the store is down"
    );
    assert_eq!(
        forwarder.status().depth.messages,
        3,
        "every record stays spooled"
    );
    store.unavailable.store(false, Ordering::SeqCst);
    let status = crate::atna::until(
        &forwarder,
        Instant::now() + Duration::from_secs(5),
        |status| status.delivered == 3,
    )
    .await;
    assert_eq!(status.delivered, 3);
    assert_eq!(store.kept().len(), 3, "each record kept once");
    forwarder.stop();
    running.await.expect("the forwarder stops");
}
