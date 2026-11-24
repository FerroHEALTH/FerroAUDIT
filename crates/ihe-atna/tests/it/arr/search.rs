// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! ITI-81 over the repository's store (`RESTful` ATNA Rev. 3.6 §3.81.4):
//! each search parameter of §3.81.4.1.2 matched against the element it
//! names, the other FHIR R4 `AuditEvent` parameters by their FHIR type, the
//! refusals, the paging, and the "Audit Log Used" record of §3.81.5.1.
//! Every identifier is synthetic.

use std::sync::atomic::Ordering;

use fhir_types::r4::audit_event::AuditEvent;
use fhir_types::r4::resource::Resource;
use http::StatusCode;
use ihe_atna::arr::Requester;
use ihe_atna::arr::search::{Retrieval, matches, retrieve};
use ihe_atna::retrieve::Query;

use super::Memory;

/// The record every query is asked to find: a read of a patient's record by
/// a user, from a workstation, recorded by `cdr.example.org`.
fn found() -> AuditEvent {
    serde_json::from_value(serde_json::json!({
        "resourceType": "AuditEvent",
        "type": {"system": "http://dicom.nema.org/resources/ontology/DCM", "code": "110110"},
        "subtype": [{"system": "urn:ihe:event-type-code", "code": "ITI-81"}],
        "action": "R",
        "recorded": "2013-01-05T10:00:00Z",
        "outcome": "4",
        "agent": [
            {
                "who": {"reference": "Practitioner/Qz7-practitioner",
                        "identifier": {"system": "urn:oid:2.999.1", "value": "Qz7-user"}},
                "name": "Qz7 Clinician",
                "altId": "Qz7-alt",
                "role": [{"coding": [{"system": "urn:example:role", "code": "doctor"}]}],
                "requestor": true,
                "policy": ["urn:uuid:7a1d7e92-6a6e-4d2c-9a7b-0a1b2c3d4e5f"],
                "network": {"address": "192.168.0.17", "type": "2"}
            },
            {
                "who": {"reference": "Patient/Qz7-patient-ref",
                        "identifier": {"system": "urn:oid:1.2.3.4", "value": "Qz7-5678"}},
                "requestor": false
            }
        ],
        "source": {
            "site": "Qz7-site",
            "observer": {"reference": "Device/Qz7-cdr", "identifier": {"value": "1234"}}
        },
        "entity": [
            {
                "what": {"reference": "Patient/Qz7-patient-ref",
                         "identifier": {"system": "urn:oid:1.2.3.4", "value": "Qz7-5678"}},
                "type": {"system": "http://terminology.hl7.org/CodeSystem/audit-entity-type", "code": "1"},
                "role": {"system": "http://terminology.hl7.org/CodeSystem/object-role", "code": "1"}
            },
            {
                "what": {"reference": "DocumentReference/Qz7-doc",
                         "identifier": {"system": "urn:oid:1.2.3.4.5", "value": "123-203-FJ"}},
                "type": {"system": "http://terminology.hl7.org/CodeSystem/audit-entity-type", "code": "2"},
                "role": {"system": "http://terminology.hl7.org/CodeSystem/object-role", "code": "3"},
                "name": "Qz7 Discharge Letter"
            }
        ]
    }))
    .expect("an AuditEvent")
}

/// A record no query below finds but by its date.
fn other() -> AuditEvent {
    serde_json::from_value(serde_json::json!({
        "resourceType": "AuditEvent",
        "type": {"system": "http://dicom.nema.org/resources/ontology/DCM", "code": "110106"},
        "action": "C",
        "recorded": "2013-01-05T11:00:00Z",
        "outcome": "0",
        "agent": [{"who": {"identifier": {"value": "Qz7-other"}}, "requestor": true}],
        "source": {"observer": {"identifier": {"value": "9999"}}}
    }))
    .expect("an AuditEvent")
}

/// Whether the query `params`, after the day's date bounds, finds
/// [`found`] and leaves [`other`].
fn finds(params: &str) -> (bool, bool) {
    let query = Query::parse(&format!("date=ge2013-01-05&date=le2013-01-05&{params}"))
        .expect("the query reads");
    let zone = jiff::tz::TimeZone::UTC;
    (
        matches(&query, &found(), &zone),
        matches(&query, &other(), &zone),
    )
}

/// Asserts `hit` finds [`found`] alone and `miss` finds neither.
fn selects(hit: &str, miss: &str) {
    assert_eq!(finds(hit), (true, false), "{hit} finds the record alone");
    assert_eq!(finds(miss), (false, false), "{miss} finds nothing");
}

/// §3.81.4.1.2.1: `date` bounds `recorded`, at the precision it is given.
#[test]
fn date_bounds_the_recorded_time() {
    let zone = jiff::tz::TimeZone::UTC;
    let day = Query::parse("date=ge2013-01-05&date=le2013-01-05").expect("a query");
    assert!(matches(&day, &found(), &zone));
    let before = Query::parse("date=lt2013-01-05T10:00:00Z").expect("a query");
    assert!(!matches(&before, &found(), &zone));
    let next_day = Query::parse("date=2013-01-06").expect("a query");
    assert!(!matches(&next_day, &found(), &zone));
}

/// §3.81.4.1.2.2: `address` matches `agent.network.address` as a substring.
#[test]
fn address_matches_a_substring_of_the_network_address() {
    selects("address=168.0.1", "address=10.0.0");
}

/// §3.81.4.1.2.2: `agent.identifier` matches `agent.who.identifier`.
#[test]
fn agent_identifier_matches_the_agents_identifier() {
    selects(
        "agent.identifier=urn:oid:2.999.1|Qz7-user",
        "agent.identifier=urn:oid:2.999.2|Qz7-user",
    );
    assert_eq!(
        finds("agent.identifier=Qz7-user"),
        (true, false),
        "the code alone"
    );
}

/// §3.81.4.1.2.2: `patient.identifier` matches the identifier of an agent
/// or an entity that is the patient, and of no other.
#[test]
fn patient_identifier_matches_the_patients_identifier() {
    selects(
        "patient.identifier=urn:oid:1.2.3.4|Qz7-5678",
        "patient.identifier=urn:oid:2.999.1|Qz7-user",
    );
}

/// §3.81.4.1.2.2: `entity.identifier` matches `entity.what.identifier`,
/// with the system or the no-system form.
#[test]
fn entity_identifier_matches_the_entitys_identifier() {
    selects(
        "entity.identifier=urn:oid:1.2.3.4.5|123-203-FJ",
        "entity.identifier=|123-203-FJ",
    );
}

/// §3.81.4.1.2.2: `entity-type` matches `entity.type`.
#[test]
fn entity_type_matches_the_entitys_type() {
    selects(
        "entity-type=http://terminology.hl7.org/CodeSystem/audit-entity-type|2",
        "entity-type=http://terminology.hl7.org/CodeSystem/audit-entity-type|4",
    );
}

/// §3.81.4.1.2.2: `entity-role` matches `entity.role`.
#[test]
fn entity_role_matches_the_entitys_role() {
    selects(
        "entity-role=http://terminology.hl7.org/CodeSystem/object-role|3",
        "entity-role=http://terminology.hl7.org/CodeSystem/object-role|13",
    );
}

/// §3.81.4.1.2.2: `source.identifier` matches `source.observer.identifier`.
#[test]
fn source_identifier_matches_the_observers_identifier() {
    selects("source.identifier=1234", "source.identifier=4321");
}

/// §3.81.4.1.2.2: `type` matches `AuditEvent.type`.
#[test]
fn type_matches_the_event_type() {
    selects(
        "type=http://dicom.nema.org/resources/ontology/DCM|110110",
        "type=http://dicom.nema.org/resources/ontology/DCM|110112",
    );
}

/// §3.81.4.1.2.2: `subtype` matches `AuditEvent.subtype`.
#[test]
fn subtype_matches_the_event_subtype() {
    selects(
        "subtype=urn:ihe:event-type-code|ITI-81",
        "subtype=urn:ihe:event-type-code|ITI-43",
    );
}

/// §3.81.4.1.2.2: `outcome` matches `AuditEvent.outcome`, any of its
/// values.
#[test]
fn outcome_matches_any_of_its_values() {
    selects(
        "outcome=http://hl7.org/fhir/audit-event-outcome|4,8,12",
        "outcome=http://hl7.org/fhir/audit-event-outcome|8,12",
    );
}

/// §3.81.4.1.2.2: parameters combine with AND.
#[test]
fn parameters_combine_with_and() {
    assert_eq!(finds("outcome=4&type=110110"), (true, false));
    assert_eq!(finds("outcome=4&type=110106"), (false, false));
}

/// §3.81.4.1.3: the other FHIR R4 `AuditEvent` parameters follow the
/// matching rules of their type.
#[test]
fn the_fhir_parameters_match_by_their_type() {
    selects("action=R", "action=U");
    selects("agent-name=qz7 clin", "agent-name=Clinician");
    selects("entity-name=Qz7 Discharge", "entity-name=Letter");
    selects(
        "agent-role=urn:example:role|doctor",
        "agent-role=urn:example:role|nurse",
    );
    selects("altid=Qz7-alt", "altid=Qz7-other-alt");
    selects("site=Qz7-site", "site=Qz7-other-site");
    selects(
        "policy=urn:uuid:7a1d7e92-6a6e-4d2c-9a7b-0a1b2c3d4e5f",
        "policy=urn:uuid:00000000-0000-4000-8000-000000000000",
    );
    selects(
        "agent=Practitioner/Qz7-practitioner",
        "agent=Practitioner/Qz7-nobody",
    );
    selects("entity=DocumentReference/Qz7-doc", "entity=Qz7-nothing");
    selects("patient=Qz7-patient-ref", "patient=Qz7-doc");
    selects("source=Device/Qz7-cdr", "source=Device/Qz7-other");
}

/// The repository's view of one retrieval.
fn retrieval<'a>(query: &'a str, requester: &'a Requester) -> Retrieval<'a> {
    Retrieval {
        query,
        endpoint: "https://arr.example.org/fhir/AuditEvent",
        requester,
        observer: "arr.example.org",
    }
}

fn requester() -> Requester {
    Requester {
        user: Some("Qz7-auditor".to_owned()),
        address: Some("192.0.2.9".to_owned()),
    }
}

/// The store, holding [`found`] and [`other`].
fn store() -> Memory {
    let store = Memory::default();
    store
        .kept
        .lock()
        .expect("the store")
        .extend([found(), other()]);
    store
}

/// Xt-EHR D8.2 §5.1.11 log-Review: the repository answers ITI-81 with the
/// matching records in a `searchset` Bundle, and keeps the "Audit Log Used"
/// record of the retrieval (§3.81.5.1).
#[tokio::test]
async fn d8_2_log_review_retrieves_the_matching_records_and_records_the_retrieval() {
    let store = store();
    let requester = requester();
    let answer = retrieve(
        &store,
        retrieval("date=ge2013-01-05&date=le2013-01-05&outcome=4", &requester),
    )
    .await;
    assert_eq!(answer.status, StatusCode::OK);
    let Some(Resource::Bundle(bundle)) = answer.body else {
        panic!("a Bundle, got {:?}", answer.body);
    };
    assert_eq!(bundle.r#type.value.as_deref(), Some("searchset"));
    assert_eq!(bundle.total.and_then(|total| total.value), Some(1));
    assert_eq!(bundle.entry.len(), 1);
    let kept = store.kept();
    let used = kept.last().expect("the Audit Log Used record");
    assert_eq!(
        used.r#type.code.as_ref().and_then(|c| c.value.as_deref()),
        Some("110101")
    );
    assert_eq!(
        used.action.as_ref().and_then(|a| a.value.as_deref()),
        Some("R")
    );
    assert_eq!(
        used.subtype
            .first()
            .and_then(|s| s.code.as_ref())
            .and_then(|c| c.value.as_deref()),
        Some("ITI-81")
    );
    assert!(
        used.agent.iter().any(|agent| agent
            .who
            .as_ref()
            .and_then(|who| who.identifier.as_ref())
            .and_then(|id| id.value.as_ref())
            .and_then(|v| v.value.as_deref())
            == Some("Qz7-auditor")),
        "the human requestor"
    );
    assert_eq!(used.entity.len(), 1, "one entity per record returned");
}

/// §3.81.4.2.2: a query with no `date` is refused with a `400` and a
/// human-readable description.
#[tokio::test]
async fn a_query_without_a_date_is_a_bad_request() {
    let store = store();
    let requester = requester();
    let answer = retrieve(&store, retrieval("outcome=4", &requester)).await;
    assert_eq!(answer.status, StatusCode::BAD_REQUEST);
    let Some(Resource::OperationOutcome(outcome)) = answer.body else {
        panic!("an OperationOutcome, got {:?}", answer.body);
    };
    assert!(outcome.issue[0].diagnostics.is_some(), "a description");
    assert_eq!(
        store.kept().len(),
        2,
        "a refused query reads and records nothing"
    );
}

/// §3.81.4.1.3: a parameter the repository does not support, such as
/// `_sort`, is ignored.
#[tokio::test]
async fn an_unsupported_parameter_is_ignored() {
    let store = store();
    let requester = requester();
    let answer = retrieve(
        &store,
        retrieval(
            "date=ge2013-01-05&date=le2013-01-05&_sort=date&outcome=4",
            &requester,
        ),
    )
    .await;
    assert_eq!(answer.status, StatusCode::OK);
    let Some(Resource::Bundle(bundle)) = answer.body else {
        panic!("a Bundle");
    };
    assert_eq!(bundle.total.and_then(|total| total.value), Some(1));
}

/// §3.81.4.2.2: no match is a `200` with an empty Bundle.
#[tokio::test]
async fn no_match_is_an_empty_bundle() {
    let store = store();
    let requester = requester();
    let answer = retrieve(&store, retrieval("date=2020-01-01", &requester)).await;
    assert_eq!(answer.status, StatusCode::OK);
    let Some(Resource::Bundle(bundle)) = answer.body else {
        panic!("a Bundle");
    };
    assert_eq!(bundle.total.and_then(|total| total.value), Some(0));
    assert_eq!(bundle.entry.len(), 0);
}

/// §3.81.4.2.2: a large result is returned in pages, with a `next` link.
#[tokio::test]
async fn a_page_links_the_next() {
    let store = store();
    let requester = requester();
    let answer = retrieve(&store, retrieval("date=2013-01-05&_count=1", &requester)).await;
    let Some(Resource::Bundle(bundle)) = answer.body else {
        panic!("a Bundle");
    };
    assert_eq!(bundle.total.and_then(|total| total.value), Some(2));
    let next = bundle
        .link
        .iter()
        .find(|link| link.relation.value.as_deref() == Some("next"))
        .and_then(|link| link.url.value.clone())
        .expect("a next link");
    assert_eq!(
        next,
        "https://arr.example.org/fhir/AuditEvent?date=2013-01-05&_count=1&_offset=1"
    );
}

/// ITI TF-2 §3.20.4.3.2: a store that cannot be searched is a `503`, and a
/// retrieval whose record cannot be kept returns no records.
#[tokio::test]
async fn an_unavailable_store_returns_no_records() {
    let store = store();
    store.unavailable.store(true, Ordering::SeqCst);
    let requester = requester();
    let answer = retrieve(&store, retrieval("date=2013-01-05", &requester)).await;
    assert_eq!(answer.status, StatusCode::SERVICE_UNAVAILABLE);
    assert!(matches!(answer.body, Some(Resource::OperationOutcome(_))));

    let store = Memory::default();
    store.kept.lock().expect("the store").push(found());
    store.refusing.store(true, Ordering::SeqCst);
    let answer = retrieve(&store, retrieval("date=2013-01-05", &requester)).await;
    assert_eq!(
        answer.status,
        StatusCode::SERVICE_UNAVAILABLE,
        "unrecorded, unanswered"
    );
    assert!(matches!(answer.body, Some(Resource::OperationOutcome(_))));
}
