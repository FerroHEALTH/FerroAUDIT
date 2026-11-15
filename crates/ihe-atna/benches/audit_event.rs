// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! What a repository that serves its records typed, as an ITI-81 searchset
//! Bundle does, pays for one record: the JSON value read back into the
//! FHIR R4 `AuditEvent`, or the typed resource `ihe-atna` hands over; and
//! what a repository that stores the written record pays for its bytes or
//! its JSON value.
//! Run with `cargo bench -p ihe-atna --features balp --bench audit_event`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use fhir_types::r4::audit_event::AuditEvent;
use ihe_atna::balp::{
    AuditEventValue, DESTINATION_ROLE, Described, Detail, Direction, Entity, EventKind, Exchange,
    NetworkAddress, OTHER, Observer, Outcome, Peer, READ, REST, SOURCE_APPLICATION_SERVER,
    SOURCE_ROLE, SecurityLabel, What,
};
use ihe_atna::user::{Code, OnBehalfOf, PurposeOfUse, User};
use secrecy::SecretString;
use url::Url;

/// A read a user made of a patient's composition, as an EHR server records
/// the access: the user's token, roles and purposes, the patient, the
/// composition with its details, and the record's labels. Every value is
/// synthetic.
fn exchange() -> Option<Exchange> {
    let endpoint = Url::parse("https://ehr.example.org/fhir").ok()?;
    let user = User::new(
        "https://issuer.example.test".to_owned(),
        "Qz7-clinician-31".to_owned(),
        "Qz7-client-32".to_owned(),
    )
    .with_name(Some("Qz7 Clinician".to_owned()))
    .with_token_id(Some("Qz7-jti-33".to_owned()))
    .with_roles(vec![Code {
        system: Some("urn:oid:2.16.840.1.113883.2.4.15.111".to_owned()),
        code: "01.015".to_owned(),
    }])
    .with_purposes(vec![PurposeOfUse {
        system: Some("http://terminology.hl7.org/CodeSystem/v3-ActReason".to_owned()),
        code: "TREAT".to_owned(),
    }]);
    let label = |code: &str| SecurityLabel {
        system: "http://terminology.hl7.org/CodeSystem/v3-Confidentiality".to_owned(),
        version: None,
        code: code.to_owned(),
    };
    Some(Exchange {
        kind: EventKind {
            profile: "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.PatientRead",
            event_type: REST,
            subtypes: vec![READ],
            action: "R",
            client: DESTINATION_ROLE,
            server: SOURCE_ROLE,
        },
        recorded: jiff::Timestamp::from_second(1_767_225_600).ok()?,
        outcome: Outcome::Success,
        outcome_description: Some("Operation performed successfully".to_owned()),
        direction: Direction::Received {
            client: Peer {
                who: "Qz7-client-32".to_owned(),
                network: NetworkAddress::host("192.0.2.9"),
            },
            endpoint: NetworkAddress::uri(&endpoint),
        },
        on_behalf: OnBehalfOf::User(user),
        entities: vec![
            Entity::Patient {
                system: Some("urn:oid:2.16.840.1.113883.2.4.6.3".to_owned()),
                value: SecretString::from("Qz7-patient-34"),
                namespace: None,
            },
            Entity::Described(Described {
                what: Some(What::Identifier {
                    system: Some("urn:ietf:rfc:3986".to_owned()),
                    value: "Qz7-8849182c-82ad-4088-a07f-48ead4180515::ehr.example.org::1"
                        .to_owned(),
                }),
                kind: OTHER,
                role: None,
                name: Some("Qz7 encounter note".to_owned()),
                description: None,
                details: vec![
                    Detail::new("category", "patient-summary"),
                    Detail::new("template", "Qz7-encounter-note.v1"),
                    Detail::new("archetype", "openEHR-EHR-COMPOSITION.encounter.v1"),
                ],
                security_labels: vec![label("N")],
            }),
        ],
        security: vec![label("N")],
    })
}

fn observer() -> Observer {
    Observer {
        source_id: "ehr.example.org".to_owned(),
        site: Some("Qz7 site".to_owned()),
        host: NetworkAddress::host("ehr.example.org"),
        source_types: vec![SOURCE_APPLICATION_SERVER],
    }
}

fn typed_record(c: &mut Criterion) {
    let (Some(exchange), observer) = (exchange(), observer()) else {
        return;
    };
    let mut group = c.benchmark_group("typed AuditEvent of one record");
    // NOTE: the path a repository took before #233: the JSON value, read back typed.
    group.bench_function("value read back", |b| {
        b.iter(|| {
            black_box(&exchange)
                .audit_event_value(black_box(&observer))
                .map(AuditEventValue::into_value)
                .map(serde_json::from_value::<AuditEvent>)
        });
    });
    group.bench_function("typed resource", |b| {
        b.iter(|| {
            black_box(&exchange)
                .audit_event_resource(black_box(&observer))
                .into_resource()
        });
    });
    group.finish();
}

fn written_record(c: &mut Criterion) {
    let (Some(exchange), observer) = (exchange(), observer()) else {
        return;
    };
    let mut group = c.benchmark_group("written AuditEvent of one record");
    group.bench_function("bytes", |b| {
        b.iter(|| black_box(&exchange).audit_event(black_box(&observer)));
    });
    group.bench_function("value", |b| {
        b.iter(|| black_box(&exchange).audit_event_value(black_box(&observer)));
    });
    group.finish();
}

criterion_group!(benches, typed_record, written_record);
criterion_main!(benches);
