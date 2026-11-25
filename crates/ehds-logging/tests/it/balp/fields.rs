// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The logging fields an assessor checks, each held to the BALP element the
//! record writes it to: the five points of Regulation (EU) 2025/327 Annex
//! II 3.2, the Xt-EHR D8.2 (2026-05-19) logging assertions §5.1.8, §5.1.9
//! and §5.1.12, and the Xt-EHR D5.2 (2026-04-09) §7.2.7 logging metadata
//! (`EEHRXFMDReq07`, `EEHRXFMDReq08`). `docs/conformance.md` lists each field
//! with the element and the test; a test's name starts with the field's
//! source, which the conformance badges count.

use super::*;

/// The agent whose `type` is `code` in its first coding.
fn agent<'a>(written: &'a Value, code: &str) -> &'a Value {
    written["agent"]
        .as_array()
        .expect("agents")
        .iter()
        .find(|agent| agent["type"]["coding"][0]["code"] == code)
        .unwrap_or_else(|| panic!("an agent of type {code}"))
}

/// The organisation agent, the provider of Annex II 3.2(a).
fn provider(written: &Value) -> &Value {
    written["agent"]
        .as_array()
        .expect("agents")
        .iter()
        .find(|agent| agent["who"]["type"] == "Organization")
        .expect("the provider's agent")
}

/// A read by a person, with the token's `jti`.
fn read() -> AccessRecord {
    let mut record = record(Action::Read, Outcome::Success);
    record.accessor.token_id = Some(TOKEN.to_owned());
    record.recorded = "2026-05-19T08:30:00Z".parse().expect("a timestamp");
    record
}

/// A read by another EHR system acting for nobody, over the API.
fn api_read() -> AccessRecord {
    let mut record = read();
    record.accessor.acting = Some(Acting::System);
    record.accessor.subject = CLIENT.to_owned();
    record.accessor.professional = Professional::default();
    record.accessor.alt_id = None;
    record.accessor.assurance = None;
    record
}

/// Annex II 3.2(a): the healthcare provider is an agent of its own, an
/// `Organization` named by its identifier.
#[test]
fn annex_ii_3_2_a_the_provider_is_an_organization_agent() {
    let written = written(&read());
    assert_eq!(
        provider(&written)["who"]["identifier"]["value"],
        PROVIDER,
        "Annex II 3.2(a): the healthcare provider"
    );
}

/// Annex II 3.2(b): the natural person is `agent:user`, by the token's
/// issuer and subject, with the professional's name and identifier
/// (BALP 1.1.4 §3:5.7.5.4).
#[test]
fn annex_ii_3_2_b_the_natural_person_is_the_user_agent() {
    let written = written(&read());
    let person = agent(&written, "IRCP");
    assert_eq!(person["requestor"], true, "agent:user is the requestor");
    assert_eq!(
        person["who"]["identifier"]["system"],
        "https://issuer.example.org"
    );
    assert_eq!(person["who"]["identifier"]["value"], SUBJECT);
    assert_eq!(person["who"]["display"], NAME);
    let other = extensions(person, "ihe-otherId");
    let [other] = other.as_slice() else {
        panic!("one ihe-otherId, got {other:?}");
    };
    assert_eq!(other["valueIdentifier"]["type"]["coding"][0]["code"], "NPI");
    assert_eq!(other["valueIdentifier"]["value"], IDENTIFIER);
}

/// Annex II 3.2(c): the categories are `ehds-category` details of the
/// `ehds-categories` entity, coded in HL7 Europe's priority category code
/// system, and its security labels.
#[test]
fn annex_ii_3_2_c_the_categories_are_the_categories_entity() {
    let written = written(&read());
    let categories = entity(&written, "ehds-categories");
    let [categories] = categories.as_slice() else {
        panic!("one categories entity, got {categories:?}");
    };
    assert_eq!(
        details(categories, "ehds-category"),
        [format!("{PRIORITY_SYSTEM}|Laboratory-Reports")]
    );
    assert_eq!(categories["securityLabel"][0]["system"], PRIORITY_SYSTEM);
    assert_eq!(categories["securityLabel"][0]["code"], "Laboratory-Reports");
}

/// Annex II 3.2(d): the time and date of access is `recorded`.
#[test]
fn annex_ii_3_2_d_the_time_and_date_is_recorded() {
    let written = written(&read());
    assert_eq!(written["recorded"], "2026-05-19T08:30:00Z");
}

/// Annex II 3.2(e): each origin is an `origin` entity named by its
/// endpoint, with the system its data came from.
#[test]
fn annex_ii_3_2_e_each_origin_is_an_origin_entity() {
    let written = written(&read());
    let origins = entity(&written, "origin");
    let [origin] = origins.as_slice() else {
        panic!("one origin, got {origins:?}");
    };
    assert_eq!(origin["what"]["identifier"]["value"], "node-a");
    assert_eq!(details(origin, "system-id"), ["cdr-a.example.org"]);
}

/// Xt-EHR D8.2 §5.1.8 log-UserAccess: an access by a person records every
/// point of Annex II 3.2 in one `AuditEvent`.
#[test]
fn d8_2_log_user_access_records_every_annex_point() {
    let written = written(&read());
    assert_eq!(provider(&written)["who"]["identifier"]["value"], PROVIDER);
    assert_eq!(
        agent(&written, "IRCP")["who"]["identifier"]["value"],
        SUBJECT
    );
    assert_eq!(entity(&written, "ehds-categories").len(), 1);
    assert!(written["recorded"].is_string(), "the time and date");
    assert_eq!(entity(&written, "origin").len(), 1);
}

/// Xt-EHR D8.2 §5.1.9 log-APIAccess: an access by another EHR system names
/// that system as the `Application` agent and by its network address, with
/// the categories, the time and date and the origins.
#[test]
fn d8_2_log_api_access_names_the_accessing_system() {
    let written = written(&api_read());
    assert_eq!(
        agent(&written, "110150")["who"]["identifier"]["value"],
        CLIENT
    );
    assert!(
        written["agent"]
            .as_array()
            .expect("agents")
            .iter()
            .any(|agent| agent["network"]["address"] == "192.0.2.7"),
        "the accessing system's address"
    );
    assert_eq!(entity(&written, "ehds-categories").len(), 1);
    assert!(written["recorded"].is_string(), "the time and date");
    assert_eq!(entity(&written, "origin").len(), 1);
}

/// Xt-EHR D8.2 §5.1.12 log-interoperability: each action's record claims
/// the BALP 1.1.4 pattern of its action, a `StructureDefinition` of the
/// vendored package, as an Audit Creator writes it.
#[test]
fn d8_2_log_interoperability_claims_a_balp_pattern() {
    let package =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/specs/ihe-balp/package");
    for action in [
        Action::Query,
        Action::Read,
        Action::Create,
        Action::Update,
        Action::Delete,
    ] {
        let written = written(&record(action, Outcome::Success));
        let claimed = profile(&written).expect("a claimed pattern");
        let name = claimed
            .strip_prefix("https://profiles.ihe.net/ITI/BALP/StructureDefinition/")
            .expect("a BALP pattern");
        assert!(
            package
                .join(format!("StructureDefinition-{name}.json"))
                .is_file(),
            "{action:?} claims {name}, which BALP 1.1.4 defines"
        );
    }
}

/// Xt-EHR D5.2 §7.2.7 actor: the person is `agent:user` and the client the
/// `Application` agent.
#[test]
fn d5_2_actor_is_the_user_and_the_client_agent() {
    let written = written(&read());
    assert_eq!(
        agent(&written, "IRCP")["who"]["identifier"]["value"],
        SUBJECT
    );
    assert_eq!(
        agent(&written, "110150")["who"]["identifier"]["value"],
        CLIENT
    );
}

/// Xt-EHR D5.2 §7.2.7 object: the data accessed is `entity:data` and the
/// patient `entity:patient`.
#[test]
fn d5_2_object_is_the_data_and_the_patient_entity() {
    let mut record = read();
    record.request.resource = Some("Observation/Qz7-observation-70".to_owned());
    let written = written(&record);
    let data = entity(&written, "query_execute_adhoc_query");
    let [data] = data.as_slice() else {
        panic!("one entity:data, got {data:?}");
    };
    assert_eq!(data["what"]["reference"], "Observation/Qz7-observation-70");
    let patient = written["entity"]
        .as_array()
        .expect("entities")
        .iter()
        .find(|entity| entity["role"]["code"] == "1")
        .expect("entity:patient");
    assert_eq!(patient["what"]["identifier"]["value"], PATIENT);
}

/// Xt-EHR D5.2 §7.2.7 timestamp: `recorded`.
#[test]
fn d5_2_timestamp_is_recorded() {
    let written = written(&read());
    assert_eq!(written["recorded"], "2026-05-19T08:30:00Z");
}

/// Xt-EHR D5.2 §7.2.7 action: `action` and the `restful-interaction`
/// subtype.
#[test]
fn d5_2_action_is_the_action_and_the_subtype() {
    let written = written(&read());
    assert_eq!(written["action"], "R");
    assert_eq!(
        written["subtype"][0]["system"],
        "http://hl7.org/fhir/restful-interaction"
    );
    assert_eq!(written["subtype"][0]["code"], "read");
}

/// Xt-EHR D5.2 §7.2.7 purpose, and the purpose-of-use indicators of its
/// event metadata: `purposeOfEvent` and `agent:user` `purposeOfUse`.
#[test]
fn d5_2_purpose_is_the_purpose_of_event_and_of_use() {
    let written = written(&read());
    assert_eq!(written["purposeOfEvent"][0]["coding"][0]["code"], "TREAT");
    assert_eq!(
        agent(&written, "IRCP")["purposeOfUse"][0]["coding"][0]["code"],
        "TREAT"
    );
}

/// Xt-EHR D5.2 §7.2.7 context: the recording system is `source.observer`
/// and the request `entity:transaction`.
#[test]
fn d5_2_context_is_the_observer_and_the_transaction() {
    let written = written(&read());
    assert_eq!(
        written["source"]["observer"]["identifier"]["value"],
        "gateway.example.org"
    );
    let transaction = written["entity"]
        .as_array()
        .expect("entities")
        .iter()
        .find(|entity| entity["type"]["code"] == "XrequestId")
        .expect("entity:transaction");
    assert_eq!(transaction["what"]["identifier"]["value"], "Qz7-request-46");
}

/// Xt-EHR D5.2 §7.2.7 role and access metadata: `agent:user` `role` and its
/// `policy`, the token's `jti` first.
#[test]
fn d5_2_roles_and_policies_are_the_user_agents() {
    let mut record = read();
    record.accessor.policies = vec![PERMIT.to_owned()];
    let written = written(&record);
    let person = agent(&written, "IRCP");
    assert_eq!(person["role"][0]["coding"][0]["code"], "person");
    assert_eq!(person["policy"], serde_json::json!([TOKEN, PERMIT]));
}

/// Xt-EHR D5.2 §7.2.7 event metadata: `type`, a standard event type.
#[test]
fn d5_2_event_type_is_the_audit_event_type() {
    let written = written(&read());
    assert_eq!(
        written["type"]["system"],
        "http://terminology.hl7.org/CodeSystem/audit-event-type"
    );
    assert_eq!(written["type"]["code"], "rest");
}

/// Xt-EHR D5.2 §7.2.7 regulatory metadata, the retention policy: the
/// period, its end and the legal ground of its floor are details of the
/// `ehds-categories` entity (Annex II 3.4).
#[test]
fn d5_2_retention_policy_is_the_categories_entitys() {
    let written = written(&read());
    let categories = entity(&written, "ehds-categories");
    let [categories] = categories.as_slice() else {
        panic!("one categories entity, got {categories:?}");
    };
    assert_eq!(details(categories, "ehds-retention-period"), ["P3Y"]);
    assert_eq!(details(categories, "ehds-retention-ends"), ["1973-01-02"]);
    assert_eq!(
        details(categories, "ehds-retention-floor"),
        ["eu-2025-327-art-9-2"]
    );
}
