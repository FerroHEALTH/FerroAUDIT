// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! What a record writes beyond its pattern's fixed codings: a subtype known
//! only at run time, the DICOM event codes, the major-failure outcome,
//! `outcomeDesc` and `source.type` (FHIR R4 `AuditEvent`), the token's `jti`
//! at `agent[user].policy` (BALP 1.1.4 §3:5.7.5.4) with any other policy
//! after it, a user with no OAuth
//! identity, an endpoint known by its host, a patient identifier whose
//! namespace is no URI, and the record as a JSON value.
#![expect(
    clippy::disallowed_types,
    reason = "the test seam: the written record and the vendored profile are read as JSON values"
)]

use std::path::PathBuf;

use ihe_atna::balp::{
    APPLICATION, CUSTODIAN, Coded, DELETE, DESTINATION_ROLE, Direction, EXPORT, Entity, EventKind,
    Exchange, IMPORT, LOGIN, LOGOUT, NetworkAddress, OAUTH_COMPREHENSIVE, OPERATION, Observer,
    Outcome, PATIENT_NAMESPACE, Peer, READ, REST, SOURCE_APPLICATION_SERVER, SOURCE_ROLE,
    USER_AUTHENTICATION,
};
use ihe_atna::user::{OnBehalfOf, User};
use secrecy::{ExposeSecret as _, SecretString};
use serde_json::Value;
use url::Url;

use super::{CLIENT, ISSUER, SUBJECT, observer};

/// The synthetic token id, unlike any other text.
const TOKEN: &str = "Qz7-jti-61";
/// The synthetic data permits, unlike any other text.
const PERMIT: &str = "urn:uuid:5b0f3c2e-8d1a-4f6b-9c7e-2a4d6f8b0c11";
const SECOND_PERMIT: &str = "Qz7-permit-62";

fn read_kind() -> EventKind {
    EventKind {
        profile: "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Read",
        event_type: REST,
        subtypes: vec![READ],
        action: "R",
        client: DESTINATION_ROLE,
        server: SOURCE_ROLE,
    }
}

fn exchange(on_behalf: OnBehalfOf) -> Exchange {
    Exchange {
        kind: read_kind(),
        recorded: jiff::Timestamp::UNIX_EPOCH,
        outcome: Outcome::Success,
        outcome_description: None,
        direction: Direction::Received {
            client: Peer::server(&Url::parse("https://client.example.org/").expect("a URL")),
            endpoint: NetworkAddress::uri(
                &Url::parse("https://gateway.example.org/v1/ehr").expect("a URL"),
            ),
        },
        on_behalf,
        entities: Vec::new(),
        security: Vec::new(),
    }
}

fn written_by(exchange: &Exchange, observer: &Observer) -> Value {
    let record = exchange
        .audit_event(observer)
        .expect("the record is written");
    serde_json::from_slice(record.into_bytes().expose_secret()).expect("the record is JSON")
}

fn written(exchange: &Exchange) -> Value {
    written_by(exchange, &observer())
}

fn agents(record: &Value) -> &Vec<Value> {
    record["agent"].as_array().expect("agents")
}

fn requestor(record: &Value) -> &Value {
    agents(record)
        .iter()
        .find(|agent| agent["requestor"] == true)
        .expect("the agent:user")
}

fn profiles(record: &Value) -> Vec<&str> {
    record["meta"]["profile"]
        .as_array()
        .map(|profiles| profiles.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

fn is_application(agent: &Value) -> bool {
    agent["type"]["coding"][0]["code"] == APPLICATION.code.as_ref()
}

/// An `EventKind` can carry a subtype known only when the exchange is made,
/// after the codes the pattern fixes, as every `RESTful` pattern's open
/// `subtype` slicing admits.
#[test]
fn a_subtype_known_at_run_time_follows_the_fixed_one() {
    let mut exchange = exchange(OnBehalfOf::System);
    let operation = String::from("composition_get");
    exchange.kind.subtypes.push(Coded::new(
        "urn:example:its-rest:operation",
        operation.clone(),
        operation,
    ));
    let record = written(&exchange);
    assert_eq!(record["subtype"][0]["code"], "read");
    assert_eq!(record["subtype"][1]["code"], "composition_get");
    assert_eq!(
        record["subtype"][1]["system"],
        "urn:example:its-rest:operation"
    );
}

/// DICOM PS3.15 Tables A.5.3.4-1, A.5.3.5-1 and A.5.3.12-1, and FHIR R4
/// `restful-interaction` `operation`.
#[test]
fn the_event_codes_are_the_ones_their_tables_fix() {
    let dcm = "http://dicom.nema.org/resources/ontology/DCM";
    for (coded, code, display) in [
        (&USER_AUTHENTICATION, "110114", "User Authentication"),
        (&LOGIN, "110122", "Login"),
        (&LOGOUT, "110123", "Logout"),
        (&EXPORT, "110106", "Export"),
        (&IMPORT, "110107", "Import"),
    ] {
        assert_eq!(coded.system, dcm, "{code}");
        assert_eq!(coded.code, code, "{code}");
        assert_eq!(coded.display, display, "{code}");
    }
    assert_eq!(OPERATION.system, "http://hl7.org/fhir/restful-interaction");
    assert_eq!(OPERATION.code, "operation");
}

/// FHIR R4 `audit-event-outcome` and DICOM PS3.15 A.5.1 define `12`.
#[test]
fn a_major_failure_is_written_as_twelve() {
    let mut exchange = exchange(OnBehalfOf::System);
    exchange.outcome = Outcome::MajorFailure;
    assert_eq!(written(&exchange)["outcome"], "12");
}

/// FHIR R4 `AuditEvent.outcomeDesc` and `AuditEvent.source.type`.
#[test]
fn the_outcome_description_and_the_source_type_are_written_when_given() {
    let plain = written(&exchange(OnBehalfOf::System));
    assert!(plain.get("outcomeDesc").is_none(), "{plain}");
    assert!(plain["source"].get("type").is_none(), "{plain}");

    let mut exchange = exchange(OnBehalfOf::System);
    exchange.kind.profile = "";
    exchange.outcome = Outcome::MinorFailure;
    exchange.outcome_description = Some("the composition was not found".to_owned());
    let observer = Observer {
        source_types: vec![SOURCE_APPLICATION_SERVER],
        ..observer()
    };
    let record = written_by(&exchange, &observer);
    assert_eq!(record["outcomeDesc"], "the composition was not found");
    let kind = &record["source"]["type"][0];
    assert_eq!(
        kind["system"],
        "http://terminology.hl7.org/CodeSystem/security-source-type"
    );
    assert_eq!(kind["code"], "4");
}

/// `RESTful` ATNA Table 3.81.4.2.2.1-1 maps the audit source id to
/// `source.observer`, and the ITI-81 `source.identifier` parameter matches
/// `source.observer.identifier` (§3.81.4.1.2.2), so the record carries it
/// beside the display.
#[test]
fn the_audit_source_id_is_the_observers_identifier() {
    let record = written(&exchange(OnBehalfOf::System));
    let observer = &record["source"]["observer"];
    assert_eq!(
        observer["identifier"]["value"], "gateway.example.org",
        "{record}"
    );
    assert_eq!(observer["display"], "gateway.example.org", "{record}");
}

/// BALP 1.1.4 §3:5.7.5.4 writes the token's `jti` at `agent[user].policy`,
/// and a record that names it and the client claims the Comprehensive
/// pattern, whose `agent:oUser.policy` is `1..1` and whose `110150` client
/// names the `client_id` in `who.identifier.value`.
#[test]
fn the_token_id_is_the_users_policy_and_claims_the_comprehensive_pattern() {
    let user = User::new(ISSUER.to_owned(), SUBJECT.to_owned(), CLIENT.to_owned())
        .with_token_id(Some(TOKEN.to_owned()));
    let record = written(&exchange(OnBehalfOf::User(user)));
    assert_eq!(requestor(&record)["policy"][0], TOKEN);
    assert!(profiles(&record).contains(&OAUTH_COMPREHENSIVE), "{record}");
    let client = agents(&record)
        .iter()
        .find(|agent| is_application(agent))
        .expect("the application agent");
    assert_eq!(client["who"]["identifier"]["value"], CLIENT);

    let profile =
        vendored("StructureDefinition-IHE.BasicAudit.OAUTHaccessTokenUse.Comprehensive.json");
    assert_eq!(profile["url"], OAUTH_COMPREHENSIVE);
    let policy = profile["differential"]["element"]
        .as_array()
        .expect("elements")
        .iter()
        .find(|element| element["path"] == "AuditEvent.agent.policy")
        .expect("the policy element");
    assert_eq!(policy["min"], 1);
}

/// A user with no token id claims no token pattern, and neither does a
/// Delete record, whose only `110150` agent is its client.
#[test]
fn a_record_that_does_not_hold_to_the_token_pattern_does_not_claim_it() {
    let without = User::new(ISSUER.to_owned(), SUBJECT.to_owned(), CLIENT.to_owned());
    let record = written(&exchange(OnBehalfOf::User(without)));
    assert!(
        !profiles(&record).contains(&OAUTH_COMPREHENSIVE),
        "{record}"
    );
    assert!(requestor(&record).get("policy").is_none(), "{record}");

    let mut delete = exchange(OnBehalfOf::User(
        User::new(ISSUER.to_owned(), SUBJECT.to_owned(), CLIENT.to_owned())
            .with_token_id(Some(TOKEN.to_owned())),
    ));
    delete.kind = EventKind {
        profile: "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Delete",
        event_type: REST,
        subtypes: vec![DELETE],
        action: "D",
        client: APPLICATION,
        server: CUSTODIAN,
    };
    let record = written(&delete);
    assert!(
        !profiles(&record).contains(&OAUTH_COMPREHENSIVE),
        "{record}"
    );
    assert_eq!(
        requestor(&record)["policy"][0],
        TOKEN,
        "the jti is still written"
    );
}

/// A policy the user acts under beside the token, such as a data permit, is
/// another `agent[user].policy` after the `jti` (FHIR R4
/// `AuditEvent.agent.policy`, `0..*`), and the record then claims no
/// Comprehensive pattern, whose `agent:oUser.policy` is `..1`.
#[test]
fn a_policy_beside_the_token_follows_the_jti_and_drops_the_token_pattern() {
    let user = User::new(ISSUER.to_owned(), SUBJECT.to_owned(), CLIENT.to_owned())
        .with_token_id(Some(TOKEN.to_owned()))
        .with_policies(vec![PERMIT.to_owned(), SECOND_PERMIT.to_owned()]);
    let record = written(&exchange(OnBehalfOf::User(user)));
    assert_eq!(
        requestor(&record)["policy"],
        serde_json::json!([TOKEN, PERMIT, SECOND_PERMIT]),
        "{record}"
    );
    assert!(
        !profiles(&record).contains(&OAUTH_COMPREHENSIVE),
        "{record}"
    );
    let profile =
        vendored("StructureDefinition-IHE.BasicAudit.OAUTHaccessTokenUse.Comprehensive.json");
    let policy = profile["differential"]["element"]
        .as_array()
        .expect("elements")
        .iter()
        .find(|element| element["path"] == "AuditEvent.agent.policy")
        .expect("the policy element");
    assert_eq!(policy["max"], "1");

    let local = User::local(SUBJECT.to_owned()).with_policies(vec![PERMIT.to_owned()]);
    let record = written(&exchange(OnBehalfOf::User(local)));
    assert_eq!(
        requestor(&record)["policy"],
        serde_json::json!([PERMIT]),
        "a user with no token keeps the permit alone"
    );
}

/// A user authenticated with no OAuth token is named by their account alone:
/// no issuer system and no Application agent, since neither exists.
#[test]
fn a_local_user_writes_no_issuer_and_no_client() {
    let record = written(&exchange(OnBehalfOf::User(User::local(SUBJECT.to_owned()))));
    let user = requestor(&record);
    assert_eq!(user["who"]["identifier"]["value"], SUBJECT);
    assert!(
        user["who"]["identifier"].get("system").is_none(),
        "{record}"
    );
    assert!(
        agents(&record).iter().all(|agent| !is_application(agent)),
        "{record}"
    );
    assert!(!record.to_string().contains("UNKNOWN"), "{record}");
    assert_eq!(User::local(SUBJECT.to_owned()).atna_user_name(), SUBJECT);
}

/// FHIR R4 `network-type`: a recording system known by its host or IP is
/// written as a machine name (`1`) or an IP address (`2`).
#[test]
fn an_endpoint_known_by_its_host_or_ip_is_written_with_its_type() {
    for (host, kind) in [("cdr.example.org", "1"), ("192.0.2.17", "2")] {
        let mut exchange = exchange(OnBehalfOf::System);
        exchange.direction = Direction::Received {
            client: Peer::server(&Url::parse("https://client.example.org/").expect("a URL")),
            endpoint: NetworkAddress::host(host),
        };
        let record = written(&exchange);
        let own = &agents(&record)[1];
        assert_eq!(own["network"]["address"], host);
        assert_eq!(own["network"]["type"], kind);
    }
}

/// FHIR R4 `Identifier.system` is a URI: a namespace that is none is a
/// `detail` of the patient entity, and an identifier may have no system.
#[test]
fn a_namespace_that_is_no_uri_never_reaches_the_identifier_system() {
    let mut exchange = exchange(OnBehalfOf::System);
    exchange.entities = vec![
        Entity::Patient {
            system: None,
            value: SecretString::from("Qz7-patient-62"),
            namespace: Some("DEMOGRAPHIC".to_owned()),
        },
        Entity::Patient {
            system: None,
            value: SecretString::from("Qz7-patient-63"),
            namespace: None,
        },
    ];
    let record = written(&exchange);
    let first = &record["entity"][0];
    assert!(
        first["what"]["identifier"].get("system").is_none(),
        "{record}"
    );
    assert_eq!(first["detail"][0]["type"], PATIENT_NAMESPACE);
    assert_eq!(first["detail"][0]["valueString"], "DEMOGRAPHIC");
    let second = &record["entity"][1];
    assert!(second.get("detail").is_none(), "{record}");
    assert_eq!(second["what"]["identifier"]["value"], "Qz7-patient-63");
}

/// The record as a JSON value is the record the bytes hold, and its `Debug`
/// shows none of it.
#[test]
fn the_record_as_a_value_equals_the_bytes_and_shows_nothing() {
    let exchange = exchange(OnBehalfOf::User(User::new(
        ISSUER.to_owned(),
        SUBJECT.to_owned(),
        CLIENT.to_owned(),
    )));
    let value = exchange
        .audit_event_value(&observer())
        .expect("the record is written");
    assert!(!format!("{value:?}").contains(SUBJECT), "{value:?}");
    assert_eq!(value.into_value(), written(&exchange));
}

/// A vendored BALP `StructureDefinition`, by its file name.
fn vendored(file: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/specs/ihe-balp/package")
        .join(file);
    serde_json::from_slice(&std::fs::read(&path).expect("the vendored profile")).expect("JSON")
}
