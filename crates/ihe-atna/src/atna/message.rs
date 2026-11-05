// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The DICOM PS3.15 Annex A.5 `AuditMessage`, the XML an ITI-20 syslog
//! message carries (ITI TF-2 §3.20.4.1.2).
//!
//! The model holds what an IHE transaction's audit table fills: the event,
//! the active participants, the audit source and the participant objects. A
//! participant object's id, query and detail values may name a patient, so
//! they are held as secrets, and the written message is a secret too.

use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use jiff::Timestamp;
use quick_xml::Writer;
use quick_xml::events::{BytesDecl, BytesText, Event};
use secrecy::{ExposeSecret, SecretSlice, SecretString};

use crate::escape;
use crate::redact::REDACTED;

/// A coded value, written `EV(code, codeSystemName, originalText)` in the
/// IHE audit tables: the attributes `csd-code`, `codeSystemName` and
/// `originalText` of the DICOM `CodedValueType`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodedValue {
    /// `csd-code`.
    pub code: String,
    /// `codeSystemName`.
    pub system_name: String,
    /// `originalText`.
    pub original_text: String,
}

impl CodedValue {
    /// The coded value `EV(code, system_name, original_text)`.
    #[must_use]
    pub fn new(code: &str, system_name: &str, original_text: &str) -> Self {
        Self {
            code: code.to_owned(),
            system_name: system_name.to_owned(),
            original_text: original_text.to_owned(),
        }
    }

    /// The coded value of a `(code, codeSystemName, originalText)` triple.
    #[must_use]
    pub fn of((code, system_name, original_text): (&str, &str, &str)) -> Self {
        Self::new(code, system_name, original_text)
    }
}

/// `EventIdentification`: what happened, when, and how it ended.
#[derive(Debug, Clone)]
pub struct EventIdentification {
    /// `EventID`.
    pub event_id: CodedValue,
    /// `EventActionCode`: `C`, `R`, `U`, `D` or `E`.
    pub action: &'static str,
    /// `EventDateTime`.
    pub date_time: Timestamp,
    /// `EventOutcomeIndicator`: `0`, `4`, `8` or `12`.
    pub outcome: &'static str,
    /// `EventTypeCode`, in order.
    pub type_codes: Vec<CodedValue>,
    /// `EventOutcomeDescription`, free text on how the event ended, when
    /// the system gives one (DICOM PS3.15 A.5.1, optional).
    pub outcome_description: Option<String>,
    /// `PurposeOfUse`, in order: the purposes of use the event was made
    /// for, each a `CodedValueType` (ITI TF-2 §3.20.7.1.2), written after
    /// the elements PS3.15 A.5.1 defines.
    pub purposes_of_use: Vec<CodedValue>,
}

/// A `NetworkAccessPointID` with its `NetworkAccessPointTypeCode`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessPoint {
    /// `NetworkAccessPointTypeCode`: `1` for a machine name, `2` for an IP
    /// address.
    pub type_code: &'static str,
    /// `NetworkAccessPointID`.
    pub id: String,
}

/// `ActiveParticipant`: a user, process or system that took part.
///
/// A participant may be a person, so `Debug` shows neither its `UserID` nor
/// its `UserName`.
///
/// It names no policy the participant acted under: DICOM PS3.15 A.5.1.1
/// defines no such element of `ActiveParticipant`. `RESTful` ATNA Rev. 3.6
/// Tables 3.20.4.2.2.1-1 and 3.81.4.2.2.1-1 map `AuditEvent.agent.policy` to
/// a `ParticipantRoleIDCode` that schema does not hold, so a policy, such as
/// a data permit, travels in the FHIR `AuditEvent` alone.
#[derive(Clone)]
pub struct ActiveParticipant {
    /// `UserID`.
    pub user_id: String,
    /// `AlternativeUserID`, when the table fills it.
    pub alternative_user_id: Option<String>,
    /// `UserName`, when the table or a grouped profile fills it.
    pub user_name: Option<String>,
    /// `UserIsRequestor`.
    pub user_is_requestor: bool,
    /// `RoleIDCode`, in order.
    pub role_id_codes: Vec<CodedValue>,
    /// The network access point, when the participant has one.
    pub access_point: Option<AccessPoint>,
}

impl fmt::Debug for ActiveParticipant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ActiveParticipant")
            .field("user_id", &REDACTED)
            .field("alternative_user_id", &self.alternative_user_id)
            .field("user_name", &self.user_name.as_ref().map(|_| REDACTED))
            .field("user_is_requestor", &self.user_is_requestor)
            .field("role_id_codes", &self.role_id_codes)
            .field("access_point", &self.access_point)
            .finish()
    }
}

/// `AuditSourceIdentification`: the system that wrote the message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditSource {
    /// `AuditSourceID`.
    pub id: String,
    /// `AuditEnterpriseSiteID`, when the deployment names one.
    pub enterprise_site: Option<String>,
    /// `AuditSourceTypeCode`, in order, each optional and none by default:
    /// the IHE audit tables leave it not specialized (ITI TF-2 §3.20.7).
    pub type_codes: Vec<SourceType>,
}

/// One `AuditSourceTypeCode` (DICOM PS3.15 A.5.1).
///
/// A `csd-code` of `1` to `9` alone, such as
/// [`codes::SOURCE_APPLICATION_SERVER`](super::codes::SOURCE_APPLICATION_SERVER),
/// or another code with the code system that defines it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceType {
    /// `csd-code`.
    pub code: String,
    /// `codeSystemName`, which PS3.15 A.5.1 asks for a code other than `1`
    /// to `9`.
    pub system_name: Option<String>,
    /// `originalText`.
    pub original_text: Option<String>,
}

impl SourceType {
    /// The source type `code` of the PS3.15 A.5.1 list, written alone.
    #[must_use]
    pub fn of(code: &str) -> Self {
        Self {
            code: code.to_owned(),
            system_name: None,
            original_text: None,
        }
    }
}

/// `ParticipantObjectIdentification`: an object the event concerned.
#[derive(Clone)]
pub struct ParticipantObject {
    /// `ParticipantObjectID`.
    pub id: SecretString,
    /// `ParticipantObjectTypeCode`, when the table fills it.
    pub type_code: Option<&'static str>,
    /// `ParticipantObjectTypeCodeRole`, when the table fills it.
    pub type_code_role: Option<&'static str>,
    /// `ParticipantObjectDataLifeCycle`, `1` to `15` (DICOM PS3.15 A.5.1),
    /// when the table or the caller fills it.
    pub data_life_cycle: Option<&'static str>,
    /// `ParticipantObjectIDTypeCode`.
    pub id_type_code: CodedValue,
    /// `ParticipantObjectName`, written when no query is given: DICOM
    /// PS3.15 A.5.1 holds either a name or a query, and `RESTful` ATNA Rev.
    /// 3.6 Table 3.20.4.2.2.1-1 maps it from `entity.name`.
    pub name: Option<SecretString>,
    /// `ParticipantObjectQuery`, written base64-encoded, in place of the
    /// name.
    pub query: Option<SecretString>,
    /// `ParticipantObjectDetail` pairs: the type, and the value written
    /// base64-encoded.
    pub details: Vec<(String, SecretString)>,
}

impl fmt::Debug for ParticipantObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParticipantObject")
            .field("id", &REDACTED)
            .field("type_code", &self.type_code)
            .field("type_code_role", &self.type_code_role)
            .field("data_life_cycle", &self.data_life_cycle)
            .field("id_type_code", &self.id_type_code)
            .field("name", &self.name.as_ref().map(|_| REDACTED))
            .field("query", &self.query.as_ref().map(|_| REDACTED))
            .field(
                "details",
                &self
                    .details
                    .iter()
                    .map(|(kind, _)| (kind.as_str(), REDACTED))
                    .collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// One DICOM PS3.15 Annex A.5 `AuditMessage`.
#[derive(Debug, Clone)]
pub struct AuditMessage {
    /// `EventIdentification`.
    pub event: EventIdentification,
    /// `ActiveParticipant`, at least one, in order.
    pub participants: Vec<ActiveParticipant>,
    /// `AuditSourceIdentification`.
    pub source: AuditSource,
    /// `ParticipantObjectIdentification`, in order.
    pub objects: Vec<ParticipantObject>,
}

/// Why a message could not be written.
#[derive(Debug, thiserror::Error)]
#[error("the audit message could not be written")]
pub struct MessageError(#[source] std::io::Error);

impl AuditMessage {
    /// The message as UTF-8 XML, with no byte order mark (ITI TF-2
    /// §3.20.4.1.2), in the element order of the PS3.15 A.5.1 schema.
    ///
    /// # Errors
    ///
    /// A [`MessageError`] when the writer fails, which writing to memory does
    /// not.
    pub fn to_xml(&self) -> Result<SecretSlice<u8>, MessageError> {
        let mut writer = Writer::new(Vec::new());
        writer
            .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
            .map_err(MessageError)?;
        writer
            .create_element("AuditMessage")
            .write_inner_content(|writer| {
                self.write_event(writer)?;
                for participant in &self.participants {
                    write_participant(writer, participant)?;
                }
                self.write_source(writer)?;
                for object in &self.objects {
                    write_object(writer, object)?;
                }
                Ok(())
            })
            .map_err(MessageError)?;
        Ok(SecretSlice::from(writer.into_inner()))
    }

    fn write_event(&self, writer: &mut Writer<Vec<u8>>) -> std::io::Result<()> {
        let event = &self.event;
        let date_time = event.date_time.to_string();
        writer
            .create_element("EventIdentification")
            .with_attributes([
                ("EventActionCode", escape::untrusted(event.action).as_ref()),
                ("EventDateTime", date_time.as_str()),
                (
                    "EventOutcomeIndicator",
                    escape::untrusted(event.outcome).as_ref(),
                ),
            ])
            .write_inner_content(|writer| {
                write_coded(writer, "EventID", &event.event_id)?;
                for code in &event.type_codes {
                    write_coded(writer, "EventTypeCode", code)?;
                }
                // NOTE: DICOM PS3.15 A.5.1: EventOutcomeDescription follows EventTypeCode.
                if let Some(description) = &event.outcome_description {
                    writer
                        .create_element("EventOutcomeDescription")
                        .write_text_content(BytesText::new(&escape::untrusted(description)))?;
                }
                // NOTE: ITI TF-2 §3.20.7.1.2 adds PurposeOfUse, which PS3.15 A.5.1 does not
                // place, so it follows the elements the schema defines.
                for purpose in &event.purposes_of_use {
                    write_coded(writer, "PurposeOfUse", purpose)?;
                }
                Ok(())
            })?;
        Ok(())
    }

    fn write_source(&self, writer: &mut Writer<Vec<u8>>) -> std::io::Result<()> {
        let source = &self.source;
        let mut attributes = Vec::new();
        if let Some(site) = &source.enterprise_site {
            attributes.push(("AuditEnterpriseSiteID", escape::untrusted(site)));
        }
        attributes.push(("AuditSourceID", escape::untrusted(&source.id)));
        let attributes: Vec<(&str, &str)> = attributes
            .iter()
            .map(|(name, value)| (*name, value.as_ref()))
            .collect();
        let element = writer
            .create_element("AuditSourceIdentification")
            .with_attributes(attributes);
        if source.type_codes.is_empty() {
            element.write_empty()?;
            return Ok(());
        }
        element.write_inner_content(|writer| {
            for kind in &source.type_codes {
                let mut attributes = vec![("csd-code", escape::untrusted(&kind.code))];
                if let Some(system) = &kind.system_name {
                    attributes.push(("codeSystemName", escape::untrusted(system)));
                }
                if let Some(text) = &kind.original_text {
                    attributes.push(("originalText", escape::untrusted(text)));
                }
                writer
                    .create_element("AuditSourceTypeCode")
                    .with_attributes(
                        attributes
                            .iter()
                            .map(|(name, value)| (*name, value.as_ref())),
                    )
                    .write_empty()?;
            }
            Ok(())
        })?;
        Ok(())
    }
}

fn write_coded(
    writer: &mut Writer<Vec<u8>>,
    element: &str,
    code: &CodedValue,
) -> std::io::Result<()> {
    writer
        .create_element(element)
        .with_attributes([
            ("csd-code", escape::untrusted(&code.code).as_ref()),
            (
                "codeSystemName",
                escape::untrusted(&code.system_name).as_ref(),
            ),
            (
                "originalText",
                escape::untrusted(&code.original_text).as_ref(),
            ),
        ])
        .write_empty()?;
    Ok(())
}

fn write_participant(
    writer: &mut Writer<Vec<u8>>,
    participant: &ActiveParticipant,
) -> std::io::Result<()> {
    let mut attributes = vec![("UserID", escape::untrusted(&participant.user_id))];
    if let Some(alternative) = &participant.alternative_user_id {
        attributes.push(("AlternativeUserID", escape::untrusted(alternative)));
    }
    if let Some(name) = &participant.user_name {
        attributes.push(("UserName", escape::untrusted(name)));
    }
    attributes.push((
        "UserIsRequestor",
        std::borrow::Cow::Borrowed(if participant.user_is_requestor {
            "true"
        } else {
            "false"
        }),
    ));
    if let Some(point) = &participant.access_point {
        attributes.push(("NetworkAccessPointID", escape::untrusted(&point.id)));
        attributes.push((
            "NetworkAccessPointTypeCode",
            escape::untrusted(point.type_code),
        ));
    }
    writer
        .create_element("ActiveParticipant")
        .with_attributes(
            attributes
                .iter()
                .map(|(name, value)| (*name, value.as_ref())),
        )
        .write_inner_content(|writer| {
            for code in &participant.role_id_codes {
                write_coded(writer, "RoleIDCode", code)?;
            }
            Ok(())
        })?;
    Ok(())
}

fn write_object(writer: &mut Writer<Vec<u8>>, object: &ParticipantObject) -> std::io::Result<()> {
    let mut attributes = vec![(
        "ParticipantObjectID",
        escape::untrusted(object.id.expose_secret()),
    )];
    for (name, value) in [
        ("ParticipantObjectTypeCode", object.type_code),
        ("ParticipantObjectTypeCodeRole", object.type_code_role),
        ("ParticipantObjectDataLifeCycle", object.data_life_cycle),
    ] {
        if let Some(value) = value {
            attributes.push((name, escape::untrusted(value)));
        }
    }
    let details: Vec<(std::borrow::Cow<'_, str>, String)> = object
        .details
        .iter()
        .map(|(kind, value)| {
            (
                escape::untrusted(kind),
                STANDARD.encode(value.expose_secret()),
            )
        })
        .collect();
    writer
        .create_element("ParticipantObjectIdentification")
        .with_attributes(
            attributes
                .iter()
                .map(|(name, value)| (*name, value.as_ref())),
        )
        .write_inner_content(|writer| {
            write_coded(writer, "ParticipantObjectIDTypeCode", &object.id_type_code)?;
            // NOTE: DICOM PS3.15 A.5.1 holds a ParticipantObjectName or a ParticipantObjectQuery,
            // and RESTful ATNA Rev. 3.6 Table 3.20.4.2.2.1-1 Note 2 uses one of the two.
            if let Some(query) = &object.query {
                writer
                    .create_element("ParticipantObjectQuery")
                    .write_text_content(BytesText::new(&STANDARD.encode(query.expose_secret())))?;
            } else if let Some(name) = &object.name {
                writer
                    .create_element("ParticipantObjectName")
                    .write_text_content(BytesText::new(&escape::untrusted(name.expose_secret())))?;
            }
            for (kind, value) in &details {
                writer
                    .create_element("ParticipantObjectDetail")
                    .with_attributes([("type", kind.as_ref()), ("value", value.as_str())])
                    .write_empty()?;
            }
            Ok(())
        })?;
    Ok(())
}
