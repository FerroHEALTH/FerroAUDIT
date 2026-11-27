// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The bytes an exchange is written as, held to the bytes pinned for the
//! same exchange under `rendering/`: each exchange fills
//! every element a caller can, with text the escape rewrites in every value
//! a caller supplies, so a value written around the escape changes the bytes.
//! The written forms agree: the bytes, the JSON value, and the typed resource.

use std::path::PathBuf;

use ihe_atna::balp::{
    APPLICATION, AuditEventResource, CUSTODIAN, Coded, DELETE, DESTINATION_ROLE, DOMAIN_RESOURCE,
    Described, Detail, Direction, Entity, EventKind, Exchange, NetworkAddress, OTHER, Observer,
    Outcome, Peer, READ, REST, SEARCH, SOURCE_ROLE, SecurityLabel, What,
};
use ihe_atna::user::{AccessToken, Code, OnBehalfOf, PersonId, PurposeOfUse, User};
use secrecy::{ExposeSecret as _, SecretString};
use url::Url;

/// Text with a C0 control, an escape sequence, a bidirectional control and a
/// line separator, which the escape rewrites wherever it is written.
fn hostile(name: &str) -> String {
    format!("Qz7-{name}\u{0}\u{1b}[31m\u{202e}\u{2028}")
}

fn hostile_coded(name: &str) -> Coded {
    Coded::new(
        hostile(&format!("{name}-system")),
        hostile(&format!("{name}-code")),
        hostile(&format!("{name}-display")),
    )
}

fn hostile_code(name: &str) -> Code {
    Code {
        system: Some(hostile(&format!("{name}-system"))),
        code: hostile(&format!("{name}-code")),
    }
}

fn hostile_label(name: &str) -> SecurityLabel {
    SecurityLabel {
        system: hostile(&format!("{name}-system")),
        version: Some(hostile(&format!("{name}-version"))),
        code: hostile(&format!("{name}-code")),
    }
}

fn hostile_observer() -> Observer {
    Observer {
        source_id: hostile("source"),
        site: Some(hostile("site")),
        host: NetworkAddress::host(&hostile("host")),
        source_types: vec![hostile_coded("source-type")],
    }
}

/// A user whose every value is hostile, with every element a token can
/// name, and two purposes the escape writes alike: the raw control
/// character and the text the escape writes it as.
fn hostile_user() -> User {
    User::new(hostile("issuer"), hostile("subject"), hostile("client"))
        .with_name(Some(hostile("name")))
        .with_alt_id(Some(hostile("alt-id")))
        .with_audience(Some(hostile("audience")))
        .with_organisation(Some(hostile("organisation")))
        .with_organisation_name(Some(hostile("organisation-name")))
        .with_provider_identifier(Some(hostile("provider")))
        .with_person_id(Some(PersonId {
            system: Some(hostile("person-system")),
            value: hostile("person"),
        }))
        .with_assurance(Some(hostile_code("assurance")))
        .with_roles(vec![hostile_code("role-a"), hostile_code("role-b")])
        .with_purposes(vec![
            PurposeOfUse {
                system: Some(hostile("purpose-system")),
                code: "Qz7-purpose\u{7}".to_owned(),
                text: None,
            },
            PurposeOfUse {
                system: Some(hostile("purpose-system")),
                code: r"Qz7-purpose\u{7}".to_owned(),
                text: None,
            },
            PurposeOfUse {
                system: Some(hostile("purpose-system")),
                code: "Qz7-purpose\u{7}".to_owned(),
                text: None,
            },
            PurposeOfUse {
                system: None,
                code: hostile("purpose-bare"),
                text: None,
            },
        ])
}

/// Every entity form, each with hostile values.
fn hostile_entities() -> Vec<Entity> {
    vec![
        Entity::Query(SecretString::from(hostile("query"))),
        Entity::Patient {
            system: Some(hostile("patient-system")),
            value: SecretString::from(hostile("patient")),
            namespace: Some(hostile("patient-namespace")),
        },
        Entity::PatientReference(SecretString::from(hostile("patient-reference"))),
        Entity::Resource {
            reference: Some(hostile("resource")),
            resource_type: "Observation",
            kind: hostile_coded("resource-kind"),
            role: Some(hostile_coded("resource-role")),
            name: Some("Qz7-resource-name"),
            query: Some(hostile("resource-query")),
        },
        Entity::Described(Described {
            what: Some(What::Reference {
                reference: hostile("described-reference"),
                resource_type: Some(hostile("described-type")),
            }),
            kind: hostile_coded("described-kind"),
            role: Some(hostile_coded("described-role")),
            name: Some(hostile("described-name")),
            description: Some(hostile("described-description")),
            details: vec![
                Detail::new(hostile("detail-a"), hostile("detail-a-value")),
                Detail::new(hostile("detail-b"), hostile("detail-b-value")),
            ],
            security_labels: vec![hostile_label("entity-label")],
        }),
        Entity::Described(Described {
            what: Some(What::Identifier {
                system: Some(hostile("identifier-system")),
                value: hostile("identifier"),
            }),
            kind: OTHER,
            role: None,
            name: None,
            description: None,
            details: Vec::new(),
            security_labels: Vec::new(),
        }),
    ]
}

/// A search the recording system sent for a user, under the BALP Query
/// pattern, with an operation id among its subtypes.
fn sent_search() -> Exchange {
    Exchange {
        kind: EventKind {
            profile: "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Query",
            event_type: REST,
            subtypes: vec![SEARCH, hostile_coded("operation")],
            action: "E",
            client: SOURCE_ROLE,
            server: DESTINATION_ROLE,
        },
        recorded: jiff::Timestamp::from_second(1_767_225_600).expect("a timestamp"),
        outcome: Outcome::Success,
        outcome_description: Some(hostile("outcome")),
        direction: Direction::Sent {
            server: Peer {
                who: hostile("server"),
                network: NetworkAddress::host(&hostile("server-host")),
            },
        },
        on_behalf: OnBehalfOf::User(
            hostile_user()
                .with_token(Some(AccessToken::Jwt(hostile("token"))))
                .with_policies(vec![hostile("policy-a"), hostile("policy-b")]),
        ),
        entities: hostile_entities(),
        security: vec![hostile_label("label-a"), hostile_label("label-b")],
    }
}

/// A read the recording system served for a user whose token names its
/// `jti` and no other policy, which claims the OAuth Comprehensive pattern.
fn received_read() -> Exchange {
    Exchange {
        kind: EventKind {
            profile: "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Read",
            event_type: REST,
            subtypes: vec![READ],
            action: "R",
            client: DESTINATION_ROLE,
            server: SOURCE_ROLE,
        },
        recorded: jiff::Timestamp::from_second(1_767_225_601).expect("a timestamp"),
        outcome: Outcome::MinorFailure,
        outcome_description: Some(hostile("outcome")),
        direction: Direction::Received {
            client: Peer {
                who: hostile("client-peer"),
                network: NetworkAddress::host("192.0.2.9"),
            },
            endpoint: NetworkAddress::uri(
                &Url::parse("https://gateway.example.org/fhir").expect("a URL"),
            ),
        },
        on_behalf: OnBehalfOf::User(
            hostile_user().with_token(Some(AccessToken::Jwt(hostile("token")))),
        ),
        entities: hostile_entities(),
        security: Vec::new(),
    }
}

/// A failed delete under no profile, whose client is the Application agent,
/// so the user's application is named there alone.
fn failed_delete() -> Exchange {
    Exchange {
        kind: EventKind {
            profile: "",
            event_type: REST,
            subtypes: vec![DELETE],
            action: "D",
            client: APPLICATION,
            server: CUSTODIAN,
        },
        recorded: jiff::Timestamp::from_second(1_767_225_602).expect("a timestamp"),
        outcome: Outcome::SeriousFailure,
        outcome_description: None,
        direction: Direction::Sent {
            server: Peer::server(
                &Url::parse("https://u:p@fhir.example.org/r4?q#f").expect("a URL"),
            ),
        },
        on_behalf: OnBehalfOf::User(
            User::local(hostile("local-subject"))
                .with_client_id(Some(hostile("client")))
                .with_token(Some(AccessToken::Jwt(hostile("token")))),
        ),
        entities: vec![Entity::Resource {
            reference: None,
            resource_type: "Patient",
            kind: DOMAIN_RESOURCE,
            role: None,
            name: None,
            query: None,
        }],
        security: vec![hostile_label("label")],
    }
}

/// A consent decision the recording system made for a user's request, its
/// own agent written with no network.
fn decided_consent() -> Exchange {
    Exchange {
        kind: EventKind::authz_consent(),
        recorded: jiff::Timestamp::from_second(1_767_225_603).expect("a timestamp"),
        outcome: Outcome::Success,
        outcome_description: None,
        direction: Direction::Decided {
            client: Peer {
                who: hostile("decided-client"),
                network: NetworkAddress::host("192.0.2.10"),
            },
        },
        on_behalf: OnBehalfOf::User(
            User::local(hostile("decided-subject"))
                .with_organisation(Some(hostile("decided-organisation"))),
        ),
        entities: vec![
            Entity::consent(hostile("consent")),
            Entity::token(hostile("consent-token")),
        ],
        security: Vec::new(),
    }
}

/// An exchange the system made on its own behalf, with nothing optional.
fn system_minimal() -> Exchange {
    Exchange {
        kind: EventKind {
            profile: "",
            event_type: REST,
            subtypes: Vec::new(),
            action: "R",
            client: SOURCE_ROLE,
            server: DESTINATION_ROLE,
        },
        recorded: jiff::Timestamp::UNIX_EPOCH,
        outcome: Outcome::MajorFailure,
        outcome_description: None,
        direction: Direction::Sent {
            server: Peer {
                who: "Qz7-server".to_owned(),
                network: NetworkAddress::host("2001:db8::1"),
            },
        },
        on_behalf: OnBehalfOf::System,
        entities: Vec::new(),
        security: Vec::new(),
    }
}

fn plain_observer() -> Observer {
    Observer {
        source_id: "gateway.example.org".to_owned(),
        site: None,
        host: NetworkAddress::host("198.51.100.4"),
        source_types: Vec::new(),
    }
}

fn cases() -> [(&'static str, Exchange, Observer); 5] {
    [
        ("sent-search", sent_search(), hostile_observer()),
        ("received-read", received_read(), hostile_observer()),
        ("failed-delete", failed_delete(), hostile_observer()),
        ("decided-consent", decided_consent(), hostile_observer()),
        ("system-minimal", system_minimal(), plain_observer()),
    ]
}

fn expected(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/it/balp/rendering")
        .join(format!("{name}.json"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn the_bytes_are_the_pinned_bytes() {
    for (name, exchange, observer) in cases() {
        let record = exchange.audit_event(&observer).expect("the record");
        assert!(
            record.into_bytes().expose_secret() == expected(name).as_slice(),
            "{name}: the bytes differ from the pinned bytes"
        );
    }
}

#[test]
#[expect(
    clippy::disallowed_types,
    reason = "the test seam: the record as a JSON value is written as bytes"
)]
fn the_value_writes_the_pinned_bytes() {
    for (name, exchange, observer) in cases() {
        let value: serde_json::Value = exchange
            .audit_event_value(&observer)
            .expect("the record")
            .into_value();
        assert!(
            serde_json::to_vec(&value).expect("JSON") == expected(name),
            "{name}: the value differs from the pinned bytes"
        );
    }
}

#[test]
fn the_typed_resource_writes_the_pinned_bytes() {
    for (name, exchange, observer) in cases() {
        let resource: AuditEventResource = exchange.audit_event_resource(&observer);
        let record = resource.to_record().expect("the record");
        assert!(
            record.into_bytes().expose_secret() == expected(name).as_slice(),
            "{name}: the typed resource differs from the pinned bytes"
        );
    }
}

#[test]
fn the_typed_resource_reads_back_as_itself() {
    for (name, exchange, observer) in cases() {
        let resource = exchange.audit_event_resource(&observer).into_resource();
        let read: fhir_types::r4::audit_event::AuditEvent =
            serde_json::from_slice(&expected(name)).expect("an AuditEvent");
        assert!(
            read == resource,
            "{name}: the typed resource is not the one the bytes read as"
        );
    }
}

#[test]
fn a_withdrawable_record_holds_the_message_and_the_record_refused() {
    for (name, exchange, observer) in cases() {
        for outcome in [
            Outcome::Success,
            Outcome::MinorFailure,
            Outcome::SeriousFailure,
            Outcome::MajorFailure,
        ] {
            let exchange = Exchange {
                outcome,
                ..exchange.clone()
            };
            let refused = Exchange {
                outcome: Outcome::SeriousFailure,
                ..exchange.clone()
            };
            let pair = exchange.withdrawable(&observer).expect("the records");
            let message = exchange.audit_event(&observer).expect("the record");
            assert!(
                pair.message().expose_secret() == message.into_bytes().expose_secret(),
                "{name} {outcome:?}: the message is the record of the exchange"
            );
            let refusal = pair.into_refusal().expect("the refusal fits its place");
            let expected = refused.audit_event(&observer).expect("the record");
            assert!(
                refusal.expose_secret() == expected.into_bytes().expose_secret(),
                "{name} {outcome:?}: the refusal is the record with the outcome serious failure"
            );
        }
    }
}

/// Reads one JSON value and fails unless every object's keys are strictly
/// ascending by byte, the order `serde_json::Value` writes without
/// `preserve_order`, read with no `Value` so no build feature reaches it.
struct Ascending;

impl<'de> serde::de::DeserializeSeed<'de> for Ascending {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
        deserializer.deserialize_any(self)
    }
}

impl<'de> serde::de::Visitor<'de> for Ascending {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }

    fn visit_i64<E>(self, _: i64) -> Result<(), E> {
        Ok(())
    }

    fn visit_u64<E>(self, _: u64) -> Result<(), E> {
        Ok(())
    }

    fn visit_f64<E>(self, _: f64) -> Result<(), E> {
        Ok(())
    }

    fn visit_str<E>(self, _: &str) -> Result<(), E> {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while seq.next_element_seed(Ascending)?.is_some() {}
        Ok(())
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        let mut last: Option<String> = None;
        while let Some(key) = map.next_key::<String>()? {
            if let Some(last) = &last
                && last.as_bytes() >= key.as_bytes()
            {
                return Err(serde::de::Error::custom(format!("{key} follows {last}")));
            }
            map.next_value_seed(Ascending)?;
            last = Some(key);
        }
        Ok(())
    }
}

#[test]
fn every_object_of_the_bytes_holds_its_keys_in_ascending_order() {
    for (name, exchange, observer) in cases() {
        let record = exchange.audit_event(&observer).expect("the record");
        let bytes = record.into_bytes();
        let mut reader = serde_json::Deserializer::from_slice(bytes.expose_secret());
        serde::de::DeserializeSeed::deserialize(Ascending, &mut reader)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        reader.end().expect("one value");
    }
}
