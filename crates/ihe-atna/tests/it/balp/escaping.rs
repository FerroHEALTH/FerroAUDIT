// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! Every string the `AuditEvent` carries is escaped as the DICOM message's
//! are (`ihe_atna::escape`), so a JSON reader that decodes the record meets
//! no control character, no bidirectional control and no line separator,
//! and a value of any length is written whole; and the person a user's
//! token names in a person directory is the `agent:user` `otherId` typed
//! `PRN` (BALP 1.1.4 §3:5.7.5.4, `ihe_iua:person_id`).
#![expect(
    clippy::disallowed_types,
    reason = "the test seam: the written record is read as a JSON value"
)]

use ihe_atna::balp::{
    DESTINATION_ROLE, Described, Detail, Direction, Entity, EventKind, Exchange, NetworkAddress,
    OTHER, Outcome, Peer, READ, REST, SOURCE_ROLE, What,
};
use ihe_atna::escape;
use ihe_atna::user::{OnBehalfOf, PersonId, User};
use secrecy::ExposeSecret as _;
use serde_json::Value;
use url::Url;

use super::{CLIENT, ISSUER, SUBJECT, observer};

const HOSTILE: &str = "Qz7-a\u{0}b\u{1b}[31mc\u{202e}d\u{2028}e";
const ESCAPED: &str = r"Qz7-a\u{0}b\u{1b}[31mc\u{202e}d\u{2028}e";

fn exchange(on_behalf: OnBehalfOf, value: &str) -> Exchange {
    Exchange {
        kind: EventKind {
            profile: "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Read",
            event_type: REST,
            subtypes: vec![READ],
            action: "R",
            client: DESTINATION_ROLE,
            server: SOURCE_ROLE,
        },
        recorded: jiff::Timestamp::UNIX_EPOCH,
        outcome: Outcome::Success,
        outcome_description: Some(value.to_owned()),
        direction: Direction::Received {
            client: Peer {
                who: value.to_owned(),
                network: NetworkAddress::host("192.0.2.9"),
            },
            endpoint: NetworkAddress::uri(
                &Url::parse("https://gateway.example.org/").expect("a URL"),
            ),
        },
        on_behalf,
        entities: vec![Entity::Described(Described {
            what: Some(What::Identifier {
                system: None,
                value: value.to_owned(),
            }),
            kind: OTHER,
            role: None,
            name: Some(value.to_owned()),
            description: Some(value.to_owned()),
            details: vec![Detail::new(value, value)],
            security_labels: Vec::new(),
        })],
        security: Vec::new(),
    }
}

fn written(exchange: &Exchange) -> Value {
    let record = exchange
        .audit_event(&observer())
        .expect("the record is written");
    serde_json::from_slice(record.into_bytes().expose_secret()).expect("the record is JSON")
}

/// Every string of `value`, depth first.
fn strings(value: &Value) -> Vec<&str> {
    match value {
        Value::String(text) => vec![text.as_str()],
        Value::Array(items) => items.iter().flat_map(strings).collect(),
        Value::Object(members) => members.values().flat_map(strings).collect(),
        _ => Vec::new(),
    }
}

fn user(value: &str) -> OnBehalfOf {
    OnBehalfOf::User(
        User::new(ISSUER.to_owned(), value.to_owned(), CLIENT.to_owned())
            .with_name(Some(value.to_owned()))
            .with_alt_id(Some(value.to_owned())),
    )
}

#[test]
fn a_hostile_value_is_escaped_in_every_string_of_the_record() {
    let record = written(&exchange(user(HOSTILE), HOSTILE));
    for text in strings(&record) {
        assert_eq!(escape::untrusted(text), text, "{text:?} in {record}");
    }
    let escaped = strings(&record)
        .into_iter()
        .filter(|text| *text == ESCAPED)
        .count();
    assert!(escaped >= 8, "{escaped} escaped values in {record}");
}

#[test]
fn the_record_as_a_value_is_escaped_too() {
    let value = exchange(user(HOSTILE), HOSTILE)
        .audit_event_value(&observer())
        .expect("the record is written")
        .into_value();
    assert!(
        strings(&value)
            .iter()
            .all(|text| escape::untrusted(text) == *text),
        "{value}"
    );
}

#[test]
fn a_value_of_ten_thousand_characters_is_written_whole_and_escaped() {
    let long = "Qz7\u{202e}".repeat(2_500);
    let record = written(&exchange(user(&long), &long));
    assert_eq!(
        record["outcomeDesc"],
        r"Qz7\u{202e}".repeat(2_500),
        "the long value is cut or left raw"
    );
}

fn requestor(record: &Value) -> &Value {
    record["agent"]
        .as_array()
        .expect("agents")
        .iter()
        .find(|agent| agent["requestor"] == true)
        .expect("the agent:user")
}

/// BALP 1.1.4 §3:5.7.5.4 writes `ihe_iua:person_id` at
/// `agent[user].extension[otherId][provider-id]`, which the `ihe-otherId`
/// extension types `PRN` from HL7 v2 table 0203 (`OtherIdentifierTypesVS`).
#[test]
fn a_users_person_id_is_the_otherid_typed_prn() {
    let person = PersonId {
        system: Some("urn:oid:2.999.7".to_owned()),
        value: "Qz7-party-82".to_owned(),
    };
    let on_behalf = OnBehalfOf::User(
        User::new(ISSUER.to_owned(), SUBJECT.to_owned(), CLIENT.to_owned())
            .with_person_id(Some(person.clone())),
    );
    let record = written(&exchange(on_behalf, "Qz7-plain-83"));
    let other_ids: Vec<&Value> = requestor(&record)["extension"]
        .as_array()
        .expect("extensions")
        .iter()
        .filter(|extension| {
            extension["url"] == "https://profiles.ihe.net/ITI/BALP/StructureDefinition/ihe-otherId"
        })
        .collect();
    let [other_id] = other_ids.as_slice() else {
        panic!("one otherId, got {other_ids:?}");
    };
    let identifier = &other_id["valueIdentifier"];
    assert_eq!(
        identifier["type"]["coding"][0]["system"],
        "http://terminology.hl7.org/CodeSystem/v2-0203"
    );
    assert_eq!(identifier["type"]["coding"][0]["code"], "PRN");
    assert_eq!(identifier["system"], "urn:oid:2.999.7");
    assert_eq!(identifier["value"], "Qz7-party-82");
    let user = User::local(SUBJECT.to_owned()).with_person_id(Some(person));
    assert_eq!(
        user.person_id().map(|id| id.value.as_str()),
        Some("Qz7-party-82")
    );
    assert!(!format!("{user:?}").contains("Qz7-party-82"));
}

#[test]
fn a_user_with_no_person_id_writes_no_prn() {
    let record = written(&exchange(user("Qz7-plain-84"), "Qz7-plain-84"));
    assert!(!record.to_string().contains("\"PRN\""), "{record}");
}
