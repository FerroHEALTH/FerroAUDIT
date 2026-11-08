// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The access record written as a DICOM PS3.15 A.5 audit message (feature
//! `atna`), from the same exchange as its BALP record by the `RESTful` ATNA
//! Rev. 3.6 Table 3.20.4.2.2.1-1 mapping, with the participants the eHDSI
//! Audit Trail Profile 10.1.0 Patient Privacy schema names for a relayed
//! access (§2.3.13 to §2.3.15, §2.3.17, §2.3.41).

use std::collections::BTreeMap;

use base64::Engine as _;
use ehds_logging::atna::{AtnaError, audit_message};
use ehds_logging::record::{
    AccessClass, AccessRecord, Action, Binding, Coded, Correlation, CorrelationSource, IssuedId,
    Outcome, PatientIdentifier, Relayed, Representation, Via,
};
use ihe_atna::atna::message::AuditSource;
use ihe_atna::escape;
use quick_xml::XmlVersion;
use quick_xml::events::Event;
use secrecy::{ExposeSecret as _, SecretString};

use super::{PATIENT, gateway, record};
use crate::support::relayed::*;
use crate::support::{bound, contact_point, organisation};

/// One element of the written message: its name, its attributes as a reader
/// unescapes them, and its text.
#[derive(Debug)]
struct Element {
    name: String,
    attributes: BTreeMap<String, String>,
    text: String,
}

fn source() -> AuditSource {
    AuditSource {
        id: "XA".to_owned(),
        enterprise_site: None,
        type_codes: Vec::new(),
    }
}

fn xml(record: &AccessRecord) -> String {
    let message = audit_message(record, &gateway(), &source()).expect("a message");
    let bytes = message.to_xml().expect("the message is written");
    String::from_utf8(bytes.expose_secret().to_vec()).expect("UTF-8")
}

fn elements(text: &str) -> Vec<Element> {
    let mut reader = quick_xml::Reader::from_str(text);
    let mut elements: Vec<Element> = Vec::new();
    loop {
        match reader.read_event().expect("the message is XML") {
            Event::Start(start) | Event::Empty(start) => elements.push(Element {
                name: start.name().as_ref().to_owned(),
                attributes: start
                    .attributes()
                    .map(|attribute| {
                        let attribute = attribute.expect("an attribute");
                        (
                            attribute.key.as_ref().to_owned(),
                            attribute
                                .normalized_value(XmlVersion::Implicit1_0)
                                .expect("an unescaped value")
                                .into_owned(),
                        )
                    })
                    .collect(),
                text: String::new(),
            }),
            Event::Text(text) => {
                if let Some(last) = elements.last_mut() {
                    last.text
                        .push_str(&text.xml_content(XmlVersion::Implicit1_0));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    elements
}

fn participants(text: &str) -> Vec<BTreeMap<String, String>> {
    elements(text)
        .into_iter()
        .filter(|element| element.name == "ActiveParticipant")
        .map(|element| element.attributes)
        .collect()
}

fn participant<'a>(
    participants: &'a [BTreeMap<String, String>],
    user_id: &str,
) -> &'a BTreeMap<String, String> {
    participants
        .iter()
        .find(|participant| participant["UserID"] == user_id)
        .unwrap_or_else(|| {
            panic!(
                "no participant with the user id asked for among {}",
                participants.len()
            )
        })
}

/// The role codes of the participant `user_id`, as `(csd-code,
/// codeSystemName, originalText)`.
fn roles(text: &str, user_id: &str) -> Vec<(String, String, String)> {
    let mut roles = Vec::new();
    let mut inside = false;
    for element in elements(text) {
        match element.name.as_str() {
            "ActiveParticipant" => {
                inside = element.attributes.get("UserID").map(String::as_str) == Some(user_id);
            }
            "RoleIDCode" if inside => roles.push((
                element.attributes["csd-code"].clone(),
                element.attributes["codeSystemName"].clone(),
                element.attributes["originalText"].clone(),
            )),
            "RoleIDCode" => {}
            _ => inside = false,
        }
    }
    roles
}

/// Every detail of the participant object named `name`, decoded.
fn object_details(text: &str, name: &str) -> Vec<(String, String)> {
    let mut details = Vec::new();
    let mut inside = false;
    for element in elements(text) {
        match element.name.as_str() {
            "ParticipantObjectIdentification" => inside = false,
            "ParticipantObjectName" => inside = element.text == name,
            "ParticipantObjectDetail" if inside => {
                let value = base64::engine::general_purpose::STANDARD
                    .decode(&element.attributes["value"])
                    .expect("base64");
                details.push((
                    element.attributes["type"].clone(),
                    String::from_utf8(value).expect("UTF-8"),
                ));
            }
            _ => {}
        }
    }
    details
}

fn relayed_record() -> AccessRecord {
    let mut record = record(Action::Query, Outcome::Success);
    record.accessor.relayed = Some(Relayed::try_from(contact_point()).expect("a relayed access"));
    record.accessor.binding = Some(Binding::External);
    record
}

/// eHDSI Audit Trail Profile 10.1.0 §2.3.13 and §2.3.41: the requester's
/// `UserID` is `SPProvidedID<NameID@Issuer>`, its `AlternativeUserID` the
/// `subject-id` name and its `RoleIDCode` the role.
#[test]
fn the_relayed_professional_is_the_human_requestor() {
    let text = xml(&relayed_record());
    let participants = participants(&text);
    let requestor = participant(
        &participants,
        &format!("{SP_PROVIDED_ID}<{NAME_ID}@{IDA_ISSUER}>"),
    );
    assert_eq!(requestor["AlternativeUserID"], NAME);
    assert_eq!(requestor["UserIsRequestor"], "true");
    assert_eq!(
        roles(&text, &format!("{SP_PROVIDED_ID}<{NAME_ID}@{IDA_ISSUER}>")),
        [(ROLE.to_owned(), ISCO.to_owned(), ROLE.to_owned())]
    );
}

/// eHDSI Audit Trail Profile 10.1.0 §2.3.14, §2.3.15 and §2.3.40: each NCP
/// is named by the CN of its TLS certificate, the consumer as a requestor.
#[test]
fn each_ncp_is_named_by_its_certificate_cn() {
    let text = xml(&relayed_record());
    let participants = participants(&text);
    assert_eq!(
        participant(&participants, CONSUMER_NCP)["UserIsRequestor"],
        "true"
    );
    assert_eq!(
        participant(&participants, PROVIDER_NCP)["UserIsRequestor"],
        "false"
    );
    assert_eq!(
        roles(&text, CONSUMER_NCP),
        [(
            "ServiceConsumer".to_owned(),
            "eHealth DSI".to_owned(),
            "eHealth DSI Service Consumer".to_owned()
        )]
    );
    assert_eq!(
        roles(&text, PROVIDER_NCP),
        [(
            "ServiceProvider".to_owned(),
            "eHealth DSI".to_owned(),
            "eHealth DSI Service Provider".to_owned()
        )]
    );
}

/// eHDSI Audit Trail Profile 10.1.0 §2.3.17: the patient is type `1`, role
/// `1`, its id in HL7 v2 CX form, which is the pseudonym the recording
/// system's subject namespace gives it.
#[test]
fn the_patient_is_its_cx_identifier() {
    let text = xml(&relayed_record());
    let patient = elements(&text)
        .into_iter()
        .find(|element| {
            element.name == "ParticipantObjectIdentification"
                && element
                    .attributes
                    .get("ParticipantObjectTypeCodeRole")
                    .map(String::as_str)
                    == Some("1")
        })
        .expect("the patient");
    assert_eq!(patient.attributes["ParticipantObjectTypeCode"], "1");
    assert_eq!(
        patient.attributes["ParticipantObjectID"],
        format!("{PATIENT}^^^&2.999.1&ISO")
    );
}

/// HL7 v2.5 §2.7: a delimiter inside a component is written as its escape
/// sequence, so a value cannot add a component of its own.
#[test]
fn a_cx_delimiter_in_the_patient_id_is_escaped() {
    let mut record = relayed_record();
    record.subject.patient = Some(PatientIdentifier {
        namespace: Some("urn:oid:2.999.1".to_owned()),
        value: SecretString::from("Qz7^a&b~c|d\\e"),
    });
    let text = xml(&record);
    let ids: Vec<String> = elements(&text)
        .into_iter()
        .filter(|element| element.name == "ParticipantObjectIdentification")
        .map(|element| element.attributes["ParticipantObjectID"].clone())
        .collect();
    assert!(
        ids.contains(&r"Qz7\S\a\T\b\R\c\F\d\E\e^^^&2.999.1&ISO".to_owned()),
        "{ids:?}"
    );
    record.subject.patient = Some(PatientIdentifier {
        namespace: Some("national-pseudonyms".to_owned()),
        value: SecretString::from("Qz7-pseudonym-84"),
    });
    let text = xml(&record);
    assert!(
        text.contains("Qz7-pseudonym-84^^^national-pseudonyms"),
        "{text}"
    );
}

/// eHDSI Section I Security Policies 10.0.0 SR 5: the message carries the
/// assertion and message ids, never a security header, and no request or
/// response message object of the NCP non-repudiation entries (Audit Trail
/// Profile 10.1.0 §2.3.31, §2.3.32).
#[test]
fn the_message_carries_the_ids_and_no_security_header() {
    let text = xml(&relayed_record());
    assert!(!text.contains("securityheader"), "{text}");
    assert!(!text.contains(r#"csd-code="req""#), "{text}");
    let relayed = object_details(&text, "ehds-relayed");
    for (kind, value) in [
        ("identity-assertion-id", IDA),
        ("trc-assertion-id", TRC),
        ("request-message-id", MESSAGE),
        ("point-of-care", POINT_OF_CARE),
        ("hp-provider-identifier", HP_PROVIDER),
    ] {
        assert!(
            relayed.contains(&(kind.to_owned(), value.to_owned())),
            "{kind}: {relayed:?}"
        );
    }
}

/// ITI TF-2 §3.20.7.1.2: the purpose of use is the event's `PurposeOfUse`.
#[test]
fn the_purpose_of_use_is_the_events() {
    let text = xml(&relayed_record());
    let purposes: Vec<String> = elements(&text)
        .into_iter()
        .filter(|element| element.name == "PurposeOfUse")
        .map(|element| element.attributes["csd-code"].clone())
        .collect();
    assert_eq!(purposes, ["TREAT"]);
}

/// The event is the BALP record's, by the `RESTful` ATNA mapping: `type` is
/// the `EventID`, each `subtype` an `EventTypeCode`.
#[test]
fn the_event_is_the_balp_records() {
    let text = xml(&relayed_record());
    let event = elements(&text)
        .into_iter()
        .find(|element| element.name == "EventIdentification")
        .expect("the event");
    assert_eq!(event.attributes["EventActionCode"], "E");
    assert_eq!(event.attributes["EventOutcomeIndicator"], "0");
    let codes: Vec<(String, String)> = elements(&text)
        .into_iter()
        .filter(|element| element.name == "EventID" || element.name == "EventTypeCode")
        .map(|element| (element.name, element.attributes["csd-code"].clone()))
        .collect();
    assert_eq!(
        codes,
        [
            ("EventID".to_owned(), "rest".to_owned()),
            ("EventTypeCode".to_owned(), "search".to_owned())
        ]
    );
}

#[test]
fn the_binding_rides_as_a_participant_object() {
    let mut record = relayed_record();
    record.accessor.binding = Some(bound(false));
    let text = xml(&record);
    let accessor = object_details(&text, "ehds-accessor");
    assert!(
        accessor.contains(&("binding".to_owned(), "bound".to_owned())),
        "{accessor:?}"
    );
    assert!(
        accessor.contains(&(
            "party".to_owned(),
            "demographic|PERSON|Qz7-party-66".to_owned()
        )),
        "{accessor:?}"
    );
}

/// Each caller-kind refusal rides as the `binding` detail of the accessor
/// object, with no `security-event` detail.
#[test]
fn the_caller_kind_refusals_ride_as_details() {
    for binding in [
        Binding::RefusedSystemPersonalScope,
        Binding::RefusedNoAgreement,
        Binding::RefusedScopeBeyondAgreement,
        Binding::RefusedCorrelationMissing,
    ] {
        let code = binding.code();
        let mut record = record(Action::Read, Outcome::MinorFailure);
        record.accessor.binding = Some(binding);
        let accessor = object_details(&xml(&record), "ehds-accessor");
        assert!(
            accessor.contains(&("binding".to_owned(), code.to_owned())),
            "{code} in {accessor:?}"
        );
        assert!(
            !accessor.iter().any(|(kind, _)| kind == "security-event"),
            "{code} in {accessor:?}"
        );
    }
}

/// Each Wabvpz refusal rides as the `binding` detail of the accessor object,
/// with no `security-event` detail.
#[test]
fn the_wabvpz_refusals_ride_as_details() {
    for binding in [
        Binding::RefusedWabvpzRole,
        Binding::RefusedWabvpzOrganisationType,
    ] {
        let code = binding.code();
        let mut record = record(Action::Read, Outcome::MinorFailure);
        record.accessor.binding = Some(binding);
        let accessor = object_details(&xml(&record), "ehds-accessor");
        assert!(
            accessor.contains(&("binding".to_owned(), code.to_owned())),
            "{code} in {accessor:?}"
        );
        assert!(
            !accessor.iter().any(|(kind, _)| kind == "security-event"),
            "{code} in {accessor:?}"
        );
    }
}

/// The members PS3.15 A.5 has no element for ride as details of the accessor
/// object: the added refusals, the representation's relationship, the
/// provider acted for and each facility.
#[test]
fn the_added_binding_members_ride_as_details() {
    let mut record = record(Action::Create, Outcome::MinorFailure);
    record.accessor.binding = Some(Binding::RefusedTrcMissing);
    record.accessor.represents = Some(Representation {
        represented: "Qz7-represented-86".to_owned(),
        ground: None,
        relationship: Some(Coded {
            system: None,
            code: "10".to_owned(),
        }),
    });
    record.accessor.acts_for = Some(IssuedId {
        issuer: "urn:oid:2.999.7".to_owned(),
        id: "Qz7-provider-87".to_owned(),
    });
    record.accessor.facilities = vec![IssuedId {
        issuer: "urn:oid:2.999.8".to_owned(),
        id: "Qz7-ward-88".to_owned(),
    }];
    let accessor = object_details(&xml(&record), "ehds-accessor");
    for (kind, value) in [
        ("binding", "refused-trc-missing"),
        ("security-event", "true"),
        ("represents", "Qz7-represented-86"),
        ("represents-relationship", "10"),
        ("acts-for", "urn:oid:2.999.7|Qz7-provider-87"),
        ("facility", "urn:oid:2.999.8|Qz7-ward-88"),
    ] {
        assert!(
            accessor.contains(&(kind.to_owned(), value.to_owned())),
            "{kind} in {accessor:?}"
        );
    }
}

/// A federation professional named by no identifier is no participant,
/// since PS3.15 A.5.1 requires a `UserID`, and the message is still written.
#[test]
fn a_federation_professional_with_no_identifier_is_no_participant() {
    let mut parts = contact_point();
    parts.via = Via::Federation {
        issuer: FEDERATION.to_owned(),
    };
    parts.agreement = Some(crate::support::agreement(None));
    let professional = parts.professional.as_mut().expect("a professional");
    professional.identifier = None;
    professional.name_id = None;
    let mut record = record(Action::Read, Outcome::Success);
    record.accessor.relayed = Some(Relayed::try_from(parts).expect("a relayed access"));
    let text = xml(&record);
    assert!(
        participants(&text).iter().all(|participant| participant
            .get("AlternativeUserID")
            .map(String::as_str)
            != Some(NAME)),
        "no participant for the unidentified professional"
    );
    assert!(
        object_details(&text, "ehds-relayed").contains(&("hp-name".to_owned(), NAME.to_owned()))
    );
}

/// A hostile value in each member 0.5.0 adds leaves no raw character in the
/// message, and is base64-encoded as given in its detail (DICOM PS3.15
/// A.5.1.1 `ValuePair`).
#[test]
fn a_hostile_binding_value_is_written_escaped() {
    let hostile = format!("Qz7\u{0}\u{1b}[2J\u{202e}{}", "x".repeat(10_000));
    let mut record = record(Action::Read, Outcome::Success);
    record.accessor.represents = Some(Representation {
        represented: hostile.clone(),
        ground: None,
        relationship: Some(Coded {
            system: None,
            code: hostile.clone(),
        }),
    });
    record.accessor.acts_for = Some(IssuedId {
        issuer: hostile.clone(),
        id: hostile.clone(),
    });
    record.accessor.facilities = vec![IssuedId {
        issuer: hostile.clone(),
        id: hostile.clone(),
    }];
    let mut parts = organisation(AccessClass::Automated);
    parts.asserted = BTreeMap::from([(hostile.clone(), vec![hostile.clone()])]);
    record.accessor.relayed = Some(Relayed::try_from(parts).expect("a relayed access"));
    let text = xml(&record);
    assert!(
        text.chars()
            .all(|c| escape::untrusted(&c.to_string()) == c.to_string()),
        "a raw control character in the message"
    );
    let accessor = object_details(&text, "ehds-accessor");
    for (kind, value) in [
        ("represents", hostile.clone()),
        ("represents-relationship", hostile.clone()),
        ("acts-for", format!("{hostile}|{hostile}")),
        ("facility", format!("{hostile}|{hostile}")),
    ] {
        assert!(accessor.contains(&(kind.to_owned(), value)), "{kind}");
    }
    assert!(
        object_details(&text, "ehds-relayed")
            .contains(&("asserted-field".to_owned(), format!("{hostile}={hostile}")))
    );
}

/// An organisation admitted without a named person is the requestor, by its
/// `organization-id` and name, and no person is named for it.
#[test]
fn an_organisation_access_names_the_organisation_as_the_requestor() {
    let mut record = record(Action::Read, Outcome::Success);
    record.accessor.relayed =
        Some(Relayed::try_from(organisation(AccessClass::Automated)).expect("an organisation"));
    let text = xml(&record);
    let participants = participants(&text);
    let organisation = participant(&participants, HCP_ID);
    assert_eq!(organisation["UserName"], HCP_NAME);
    assert_eq!(organisation["UserIsRequestor"], "true");
    assert!(
        participants.iter().all(|participant| participant
            .get("AlternativeUserID")
            .map(String::as_str)
            != Some(NAME)),
        "{participants:?}"
    );
}

/// A record the rules refuse is refused here as by the sink.
#[test]
fn a_chained_access_with_no_attested_correlation_is_refused() {
    let mut record = record(Action::Read, Outcome::Success);
    record.accessor.relayed =
        Some(Relayed::try_from(organisation(AccessClass::Chained)).expect("an organisation"));
    record.request.correlation = Some(Correlation {
        value: "Qz7-correlation-85".to_owned(),
        source: CorrelationSource::Header,
    });
    assert!(matches!(
        audit_message(&record, &gateway(), &source()),
        Err(AtnaError::Invalid(_))
    ));
}

/// A hostile value a gateway asserts, with control characters, a
/// right-to-left override and ten thousand characters, is written escaped
/// in each attribute it reaches, and base64-encoded as given in a detail
/// (DICOM PS3.15 A.5.1.1 `ValuePair`).
#[test]
fn a_hostile_relayed_value_is_written_escaped() {
    let hostile = format!("Qz7\u{0}\u{1b}[2J\u{202e}{}", "x".repeat(10_000));
    let escaped = format!(r"Qz7\u{{0}}\u{{1b}}[2J\u{{202e}}{}", "x".repeat(10_000));
    let mut parts = contact_point();
    let professional = parts.professional.as_mut().expect("a professional");
    professional.name = Some(hostile.clone());
    professional.name_id.as_mut().expect("a name id").value = hostile.clone();
    let mut record = record(Action::Read, Outcome::Success);
    record.accessor.relayed = Some(Relayed::try_from(parts).expect("a relayed access"));
    let text = xml(&record);
    assert!(
        text.chars()
            .all(|c| escape::untrusted(&c.to_string()) == c.to_string()),
        "a raw control character in the message"
    );
    let participants = participants(&text);
    let requestor = participant(
        &participants,
        &format!("{SP_PROVIDED_ID}<{escaped}@{IDA_ISSUER}>"),
    );
    assert_eq!(requestor["AlternativeUserID"], escaped);
    assert!(
        object_details(&text, "ehds-relayed").contains(&("hp-name".to_owned(), hostile)),
        "the detail is not the value as given"
    );
}
