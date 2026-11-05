// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! An access record written as a DICOM PS3.15 A.5 audit message (feature
//! `atna`), the form an ITI-20 syslog message carries.
//!
//! The message is written from the exchange the BALP record is written from
//! ([`crate::balp::exchange_with`]), by the mapping `RESTful` ATNA Rev. 3.6
//! Table 3.20.4.2.2.1-1 makes normative between the two forms, so the two
//! renderings of one record never disagree:
//!
//! - `type` is the `EventID`, each `subtype` an `EventTypeCode`, `action`,
//!   `recorded`, `outcome` and `outcomeDesc` the event's attributes, and the
//!   accessor's purposes of use its `PurposeOfUse` (ITI TF-2 §3.20.7.1.2);
//! - each `entity` is a `ParticipantObjectIdentification`: `what` its
//!   `ParticipantObjectID`, `type` and `role` its type code and role, `name`
//!   its `ParticipantObjectName`, `query` its `ParticipantObjectQuery` and
//!   each `detail` a `ParticipantObjectDetail`. An entity named by no `what`
//!   takes its name as its id, and its `type` coding as its id type. The
//!   patient is type `1`, role `1`, its id the HL7 v2 CX form of the
//!   identifier the record names, `<id>^^^&<oid>&ISO` for a `urn:oid:`
//!   namespace (eHDSI Audit Trail Profile 10.1.0 §2.3.17), each HL7 v2
//!   delimiter in a component written as its escape sequence;
//! - the agents are `ActiveParticipant`s: the person the token names, with
//!   the IUA `UserName` (ITI TF-2 §3.72.5.1), the client and the recording
//!   system with the role codes of the pattern, and the provider the person
//!   acts for.
//!
//! A relayed access adds the participants the eHDSI Audit Trail Profile
//! 10.1.0 Patient Privacy schema names: the human requestor, whose `UserID`
//! is `SPProvidedID<NameID@Issuer>` (§2.3.13, §2.3.41), its
//! `AlternativeUserID` the `subject-id` name and its `RoleIDCode` the role,
//! and each contact point by the CN of its TLS certificate, the service
//! consumer as a requestor (§2.3.14, §2.3.15, §2.3.40). An organisation
//! without a named person is the requestor itself, by its `organization-id`
//! and name. The message carries the assertion and message ids as details
//! and never a security header: eHDSI Section I Security Policies 10.0.0 SR
//! 5 has audit logs "not contain clinical payload", and the full header the
//! Audit Trail Profile §2.3.30 to §2.3.32 logs is the contact points' own
//! non-repudiation record (our own design at the data holder).
//!
//! Where DICOM PS3.15 A.5 has no element for a member, the member rides as a
//! detail of its entity, as in the BALP record: the binding outcome, the
//! party and its groups, the actor chain, the representation, the agreement
//! with its legal ground, the correlation id with its source, the point of
//! care, the facility type, the home community id, the permissions, the
//! authentication method and `on-behalf-of`. The person id BALP writes as
//! `otherId` typed `PRN` and a policy the record names have none, and a
//! security label has none either: `ParticipantObjectSensitivity` holds one
//! token, where a record's labels are codings.
//!
//! A code no system names is written in the code system `local`, since
//! PS3.15 A.5.1 requires a `codeSystemName` (no specification governs the
//! name: our own design). Every value is escaped as `ihe_atna::escape` does
//! when the message is written.

use ihe_atna::atna::codes;
use ihe_atna::atna::message::{
    AccessPoint, ActiveParticipant, AuditMessage, AuditSource, CodedValue, EventIdentification,
    ParticipantObject,
};
use ihe_atna::balp::{Coded, Described, Direction, Entity, Exchange, NetworkAddress, What};
use secrecy::{ExposeSecret as _, SecretString};

use crate::balp::{ObjectIds, exchange_with};
use crate::record::{AccessRecord, Coded as RecordCoded, InvalidRecord, NameId};

/// The code system name of a code no system names.
pub const LOCAL_CODES: &str = "local";

/// The role of the service consumer NCP (eHDSI Audit Trail Profile 10.1.0
/// §2.3.40).
pub const SERVICE_CONSUMER: (&str, &str, &str) = (
    "ServiceConsumer",
    "eHealth DSI",
    "eHealth DSI Service Consumer",
);

/// The role of the service provider NCP (eHDSI Audit Trail Profile 10.1.0
/// §2.3.40).
pub const SERVICE_PROVIDER: (&str, &str, &str) = (
    "ServiceProvider",
    "eHealth DSI",
    "eHealth DSI Service Provider",
);

/// The code system of the FHIR R4 `audit-entity-type` codes, which are the
/// DICOM `ParticipantObjectTypeCode` values.
const ENTITY_TYPES: &str = "http://terminology.hl7.org/CodeSystem/audit-entity-type";

/// The code system of the FHIR R4 `object-role` codes, which are the DICOM
/// `ParticipantObjectTypeCodeRole` values.
const OBJECT_ROLES: &str = "http://terminology.hl7.org/CodeSystem/object-role";

/// The DICOM `ParticipantObjectTypeCode` values (PS3.15 A.5.1).
const TYPE_CODES: [&str; 4] = ["1", "2", "3", "4"];

/// The DICOM `ParticipantObjectTypeCodeRole` values (PS3.15 A.5.1).
const ROLE_CODES: [&str; 24] = [
    "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16", "17",
    "18", "19", "20", "21", "22", "23", "24",
];

/// The detail type of an entity's `description`.
const DESCRIPTION: &str = "description";

/// The detail type of the namespace an entity's identifier was issued in.
const IDENTIFIER_SYSTEM: &str = "identifier-system";

/// Why an access record could not be written as an audit message.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AtnaError {
    /// The record breaks a rule a sink stores records under.
    #[error("the access record is not one a sink stores")]
    Invalid(#[from] InvalidRecord),
    /// The exchange holds an entity this rendering has no form for.
    #[error("the access record holds an entity the audit message has no form for")]
    Unsupported,
}

/// The audit message `record` is written as, the recording system named by
/// `endpoint`, the address its requests arrive at, and the message's audit
/// source `source`, with its ids taken as [`ObjectIds::References`].
///
/// # Errors
///
/// An [`AtnaError`] when the record breaks a rule a sink stores records
/// under, or holds an entity the message has no form for.
pub fn audit_message(
    record: &AccessRecord,
    endpoint: &NetworkAddress,
    source: &AuditSource,
) -> Result<AuditMessage, AtnaError> {
    audit_message_with(record, endpoint, source, &ObjectIds::default())
}

/// The audit message `record` is written as, as [`audit_message`] writes it,
/// whose ids `ids` declares.
///
/// # Errors
///
/// An [`AtnaError`] when the record breaks a rule a sink stores records
/// under, or holds an entity the message has no form for.
pub fn audit_message_with(
    record: &AccessRecord,
    endpoint: &NetworkAddress,
    source: &AuditSource,
    ids: &ObjectIds,
) -> Result<AuditMessage, AtnaError> {
    record.validate()?;
    let exchange = exchange_with(record, endpoint, ids);
    let kind = &exchange.kind;
    let event = EventIdentification {
        event_id: coded(&kind.event_type),
        action: kind.action,
        date_time: exchange.recorded,
        outcome: exchange.outcome.code(),
        type_codes: kind.subtypes.iter().map(coded).collect(),
        outcome_description: exchange.outcome_description.clone(),
        purposes_of_use: record
            .accessor
            .purposes
            .iter()
            .map(|purpose| local(purpose.system.as_deref(), &purpose.code))
            .collect(),
    };
    let objects = exchange
        .entities
        .iter()
        .map(|entity| object(entity, record))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(AuditMessage {
        event,
        participants: participants(record, &exchange),
        source: source.clone(),
        objects,
    })
}

/// The participants of `record`: the relayed requestor and the contact
/// points, the person the token names, the client, the recording system and
/// the provider the person acts for.
fn participants(record: &AccessRecord, exchange: &Exchange) -> Vec<ActiveParticipant> {
    let accessor = &record.accessor;
    let mut participants = Vec::new();
    if let Some(relayed) = &accessor.relayed {
        let parts = relayed.as_parts();
        participants.push(match &parts.professional {
            Some(professional) => ActiveParticipant {
                user_id: professional
                    .name_id
                    .as_ref()
                    .map_or_else(|| professional.identifier.clone(), NameId::user_id),
                alternative_user_id: Some(professional.name.clone()),
                user_name: None,
                user_is_requestor: true,
                role_id_codes: professional.roles.iter().map(record_coded).collect(),
                access_point: None,
            },
            None => ActiveParticipant {
                user_id: parts.provider.identifier.clone(),
                alternative_user_id: None,
                user_name: parts.provider.name.clone(),
                user_is_requestor: true,
                role_id_codes: Vec::new(),
                access_point: None,
            },
        });
        for (gateway, requestor, role) in [
            (&parts.gateways.consumer, true, SERVICE_CONSUMER),
            (&parts.gateways.provider, false, SERVICE_PROVIDER),
        ] {
            participants.extend(gateway.iter().map(|cn| ActiveParticipant {
                user_id: cn.clone(),
                alternative_user_id: None,
                user_name: None,
                user_is_requestor: requestor,
                role_id_codes: vec![CodedValue::of(role)],
                access_point: None,
            }));
        }
    }
    if let Some(user) = exchange.on_behalf.user() {
        let mut roles: Vec<CodedValue> = accessor
            .acting
            .map(|acting| local(None, acting.code()))
            .into_iter()
            .collect();
        roles.extend(accessor.roles.iter().map(record_coded));
        participants.push(ActiveParticipant {
            user_id: accessor.subject.clone(),
            alternative_user_id: accessor.alt_id.clone(),
            user_name: Some(user.atna_user_name()),
            user_is_requestor: true,
            role_id_codes: roles,
            access_point: None,
        });
    }
    if let Direction::Received { client, endpoint } = &exchange.direction {
        participants.push(ActiveParticipant {
            user_id: client.who.clone(),
            alternative_user_id: None,
            user_name: None,
            user_is_requestor: false,
            role_id_codes: vec![coded(&exchange.kind.client)],
            access_point: Some(access_point(&client.network)),
        });
        participants.push(ActiveParticipant {
            user_id: endpoint.address(),
            alternative_user_id: None,
            user_name: None,
            user_is_requestor: false,
            role_id_codes: vec![coded(&exchange.kind.server)],
            access_point: Some(access_point(endpoint)),
        });
    }
    participants.extend(accessor.provider.iter().map(|provider| ActiveParticipant {
        user_id: provider.clone(),
        alternative_user_id: None,
        user_name: None,
        user_is_requestor: false,
        role_id_codes: Vec::new(),
        access_point: None,
    }));
    participants
}

/// The participant object of `entity` (`RESTful` ATNA Rev. 3.6 Table
/// 3.20.4.2.2.1-1).
fn object(entity: &Entity, record: &AccessRecord) -> Result<ParticipantObject, AtnaError> {
    let plain = |id: SecretString, id_type_code: CodedValue| ParticipantObject {
        id,
        type_code: None,
        type_code_role: None,
        data_life_cycle: None,
        id_type_code,
        name: None,
        query: None,
        details: Vec::new(),
    };
    Ok(match entity {
        // NOTE: ITI TF-2 §3.20.7.1.3 and RFC 3881 §5.5.4: a query is identified by its search
        // criteria, here the operation that ran it.
        Entity::Query(query) => ParticipantObject {
            type_code: Some("2"),
            type_code_role: Some("24"),
            query: Some(query.clone()),
            ..plain(
                SecretString::from(record.request.operation.clone()),
                CodedValue::of(codes::OBJECT_ID_SEARCH_CRITERIA),
            )
        },
        Entity::Patient {
            system,
            value,
            namespace,
        } => ParticipantObject {
            type_code: Some("1"),
            type_code_role: Some("1"),
            // NOTE: ITI TF-2 §3.20.7.1.3 and NCPeH Components Specifications 10.0.0 §4.4.7 write
            // `RFC-3881` for code 2, where the Audit Trail Profile 10.1.0 tables write `DCM`.
            ..plain(
                SecretString::from(cx(
                    value.expose_secret(),
                    system.as_deref(),
                    namespace.as_deref(),
                )),
                CodedValue::of(codes::OBJECT_ID_PATIENT_NUMBER),
            )
        },
        Entity::PatientReference(reference) => ParticipantObject {
            type_code: Some("1"),
            type_code_role: Some("1"),
            ..plain(reference.clone(), CodedValue::of(codes::OBJECT_ID_URI))
        },
        Entity::Described(described) => described_object(described)?,
        _ => return Err(AtnaError::Unsupported),
    })
}

/// The participant object of a described entity.
fn described_object(described: &Described) -> Result<ParticipantObject, AtnaError> {
    let mut details = Vec::new();
    let id = match &described.what {
        Some(What::Identifier { system, value }) => {
            details.extend(system.iter().map(|system| {
                (
                    IDENTIFIER_SYSTEM.to_owned(),
                    SecretString::from(system.clone()),
                )
            }));
            value.clone()
        }
        Some(What::Reference { reference, .. }) => reference.clone(),
        None => described
            .name
            .clone()
            .unwrap_or_else(|| described.kind.code.to_string()),
        Some(_) => return Err(AtnaError::Unsupported),
    };
    details.extend(
        described
            .description
            .iter()
            .map(|text| (DESCRIPTION.to_owned(), SecretString::from(text.clone()))),
    );
    details.extend(described.details.iter().map(|detail| {
        (
            detail.kind.clone(),
            SecretString::from(detail.value.clone()),
        )
    }));
    Ok(ParticipantObject {
        id: SecretString::from(id),
        type_code: dicom_code(&described.kind, ENTITY_TYPES, &TYPE_CODES),
        type_code_role: described
            .role
            .as_ref()
            .and_then(|role| dicom_code(role, OBJECT_ROLES, &ROLE_CODES)),
        data_life_cycle: None,
        id_type_code: coded(&described.kind),
        name: described.name.clone().map(SecretString::from),
        query: None,
        details,
    })
}

/// The DICOM code of `code` when it is one of `codes` in `system`, whose
/// codes FHIR R4 takes from DICOM.
fn dicom_code(code: &Coded, system: &str, codes: &[&'static str]) -> Option<&'static str> {
    (code.system == system)
        .then(|| {
            codes
                .iter()
                .copied()
                .find(|candidate| *candidate == code.code)
        })
        .flatten()
}

/// The HL7 v2 CX form of a patient identifier: the id, with the assigning
/// authority as its fourth component when the record names one, as an ISO
/// OID, a URI or a namespace id.
fn cx(value: &str, system: Option<&str>, namespace: Option<&str>) -> String {
    let id = hl7_escaped(value);
    let authority = match (system, namespace) {
        (Some(system), _) => match system.strip_prefix("urn:oid:") {
            Some(oid) => format!("&{}&ISO", hl7_escaped(oid)),
            None => format!("&{}&URI", hl7_escaped(system)),
        },
        (None, Some(namespace)) => hl7_escaped(namespace),
        (None, None) => return id,
    };
    format!("{id}^^^{authority}")
}

/// `value` with each HL7 v2 delimiter written as its escape sequence (HL7
/// Version 2.5 Chapter 2 §2.7.4): `\F\`, `\S\`, `\T\`, `\R\` and `\E\`.
fn hl7_escaped(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '|' => escaped.push_str(r"\F\"),
            '^' => escaped.push_str(r"\S\"),
            '&' => escaped.push_str(r"\T\"),
            '~' => escaped.push_str(r"\R\"),
            '\\' => escaped.push_str(r"\E\"),
            other => escaped.push(other),
        }
    }
    escaped
}

/// The network access point of `address`; FHIR R4 `network-type` takes its
/// codes from the DICOM `NetworkAccessPointTypeCode`.
fn access_point(address: &NetworkAddress) -> AccessPoint {
    AccessPoint {
        type_code: address.type_code(),
        id: address.address(),
    }
}

/// A BALP coding as a DICOM coded value: its code, its system as the code
/// system name and its display as the original text.
fn coded(code: &Coded) -> CodedValue {
    CodedValue {
        code: code.code.to_string(),
        system_name: code.system.to_string(),
        original_text: code.display.to_string(),
    }
}

/// A record's coded value as a DICOM coded value.
fn record_coded(code: &RecordCoded) -> CodedValue {
    local(code.system.as_deref(), &code.code)
}

/// `code` in `system`, or in [`LOCAL_CODES`] when no system names it.
fn local(system: Option<&str>, code: &str) -> CodedValue {
    CodedValue {
        code: code.to_owned(),
        system_name: system.unwrap_or(LOCAL_CODES).to_owned(),
        original_text: code.to_owned(),
    }
}
