// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The reason a user gave for an access, read back over ITI-81 (`RESTful`
//! ATNA Rev. 3.6 §3.81.4.2.2): kept as the `text` of the purpose from a
//! record received over the FHIR Feed, and from a DICOM message's
//! `PurposeOfUse` `originalText` by Table 3.81.4.2.2.1-1. Every identifier
//! is synthetic.

use fhir_types::r4::audit_event::AuditEvent;
use fhir_types::r4::codeable_concept::CodeableConcept;
use fhir_types::r4::resource::Resource;
use http::StatusCode;
use ihe_atna::arr::Requester;
use ihe_atna::arr::dicom::audit_event;
use ihe_atna::arr::feed::{Format, receive};
use ihe_atna::arr::search::{Retrieval, retrieve};
use ihe_atna::atna::codes;
use ihe_atna::atna::message::{
    ActiveParticipant, AuditMessage, AuditSource, CodedValue, EventIdentification,
};
use ihe_atna::balp::{
    DESTINATION_ROLE, Direction, EventKind, Exchange, NetworkAddress, Outcome, Peer, READ, REST,
    SOURCE_ROLE,
};
use ihe_atna::user::{OnBehalfOf, PurposeOfUse, User};
use secrecy::ExposeSecret as _;
use url::Url;

use super::Memory;
use crate::balp::observer;

/// The HL7 v3 `ActReason` code system.
const ACT_REASON: &str = "http://terminology.hl7.org/CodeSystem/v3-ActReason";

/// The reason the synthetic user gave.
const REASON: &str = "Qz7 patient arrived unconscious, outside my caseload";

/// The records of a retrieval of `date` from `store`.
async fn retrieved(store: &Memory, date: &str) -> Vec<AuditEvent> {
    let requester = Requester {
        user: Some("Qz7-auditor".to_owned()),
        address: None,
    };
    let query = format!("date={date}");
    let answer = retrieve(
        store,
        Retrieval {
            query: &query,
            endpoint: "https://arr.example.org/fhir/AuditEvent",
            requester: &requester,
            observer: "arr.example.org",
        },
    )
    .await;
    assert_eq!(answer.status, StatusCode::OK);
    let Some(Resource::Bundle(bundle)) = answer.body else {
        panic!("a Bundle, got {:?}", answer.body);
    };
    bundle
        .entry
        .into_iter()
        .filter_map(|entry| match entry.resource {
            Some(Resource::AuditEvent(event)) => Some(*event),
            _ => None,
        })
        .collect()
}

/// The `(code, text)` of each concept in `concepts`.
fn stated(concepts: &[CodeableConcept]) -> Vec<(Option<String>, Option<String>)> {
    concepts
        .iter()
        .map(|concept| {
            (
                concept
                    .coding
                    .first()
                    .and_then(|coding| coding.code.as_ref())
                    .and_then(|code| code.value.clone()),
                concept.text.as_ref().and_then(|text| text.value.clone()),
            )
        })
        .collect()
}

fn want() -> Vec<(Option<String>, Option<String>)> {
    vec![(Some("BTG".to_owned()), Some(REASON.to_owned()))]
}

/// A BALP record of a read made for a user who stated `REASON` for
/// breaking the glass, received over the FHIR Feed, comes back over ITI-81
/// with the reason at the `agent:user` and at `purposeOfEvent`.
#[tokio::test]
async fn a_reason_received_over_the_feed_is_retrieved() {
    let exchange = Exchange {
        kind: EventKind {
            profile: "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Read",
            event_type: REST,
            subtypes: vec![READ],
            action: "R",
            client: DESTINATION_ROLE,
            server: SOURCE_ROLE,
        },
        recorded: jiff::Timestamp::from_second(1_767_225_600).expect("a timestamp"),
        outcome: Outcome::Success,
        outcome_description: None,
        direction: Direction::Received {
            client: Peer::server(&Url::parse("https://client.example.org/").expect("a URL")),
            endpoint: NetworkAddress::Uri(
                Url::parse("https://gateway.example.org/v1/ehr").expect("a URL"),
            ),
        },
        on_behalf: OnBehalfOf::User(
            User::new(
                "https://issuer.example.org".to_owned(),
                "Qz7-subject-61".to_owned(),
                "Qz7-client-62".to_owned(),
            )
            .with_purposes(vec![PurposeOfUse {
                system: Some(ACT_REASON.to_owned()),
                code: "BTG".to_owned(),
                text: Some(REASON.to_owned()),
            }]),
        ),
        entities: Vec::new(),
        security: Vec::new(),
    };
    let record = exchange
        .audit_event(&observer())
        .expect("a record")
        .into_bytes();
    let store = Memory::default();
    let answer = receive(&store, Format::Json, record.expose_secret()).await;
    assert_eq!(answer.status, StatusCode::CREATED, "{:?}", answer.body);

    let found = retrieved(&store, "2026-01-01").await;
    let [event] = found.as_slice() else {
        panic!("one record, got {}", found.len());
    };
    assert_eq!(stated(&event.purpose_of_event), want(), "purposeOfEvent");
    let users: Vec<_> = event
        .agent
        .iter()
        .filter(|agent| !agent.purpose_of_use.is_empty())
        .collect();
    let [user] = users.as_slice() else {
        panic!("one agent with purposes, got {}", users.len());
    };
    assert_eq!(stated(&user.purpose_of_use), want(), "agent:user");
}

/// A DICOM message whose `PurposeOfUse` carries `REASON` as its
/// `originalText` comes back over ITI-81 with the reason as the text of
/// its `purposeOfEvent`.
#[tokio::test]
async fn a_reason_received_as_a_dicom_message_is_retrieved() {
    let message = AuditMessage {
        event: EventIdentification {
            event_id: CodedValue::of(codes::EVENT_PATIENT_RECORD),
            action: "R",
            date_time: "2026-05-19T08:30:00Z".parse().expect("a time"),
            outcome: "0",
            type_codes: Vec::new(),
            outcome_description: None,
            purposes_of_use: vec![CodedValue::new("BTG", ACT_REASON, REASON)],
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
        objects: Vec::new(),
    };
    let xml = message.to_xml().expect("the message is written");
    let store = Memory::default();
    store
        .kept
        .lock()
        .expect("the store")
        .push(audit_event(xml.expose_secret()).expect("the mapping carries the message"));

    let found = retrieved(&store, "2026-05-19").await;
    let [event] = found.as_slice() else {
        panic!("one record, got {}", found.len());
    };
    assert_eq!(stated(&event.purpose_of_event), want());
}
