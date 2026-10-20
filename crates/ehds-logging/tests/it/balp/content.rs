// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! What a record writes of an accessor with no OAuth identity, its roles
//! and its token, an origin of stored data, a patient namespace that is no
//! URI, a request with no id, the operation as a subtype, the outcome, the
//! retention, and the evidence of no category and its counts.
#![expect(
    clippy::disallowed_types,
    reason = "the test seam: the written record is read as a JSON value"
)]

use ehds_logging::balp::detail;
use ehds_logging::classify::{Basis, Evidence, ResourceKind, RootObject};
use ehds_logging::record::{Action, Coded, Origin, Outcome, PatientIdentifier};
use ehds_logging::retention::{Bounds, Period, RetentionPolicy};
use secrecy::SecretString;
use serde_json::Value;
use std::collections::BTreeMap;

use super::super::support::{LAB_REPORT, map};
use super::{CLIENT, SUBJECT, details, entity, record, written};

fn requestor(record: &Value) -> &Value {
    record["agent"]
        .as_array()
        .expect("agents")
        .iter()
        .find(|agent| agent["requestor"] == true)
        .expect("the agent:user")
}

fn agents_typed<'a>(record: &'a Value, code: &str) -> Vec<&'a Value> {
    record["agent"]
        .as_array()
        .expect("agents")
        .iter()
        .filter(|agent| agent["type"]["coding"][0]["code"] == code)
        .collect()
}

fn categories(record: &Value) -> &Value {
    entity(record, "ehds-categories")
        .first()
        .copied()
        .expect("the categories entity")
}

/// A user authenticated with no token: no issuer, no client, no acting mode,
/// and nothing invented in their place.
#[test]
fn an_accessor_with_no_oauth_identity_writes_nothing_in_its_place() {
    let mut access = record(Action::Read, Outcome::Success);
    access.accessor.issuer = None;
    access.accessor.client_id = None;
    access.accessor.acting = None;
    let written = written(&access);
    let user = requestor(&written);
    assert_eq!(user["who"]["identifier"]["value"], SUBJECT);
    assert!(
        user["who"]["identifier"].get("system").is_none(),
        "{written}"
    );
    assert!(user.get("role").is_none(), "no acting mode: {written}");
    assert!(
        agents_typed(&written, "110150").is_empty(),
        "no client: {written}"
    );
    assert!(!written.to_string().contains(CLIENT), "{written}");
}

/// BALP 1.1.4 §3:5.7.5.4 writes the IUA `subject_role` at
/// `agent[user].role`, beside the acting mode, and the `jti` at
/// `agent[user].policy`.
#[test]
fn the_roles_follow_the_acting_mode_and_the_token_id_is_the_policy() {
    let mut access = record(Action::Read, Outcome::Success);
    access.accessor.roles = vec![Coded {
        system: Some("urn:example:roles".to_owned()),
        code: "CLINICIAN".to_owned(),
    }];
    access.accessor.token_id = Some("Qz7-jti-81".to_owned());
    let written = written(&access);
    let user = requestor(&written);
    assert_eq!(user["role"][0]["coding"][0]["code"], "person");
    assert_eq!(user["role"][1]["coding"][0]["code"], "CLINICIAN");
    assert_eq!(user["role"][1]["coding"][0]["system"], "urn:example:roles");
    assert_eq!(user["policy"][0], "Qz7-jti-81");
}

/// An origin of stored data names its originating system, with no endpoint
/// and no status invented for it.
#[test]
fn an_origin_of_stored_data_is_named_by_its_system_alone() {
    let mut access = record(Action::Read, Outcome::Success);
    access.origins = vec![Origin {
        endpoint: None,
        node: None,
        system_id: Some("lab.example.org".to_owned()),
        status: None,
        rows: None,
        version: None,
        created_ehr: None,
        categories: None,
    }];
    access.origin_count = Some(40);
    let written = written(&access);
    let origin = entity(&written, "origin")
        .first()
        .copied()
        .expect("the origin entity");
    assert_eq!(origin["what"]["identifier"]["value"], "lab.example.org");
    assert!(details(origin, detail::STATUS).is_empty(), "{written}");
    assert_eq!(details(origin, detail::SYSTEM_ID), ["lab.example.org"]);
    assert_eq!(details(categories(&written), detail::ORIGIN_COUNT), ["40"]);
    assert_eq!(access.origins[0].key(), Some("lab.example.org"));
}

/// FHIR R4 `Identifier.system` is a URI, so an openEHR namespace is a
/// detail of the patient entity.
#[test]
fn a_patient_namespace_that_is_no_uri_is_a_detail() {
    let mut access = record(Action::Read, Outcome::Success);
    access.subject.patient = Some(PatientIdentifier {
        namespace: Some("DEMOGRAPHIC".to_owned()),
        value: SecretString::from("Qz7-patient-82"),
    });
    let written = written(&access);
    let patient = written["entity"]
        .as_array()
        .expect("entities")
        .iter()
        .find(|entity| entity["what"]["identifier"]["value"] == "Qz7-patient-82")
        .expect("the patient entity");
    assert!(
        patient["what"]["identifier"].get("system").is_none(),
        "{written}"
    );
    assert_eq!(details(patient, detail::PATIENT_NAMESPACE), ["DEMOGRAPHIC"]);
}

/// BALP's `entity:transaction` is `0..1`: a request with no id writes none,
/// and its correlation id rides on the categories entity.
#[test]
fn a_request_with_no_id_writes_no_transaction_entity() {
    let mut access = record(Action::Query, Outcome::Success);
    access.request.id = None;
    access.request.correlation = Some("Qz7-correlation-83".to_owned());
    let written = written(&access);
    assert!(
        written["entity"]
            .as_array()
            .expect("entities")
            .iter()
            .all(|entity| entity["type"]["code"] != "XrequestId"),
        "{written}"
    );
    assert_eq!(
        details(categories(&written), detail::CORRELATION),
        ["Qz7-correlation-83"]
    );
    assert!(!written.to_string().contains("UNKNOWN"), "{written}");
}

/// The operation is a second subtype in its system, after the pattern's.
#[test]
fn the_operation_is_a_subtype_when_its_system_is_named() {
    let mut access = record(Action::Query, Outcome::Success);
    access.request.operation_system = Some("urn:example:its-rest:operation".to_owned());
    let written = written(&access);
    assert_eq!(written["subtype"][0]["code"], "search");
    assert_eq!(written["subtype"][1]["code"], "query_execute_adhoc_query");
    assert_eq!(
        written["subtype"][1]["system"],
        "urn:example:its-rest:operation"
    );
    let plain = super::written(&record(Action::Query, Outcome::Success));
    assert!(
        plain["subtype"]
            .as_array()
            .is_some_and(|all| all.len() == 1)
    );
}

#[test]
fn a_major_failure_is_twelve_with_its_description() {
    let mut access = record(Action::Read, Outcome::MajorFailure);
    access.outcome_description = Some("the store is unavailable".to_owned());
    let written = written(&access);
    assert_eq!(written["outcome"], "12");
    assert_eq!(written["outcomeDesc"], "the store is unavailable");
}

/// A record kept without end names no end date, and states its period and
/// the ground of its floor.
#[test]
fn a_record_kept_without_end_names_no_end_date() {
    let mut access = record(Action::Read, Outcome::Success);
    let policy = RetentionPolicy::declare_within(
        &Bounds::union(),
        Period::Indefinite,
        &BTreeMap::new(),
        &BTreeMap::new(),
        &map(),
    )
    .expect("a policy");
    access.retention = policy.retain(jiff::Timestamp::UNIX_EPOCH, &access.categories, []);
    let written = written(&access);
    let kept = categories(&written);
    assert_eq!(details(kept, detail::RETENTION_PERIOD), ["indefinite"]);
    assert!(
        details(kept, detail::RETENTION_ENDS).is_empty(),
        "{written}"
    );
    assert!(
        details(kept, detail::RETENTION_YEARS).is_empty(),
        "{written}"
    );
    assert_eq!(
        details(kept, detail::RETENTION_FLOOR),
        ["eu-2025-327-art-9-2"]
    );
}

/// An access that reached an `EHR_STATUS` beside a composition says so.
#[test]
fn a_resource_kind_of_no_category_is_written_with_its_basis() {
    let mut access = record(Action::Read, Outcome::Success);
    access.categories = map().classify(&Evidence::reached(
        Basis::Returned,
        vec![
            RootObject {
                template_id: Some(LAB_REPORT.to_owned()),
                ..RootObject::default()
            },
            RootObject {
                kind: Some(ResourceKind::EhrStatus),
                ..RootObject::default()
            },
        ],
    ));
    let written = written(&access);
    let kept = categories(&written);
    assert_eq!(details(kept, detail::NO_CATEGORY_KIND), ["EHR_STATUS"]);
    assert_eq!(details(kept, detail::NO_CATEGORY_BASIS), ["resource-kind"]);
    assert_eq!(details(kept, detail::CATEGORY).len(), 1, "{written}");
    assert!(details(kept, detail::UNCLASSIFIED).is_empty(), "{written}");
}

/// A set the evidence limit cut writes its true count.
#[test]
fn a_cut_evidence_set_writes_its_true_count() {
    let mut access = record(Action::Query, Outcome::Success);
    let objects = (1..=5)
        .map(|n| RootObject {
            template_id: Some(LAB_REPORT.to_owned()),
            version_uid: Some(format!("8849182c-82ad-4088-a07f-48ead418051{n}::cdr::1")),
            ..RootObject::default()
        })
        .collect();
    access.categories = map()
        .with_evidence_limit(2)
        .classify(&Evidence::reached(Basis::Returned, objects));
    let written = written(&access);
    let kept = categories(&written);
    assert_eq!(details(kept, detail::VERSION_UID).len(), 2);
    assert_eq!(details(kept, detail::VERSION_UID_COUNT), ["5"]);
    assert!(
        details(kept, detail::TEMPLATE_ID_COUNT).is_empty(),
        "one template, not cut"
    );
}
