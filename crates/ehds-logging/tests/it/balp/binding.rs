// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! How the recording system bound the accessor to a party (Regulation (EU)
//! 2025/327 Annex II 3.2(a), (b)): the binding outcome, the party and its
//! groups, the actor chain and the representation, in one entity
//! `ehds-accessor`; the party also as the person id BALP 1.1.4 §3:5.7.5.4
//! maps (`ihe_iua:person_id`, `agent[user]` `otherId` typed `PRN`); and a
//! record carrying every new member round-trips through the sink.

use std::sync::Arc;

use ehds_logging::balp::BalpSink;
use ehds_logging::record::{
    AccessClass, AccessRecord, Acting, Action, Actor, Binding, Coded, Correlation,
    CorrelationSource, IssuedId, Outcome, PartyRef, Relayed, Representation,
};
use ehds_logging::sink::AccessSink;
use ihe_atna::balp::{AuditRecorder, Observer};
use secrecy::ExposeSecret as _;

use super::{Kept, details, entity, gateway, person, record, written};
use crate::support::relayed::*;
use crate::support::{bound, contact_point, organisation, party};

fn accessor_entity(record: &AccessRecord) -> serde_json::Value {
    let written = written(record);
    let accessor = entity(&written, "ehds-accessor");
    let [accessor] = accessor.as_slice() else {
        panic!("one accessor entity, got {accessor:?}");
    };
    (*accessor).clone()
}

fn bound_record() -> AccessRecord {
    let mut record = record(Action::Read, Outcome::Success);
    record.accessor.binding = Some(bound(true));
    record.accessor.actors = vec![
        Actor {
            issuer: Some(FEDERATION.to_owned()),
            subject: "Qz7-gateway-76".to_owned(),
        },
        Actor {
            issuer: None,
            subject: "Qz7-client-77".to_owned(),
        },
    ];
    record.accessor.represents = Some(Representation {
        represented: "Qz7-represented-78".to_owned(),
        ground: Some(Coded {
            system: Some("https://example.org/representation".to_owned()),
            code: "guardian".to_owned(),
        }),
        relationship: Some(Coded {
            system: Some("openehr".to_owned()),
            code: "10".to_owned(),
        }),
    });
    record
}

#[test]
fn the_binding_its_party_and_groups_ride_in_the_accessor_entity() {
    let accessor = accessor_entity(&bound_record());
    assert_eq!(accessor["type"]["code"], "4");
    assert_eq!(details(&accessor, "binding"), ["bound"]);
    assert_eq!(details(&accessor, "guest"), ["true"]);
    assert_eq!(
        details(&accessor, "party"),
        ["demographic|PERSON|Qz7-party-66"]
    );
    assert_eq!(
        details(&accessor, "group"),
        [
            "demographic|GROUP|Qz7-group-67",
            "demographic|GROUP|Qz7-group-68"
        ]
    );
    assert_eq!(details(&accessor, "security-event"), Vec::<&str>::new());
}

/// RFC 8693 §4.1: the chain is recorded outermost first, as the token
/// nests it, and decides nothing.
#[test]
fn the_actor_chain_is_written_outermost_first() {
    let accessor = accessor_entity(&bound_record());
    assert_eq!(
        details(&accessor, "actor"),
        [
            format!("{FEDERATION}|Qz7-gateway-76"),
            "Qz7-client-77".to_owned()
        ]
    );
}

#[test]
fn a_representation_names_whom_and_on_which_ground() {
    let accessor = accessor_entity(&bound_record());
    assert_eq!(details(&accessor, "represents"), ["Qz7-represented-78"]);
    assert_eq!(
        details(&accessor, "represents-ground"),
        ["https://example.org/representation|guardian"]
    );
    assert_eq!(
        details(&accessor, "represents-relationship"),
        ["openehr|10"]
    );
}

/// A representative's token may state no ground, and the relationship may
/// be unrecorded: the record writes what it has and nothing in their place.
#[test]
fn a_representation_without_a_ground_or_relationship_writes_neither() {
    let mut record = bound_record();
    record.accessor.represents = Some(Representation {
        represented: "Qz7-represented-81".to_owned(),
        ground: None,
        relationship: None,
    });
    let accessor = accessor_entity(&record);
    assert_eq!(details(&accessor, "represents"), ["Qz7-represented-81"]);
    assert_eq!(details(&accessor, "represents-ground"), Vec::<&str>::new());
    assert_eq!(
        details(&accessor, "represents-relationship"),
        Vec::<&str>::new()
    );
}

/// The four refusals 0.4.0 lacked are written with their codes, and a
/// request with no treatment relationship confirmation as a security event.
#[test]
fn the_added_refusals_are_written_with_their_codes() {
    for (binding, code, security) in [
        (
            Binding::RefusedDeprovisioned,
            "refused-deprovisioned",
            false,
        ),
        (Binding::RefusedWrongMarker, "refused-wrong-marker", false),
        (
            Binding::RefusedRepresentationUnrecorded,
            "refused-representation-unrecorded",
            false,
        ),
        (Binding::RefusedTrcMissing, "refused-trc-missing", true),
    ] {
        let mut record = record(Action::Read, Outcome::MinorFailure);
        record.accessor.binding = Some(binding);
        let accessor = accessor_entity(&record);
        assert_eq!(details(&accessor, "binding"), [code]);
        assert_eq!(
            details(&accessor, "security-event"),
            if security { vec!["true"] } else { Vec::new() },
            "{code}"
        );
    }
}

/// Annex II 3.2(a): the provider a bound person or a system client acts
/// for, and each facility a system client's commit names, have no BALP
/// 1.1.4 element and ride in the accessor entity.
#[test]
fn the_provider_acted_for_and_each_facility_ride_in_the_accessor_entity() {
    let mut record = record(Action::Create, Outcome::Success);
    record.accessor.binding = Some(Binding::System);
    record.accessor.acts_for = Some(IssuedId {
        issuer: "urn:oid:2.999.7".to_owned(),
        id: "Qz7-provider-82".to_owned(),
    });
    record.accessor.facilities = vec![
        IssuedId {
            issuer: "urn:oid:2.999.8".to_owned(),
            id: "Qz7-ward-83".to_owned(),
        },
        IssuedId {
            issuer: "urn:oid:2.999.8".to_owned(),
            id: "Qz7-ward-84".to_owned(),
        },
    ];
    let accessor = accessor_entity(&record);
    assert_eq!(
        details(&accessor, "acts-for"),
        ["urn:oid:2.999.7|Qz7-provider-82"]
    );
    assert_eq!(
        details(&accessor, "facility"),
        ["urn:oid:2.999.8|Qz7-ward-83", "urn:oid:2.999.8|Qz7-ward-84"]
    );
}

/// A federation that asserts none of the contact-point fields writes none
/// of them, and each further field it asserted as its own detail.
#[test]
fn a_federation_access_writes_what_it_asserted_and_nothing_in_place_of_the_rest() {
    let mut parts = organisation(AccessClass::Automated);
    parts.country_code = None;
    parts
        .provider
        .as_mut()
        .expect("a provider")
        .issuing_authority = None;
    parts.asserted =
        std::collections::BTreeMap::from([("nl.ura".to_owned(), vec!["Qz7-ura-85".to_owned()])]);
    let mut record = record(Action::Read, Outcome::Success);
    record.accessor.relayed = Some(Relayed::try_from(parts).expect("a relayed access"));
    let written = written(&record);
    let relayed = entity(&written, "ehds-relayed");
    let [relayed] = relayed.as_slice() else {
        panic!("one relayed entity");
    };
    assert_eq!(details(relayed, "country-code"), Vec::<&str>::new());
    assert_eq!(
        details(relayed, "provider-issuing-authority"),
        Vec::<&str>::new()
    );
    assert_eq!(details(relayed, "asserted-field"), ["nl.ura=Qz7-ura-85"]);
}

#[test]
fn a_refusal_names_its_party_or_what_it_lacked() {
    let mut record = record(Action::Read, Outcome::MinorFailure);
    record.accessor.binding = Some(Binding::RefusedInactive {
        party: party("Qz7-party-79"),
    });
    let accessor = accessor_entity(&record);
    assert_eq!(details(&accessor, "binding"), ["refused-inactive"]);
    assert_eq!(
        details(&accessor, "party"),
        ["demographic|PERSON|Qz7-party-79"]
    );
    record.accessor.binding = Some(Binding::RefusedKeyClaimMissing {
        claim: "oid".to_owned(),
    });
    let accessor = accessor_entity(&record);
    assert_eq!(details(&accessor, "binding-claim"), ["oid"]);
    assert_eq!(details(&accessor, "party"), Vec::<&str>::new());
    record.accessor.binding = Some(Binding::RefusedExternalIncomplete {
        fields: vec!["hp_identifier".to_owned(), "organization-id".to_owned()],
    });
    let accessor = accessor_entity(&record);
    assert_eq!(
        details(&accessor, "binding-field"),
        ["hp_identifier", "organization-id"]
    );
}

/// A request whose patient is not the TRC patient is a security event (XCA
/// 8.0.1 §4.2.2 asks for an audit entry each time).
#[test]
fn a_trc_patient_mismatch_is_marked_a_security_event() {
    let mut record = record(Action::Query, Outcome::MinorFailure);
    record.accessor.binding = Some(Binding::RefusedTrcPatientMismatch);
    let accessor = accessor_entity(&record);
    assert_eq!(
        details(&accessor, "binding"),
        ["refused-trc-patient-mismatch"]
    );
    assert_eq!(details(&accessor, "security-event"), ["true"]);
}

#[test]
fn a_record_with_no_binding_actor_or_representation_has_no_accessor_entity() {
    let written = written(&record(Action::Read, Outcome::Success));
    assert_eq!(
        entity(&written, "ehds-accessor"),
        Vec::<&serde_json::Value>::new()
    );
}

/// BALP 1.1.4 §3:5.7.5.4: `ihe_iua:person_id` is
/// `agent[user].extension[otherId][provider-id]`, typed `PRN`.
#[test]
fn the_bound_party_is_the_users_person_id() {
    let written = written(&bound_record());
    let other_ids: Vec<&serde_json::Value> = person(&written)["extension"]
        .as_array()
        .expect("extensions")
        .iter()
        .filter(|extension| extension["valueIdentifier"]["type"]["coding"][0]["code"] == "PRN")
        .collect();
    let [other_id] = other_ids.as_slice() else {
        panic!("one PRN, got {other_ids:?}");
    };
    assert_eq!(other_id["valueIdentifier"]["value"], "Qz7-party-66");
    assert!(
        other_id["valueIdentifier"].get("system").is_none(),
        "a namespace that is no URI is no Identifier.system"
    );
}

#[test]
fn a_party_in_a_uri_namespace_is_the_person_ids_system() {
    let mut record = record(Action::Read, Outcome::Success);
    record.accessor.binding = Some(Binding::Provisional {
        party: ehds_logging::record::BoundParty {
            party: PartyRef {
                namespace: "urn:oid:2.999.80".to_owned(),
                ..party("Qz7-party-81")
            },
            groups: Vec::new(),
        },
    });
    let written = written(&record);
    let prn = person(&written)["extension"]
        .as_array()
        .expect("extensions")
        .iter()
        .find(|extension| extension["valueIdentifier"]["type"]["coding"][0]["code"] == "PRN")
        .expect("a PRN")
        .clone();
    assert_eq!(prn["valueIdentifier"]["system"], "urn:oid:2.999.80");
}

#[test]
fn each_new_acting_mode_is_the_users_role() {
    for (acting, code) in [
        (Acting::Delegated, "delegated"),
        (Acting::External, "external"),
        (Acting::System, "system"),
    ] {
        let mut record = record(Action::Read, Outcome::Success);
        record.accessor.acting = Some(acting);
        let written = written(&record);
        assert_eq!(person(&written)["role"][0]["coding"][0]["code"], code);
    }
}

/// The JSON of the one record a sink handed to its recorder.
fn kept_json(kept: &Kept) -> serde_json::Value {
    let observer = Observer {
        source_id: "gateway.example.org".to_owned(),
        site: None,
        host: ihe_atna::balp::NetworkAddress::host("gateway.example.org"),
        source_types: Vec::new(),
    };
    let exchanges = kept.kept.lock().expect("kept");
    let [exchange] = exchanges.as_slice() else {
        panic!("one record kept, got {}", exchanges.len());
    };
    let bytes = exchange
        .audit_event(&observer)
        .expect("a record")
        .into_bytes();
    serde_json::from_slice(bytes.expose_secret()).expect("JSON")
}

/// A record that carries every member this release adds reaches the
/// recorder through the sink with each value.
#[tokio::test]
async fn a_record_round_trips_through_the_sink_with_each_value() {
    let kept = Arc::new(Kept::default());
    let recorder: Arc<dyn AuditRecorder> = kept.clone();
    let mut record = bound_record();
    record.accessor.acting = Some(Acting::External);
    record.accessor.relayed = Some(Relayed::try_from(contact_point()).expect("a relayed access"));
    record.request.correlation = Some(Correlation {
        value: "Qz7-correlation-82".to_owned(),
        source: CorrelationSource::TokenClaim,
    });
    BalpSink::new(recorder, gateway())
        .store(record)
        .await
        .expect("stored");
    let text = kept_json(&kept).to_string();
    for value in [
        "external",
        "bound",
        "Qz7-party-66",
        "Qz7-group-67",
        "Qz7-group-68",
        "Qz7-gateway-76",
        "Qz7-client-77",
        "Qz7-represented-78",
        "guardian",
        "Qz7-correlation-82",
        "token-claim",
        CONTACT_POINT,
        NAME,
        HP_ID,
        HP_PROVIDER,
        NAME_ID,
        PERMISSION,
        HCP_ID,
        HOME_COMMUNITY,
        POINT_OF_CARE,
        IDA,
        TRC,
        MESSAGE,
        CONSUMER_NCP,
        PROVIDER_NCP,
    ] {
        assert!(text.contains(value), "{value} is missing from {text}");
    }
}

/// The sink refuses a record it cannot hold to its rules, so its caller
/// fails the access: a `chained` organisation access with no attested
/// correlation id.
#[tokio::test]
async fn the_sink_refuses_a_chained_access_with_no_attested_correlation() {
    let kept = Arc::new(Kept::default());
    let recorder: Arc<dyn AuditRecorder> = kept.clone();
    let sink = BalpSink::new(recorder, gateway());
    let mut record = record(Action::Read, Outcome::Success);
    record.accessor.relayed =
        Some(Relayed::try_from(organisation(AccessClass::Chained)).expect("an organisation"));
    record.request.correlation = Some(Correlation {
        value: "Qz7-correlation-83".to_owned(),
        source: CorrelationSource::Header,
    });
    assert!(sink.store(record.clone()).await.is_err());
    let reserved = sink.reserve().await.expect("a place");
    assert!(reserved.store(record).await.is_err());
    assert_eq!(kept.kept.lock().expect("kept").len(), 0);
}

/// A value a relaying gateway asserts is escaped in the written record
/// (`ihe_atna::escape`), whatever its length.
#[test]
fn a_hostile_relayed_value_is_written_escaped() {
    let hostile = format!("Qz7\u{0}\u{1b}[2J\u{202e}{}", "x".repeat(10_000));
    let mut parts = contact_point();
    parts.professional.as_mut().expect("a professional").name = Some(hostile.clone());
    let mut record = record(Action::Read, Outcome::Success);
    record.accessor.relayed = Some(Relayed::try_from(parts).expect("a relayed access"));
    let written = written(&record);
    let relayed = entity(&written, "ehds-relayed");
    let [relayed] = relayed.as_slice() else {
        panic!("one relayed entity");
    };
    assert_eq!(
        details(relayed, "hp-name"),
        [format!(
            r"Qz7\u{{0}}\u{{1b}}[2J\u{{202e}}{}",
            "x".repeat(10_000)
        )]
    );
    assert!(!written.to_string().contains('\u{202e}'));
}
