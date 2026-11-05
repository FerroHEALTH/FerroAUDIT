// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The DICOM PS3.15 A.5.1 audit message: its optional elements
//! `EventOutcomeDescription`, `AuditSourceTypeCode` and
//! `ParticipantObjectDataLifeCycle`, written in the schema's order when
//! given and left out when not, and the code tables products write it with.

use ihe_atna::atna::codes;
use ihe_atna::atna::message::{
    ActiveParticipant, AuditMessage, AuditSource, CodedValue, EventIdentification,
    ParticipantObject, SourceType,
};
use secrecy::{ExposeSecret as _, SecretString};

fn message(described: bool) -> AuditMessage {
    AuditMessage {
        event: EventIdentification {
            event_id: CodedValue::of(codes::EVENT_PATIENT_RECORD),
            action: "R",
            date_time: jiff::Timestamp::UNIX_EPOCH,
            outcome: "4",
            type_codes: Vec::new(),
            outcome_description: described.then(|| "the composition was not found".to_owned()),
            purposes_of_use: if described {
                vec![CodedValue::new(
                    "TREATMENT",
                    "eHDSI PurposeOfUse",
                    "TREATMENT",
                )]
            } else {
                Vec::new()
            },
        },
        participants: vec![ActiveParticipant {
            user_id: "Qz7-user-71".to_owned(),
            alternative_user_id: None,
            user_name: None,
            user_is_requestor: true,
            role_id_codes: vec![CodedValue::of(codes::ROLE_SOURCE_ROLE_ID)],
            access_point: None,
        }],
        source: AuditSource {
            id: "cdr.example.org".to_owned(),
            enterprise_site: None,
            type_codes: if described {
                vec![SourceType::of(codes::SOURCE_APPLICATION_SERVER)]
            } else {
                Vec::new()
            },
        },
        objects: vec![ParticipantObject {
            id: SecretString::from("Qz7-patient-72"),
            type_code: Some("1"),
            type_code_role: Some("1"),
            data_life_cycle: described.then_some(codes::LIFE_CYCLE_ACCESS),
            id_type_code: CodedValue::of(codes::OBJECT_ID_PATIENT_NUMBER),
            name: described.then(|| SecretString::from("Qz7-name-73")),
            query: None,
            details: Vec::new(),
        }],
    }
}

fn xml(message: &AuditMessage) -> String {
    let bytes = message.to_xml().expect("the message is written");
    String::from_utf8(bytes.expose_secret().to_vec()).expect("UTF-8")
}

/// DICOM PS3.15 A.5.1: `EventOutcomeDescription` follows the `EventID` and
/// any `EventTypeCode`, `AuditSourceTypeCode` is a child of
/// `AuditSourceIdentification`, and `ParticipantObjectDataLifeCycle` an
/// attribute of the participant object.
#[test]
fn the_optional_elements_are_written_in_the_schemas_order_when_given() {
    let text = xml(&message(true));
    let event_id = text.find("<EventID").expect("the event id");
    let description = text
        .find("<EventOutcomeDescription>the composition was not found</EventOutcomeDescription>")
        .expect("the outcome description");
    assert!(event_id < description, "{text}");
    assert!(
        text.contains(
            r#"<AuditSourceIdentification AuditSourceID="cdr.example.org"><AuditSourceTypeCode csd-code="4"/></AuditSourceIdentification>"#
        ),
        "{text}"
    );
    assert!(
        text.contains(r#"ParticipantObjectDataLifeCycle="6""#),
        "{text}"
    );
}

/// ITI TF-2 §3.20.7.1.2: a purpose of use "shall be encoded in the
/// `EventIdentification` section as `PurposeOfUse` element encoded as a
/// `CodedValueType`"; PS3.15 A.5.1 defines no place for it, so it follows
/// the DICOM elements.
#[test]
fn a_purpose_of_use_is_a_coded_element_of_the_event_after_the_dicom_ones() {
    let text = xml(&message(true));
    let purpose = text
        .find(r#"<PurposeOfUse csd-code="TREATMENT" codeSystemName="eHDSI PurposeOfUse" originalText="TREATMENT"/>"#)
        .expect("the purpose of use");
    let description = text
        .find("</EventOutcomeDescription>")
        .expect("the description");
    let end = text.find("</EventIdentification>").expect("the event");
    assert!(description < purpose && purpose < end, "{text}");
}

/// DICOM PS3.15 A.5.1: a participant object carries either a
/// `ParticipantObjectName` or a `ParticipantObjectQuery`, after its id type
/// and before its details; `RESTful` ATNA Rev. 3.6 Table 3.20.4.2.2.1-1 maps
/// it from `entity.name`.
#[test]
fn a_participant_objects_name_follows_its_id_type() {
    let text = xml(&message(true));
    let id_type = text
        .find("<ParticipantObjectIDTypeCode")
        .expect("the id type");
    let name = text
        .find("<ParticipantObjectName>Qz7-name-73</ParticipantObjectName>")
        .expect("the name");
    assert!(id_type < name, "{text}");
}

#[test]
fn a_query_is_written_in_place_of_the_name() {
    let mut message = message(true);
    message.objects[0].query = Some(SecretString::from("Qz7-query-74"));
    let text = xml(&message);
    assert!(text.contains("<ParticipantObjectQuery>"), "{text}");
    assert!(!text.contains("ParticipantObjectName"), "{text}");
}

#[test]
fn the_optional_elements_are_left_out_when_not_given() {
    let text = xml(&message(false));
    for absent in [
        "EventOutcomeDescription",
        "AuditSourceTypeCode",
        "ParticipantObjectDataLifeCycle",
        "PurposeOfUse",
        "ParticipantObjectName",
    ] {
        assert!(!text.contains(absent), "{absent}: {text}");
    }
    assert!(
        text.contains(r#"<AuditSourceIdentification AuditSourceID="cdr.example.org"/>"#),
        "{text}"
    );
}

/// DICOM PS3.15 A.5.3 and RFC 3881 §5.5.4, as the IHE audit tables write
/// them.
#[test]
fn the_code_tables_hold_the_codes_their_sources_define() {
    assert_eq!(codes::EVENT_QUERY, ("110112", "DCM", "Query"));
    assert_eq!(
        codes::EVENT_PATIENT_RECORD,
        ("110110", "DCM", "Patient Record")
    );
    assert_eq!(
        codes::EVENT_APPLICATION_ACTIVITY,
        ("110100", "DCM", "Application Activity")
    );
    assert_eq!(
        codes::EVENT_USER_AUTHENTICATION,
        ("110114", "DCM", "User Authentication")
    );
    assert_eq!(codes::TYPE_LOGIN, ("110122", "DCM", "Login"));
    assert_eq!(
        codes::ROLE_SOURCE_ROLE_ID,
        ("110153", "DCM", "Source Role ID")
    );
    assert_eq!(
        codes::ROLE_DESTINATION_ROLE_ID,
        ("110152", "DCM", "Destination Role ID")
    );
    assert_eq!(
        codes::OBJECT_ID_PATIENT_NUMBER,
        ("2", "RFC-3881", "Patient Number")
    );
    assert_eq!(
        codes::OBJECT_ID_SEARCH_CRITERIA,
        ("10", "RFC-3881", "Search Criteria")
    );
    assert_eq!(codes::OBJECT_ID_URI, ("12", "RFC-3881", "URI"));
    assert_eq!(codes::SOURCE_APPLICATION_SERVER, "4");
    assert_eq!(codes::LIFE_CYCLE_LOGICAL_DELETION, "14");
}
