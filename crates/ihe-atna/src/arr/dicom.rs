// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! A DICOM PS3.15 A.5 audit message read into a FHIR R4 `AuditEvent`.
//!
//! The mapping is the normative one of `RESTful` ATNA Rev. 3.6 Table
//! 3.81.4.2.2.1-1, so a record received over syslog is searched and
//! answered by ITI-81 like one received over the FHIR Feed.
//!
//! Every element and attribute the table maps is carried. An element it
//! does not map, or a message that is no `AuditMessage`, is refused with a
//! [`DicomError`] that names the element and never a value, and the
//! repository keeps the message for an operator rather than drop part of
//! it. No error carries the parser's own, which may quote the message.

use fhir_types::r4::audit_event::{
    AuditEvent, AuditEventAgent, AuditEventAgentNetwork, AuditEventEntity, AuditEventEntityDetail,
    AuditEventEntityDetailValue, AuditEventSource,
};
use fhir_types::r4::codeable_concept::CodeableConcept;
use fhir_types::r4::coding::Coding;
use fhir_types::r4::identifier::Identifier;
use fhir_types::r4::primitives;
use fhir_types::r4::reference::Reference;
use quick_xml::events::{BytesRef, BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

/// The DICOM code system, `codeSystemName` `DCM`.
const DCM: &str = "http://dicom.nema.org/resources/ontology/DCM";

/// The code system of the IHE transaction codes, `codeSystemName`
/// `IHE Transactions` (`RESTful` ATNA §3.81.4.1.2.2, `subtype`).
const IHE_TRANSACTIONS: &str = "urn:ihe:event-type-code";

/// The FHIR R4 code systems of the DICOM values the table maps to a
/// `Coding` with no `codeSystemName`.
const ENTITY_TYPE: &str = "http://terminology.hl7.org/CodeSystem/audit-entity-type";
const OBJECT_ROLE: &str = "http://terminology.hl7.org/CodeSystem/object-role";
const LIFECYCLE: &str = "http://terminology.hl7.org/CodeSystem/dicom-audit-lifecycle";
const SOURCE_TYPE: &str = "http://terminology.hl7.org/CodeSystem/security-source-type";

/// Why a DICOM audit message cannot be read; no variant carries a value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DicomError {
    /// The message is no well-formed XML in UTF-8.
    #[error("the audit message is no well-formed XML")]
    Xml,
    /// The root element is not `AuditMessage`.
    #[error("the audit message has no AuditMessage root")]
    Root,
    /// An element or attribute DICOM PS3.15 A.5 requires is missing.
    #[error("the audit message has no {0}")]
    Missing(&'static str),
    /// An element Table 3.81.4.2.2.1-1 does not map.
    #[error(
        "the audit message holds the element {0}, which the RESTful ATNA mapping does not carry"
    )]
    Unmapped(String),
}

/// Reads the DICOM audit message `xml` as an `AuditEvent`.
///
/// # Errors
///
/// A [`DicomError`] when `xml` is no `AuditMessage`, lacks an element PS3.15
/// A.5 requires, or holds an element the mapping does not carry.
pub fn audit_event(xml: &[u8]) -> Result<AuditEvent, DicomError> {
    let xml = std::str::from_utf8(xml).map_err(|_quotes_the_message| DicomError::Xml)?;
    let mut reader = Reader::from_str(xml.strip_prefix('\u{feff}').unwrap_or(xml));
    let mut event = AuditEvent::default();
    let mut path: Vec<String> = Vec::new();
    let mut text = String::new();
    let mut identified = false;
    let mut source = false;
    loop {
        match reader
            .read_event()
            .map_err(|_quotes_the_message| DicomError::Xml)?
        {
            Event::Start(element) => {
                let name = name(&element);
                open(
                    &mut event,
                    &path,
                    &name,
                    &element,
                    &mut identified,
                    &mut source,
                )?;
                path.push(name);
                text.clear();
            }
            Event::Empty(element) => {
                let name = name(&element);
                open(
                    &mut event,
                    &path,
                    &name,
                    &element,
                    &mut identified,
                    &mut source,
                )?;
            }
            Event::End(_) => {
                let found = std::mem::take(&mut text);
                if !found.trim().is_empty() {
                    content(&mut event, &path, found)?;
                }
                path.pop();
            }
            Event::Text(part) => text.push_str(&part.xml_content(XmlVersion::Implicit1_0)),
            Event::CData(part) => text.push_str(&part.into_inner()),
            Event::GeneralRef(reference) => text.push(entity_char(&reference)?),
            Event::Eof => break,
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) | Event::DocType(_) => {}
        }
    }
    if !identified {
        return Err(DicomError::Missing("EventIdentification"));
    }
    if !source {
        return Err(DicomError::Missing("AuditSourceIdentification"));
    }
    Ok(event)
}

/// The local name of `element`.
fn name(element: &BytesStart<'_>) -> String {
    element.local_name().as_ref().to_owned()
}

/// The character of an entity reference in text: a character reference or
/// one of the five XML 1.0 §4.6 predefined entities.
fn entity_char(reference: &BytesRef<'_>) -> Result<char, DicomError> {
    if let Some(found) = reference
        .resolve_char_ref()
        .map_err(|_quotes_the_message| DicomError::Xml)?
    {
        return Ok(found);
    }
    match reference.xml_content(XmlVersion::Implicit1_0).as_ref() {
        "lt" => Ok('<'),
        "gt" => Ok('>'),
        "amp" => Ok('&'),
        "apos" => Ok('\''),
        "quot" => Ok('"'),
        _ => Err(DicomError::Xml),
    }
}

/// The value of the attribute `key` of `element`, unescaped.
fn attribute(element: &BytesStart<'_>, key: &str) -> Result<Option<String>, DicomError> {
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|_quotes_the_message| DicomError::Xml)?;
        if attribute.key.local_name().as_ref() == key {
            let value = attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|_quotes_the_message| DicomError::Xml)?
                .into_owned();
            return Ok(Some(value));
        }
    }
    Ok(None)
}

/// The FHIR system of a DICOM `codeSystemName`: the DICOM and IHE
/// transaction systems by their URIs, and a name that is a URI as itself.
// NOTE: no specification governs a name that is neither: our own design writes no system,
// since Coding.system is a uri (FHIR R4 datatypes §2.24.0.14).
fn system(name: Option<&str>) -> Option<String> {
    match name? {
        "DCM" => Some(DCM.to_owned()),
        "IHE Transactions" => Some(IHE_TRANSACTIONS.to_owned()),
        uri if uri.contains(':') && !uri.contains(char::is_whitespace) => Some(uri.to_owned()),
        _ => None,
    }
}

/// The `Coding` of a DICOM `CodedValueType` element.
fn coding(element: &BytesStart<'_>) -> Result<Coding, DicomError> {
    let code = attribute(element, "csd-code")?.ok_or(DicomError::Missing("csd-code"))?;
    let system_name = attribute(element, "codeSystemName")?;
    Ok(Coding {
        system: system(system_name.as_deref()).map(primitives::Uri::from),
        code: Some(primitives::Code::from(code)),
        display: attribute(element, "originalText")?.map(primitives::String::from),
        ..Coding::default()
    })
}

/// A `Coding` of `code` in `system`.
fn plain(system: &str, code: String) -> Coding {
    Coding {
        system: Some(primitives::Uri::from(system)),
        code: Some(primitives::Code::from(code)),
        ..Coding::default()
    }
}

/// The DICOM `RoleIDCode`s FHIR R4 `AuditEvent.agent.type` lists
/// (`ParticipationRoleType`, DCM 110150 to 110155), which Note 1 of Table
/// 3.81.4.2.2.1-1 maps to `agent.type`.
fn is_agent_type(coding: &Coding) -> bool {
    coding
        .system
        .as_ref()
        .and_then(|system| system.value.as_deref())
        == Some(DCM)
        && coding
            .code
            .as_ref()
            .and_then(|code| code.value.as_deref())
            .is_some_and(|code| ("110150"..="110155").contains(&code) && code.len() == 6)
}

/// Takes the opening of `name` under `path` into `event`.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per element of Table 3.81.4.2.2.1-1, read side by side with it"
)]
fn open(
    event: &mut AuditEvent,
    path: &[String],
    name: &str,
    element: &BytesStart<'_>,
    identified: &mut bool,
    source: &mut bool,
) -> Result<(), DicomError> {
    let parent = path.last().map(String::as_str);
    match (parent, name) {
        (None, "AuditMessage")
        | (Some("EventIdentification"), "EventOutcomeDescription")
        | (Some("ActiveParticipant"), "MediaIdentifier")
        | (
            Some("ParticipantObjectIdentification"),
            "ParticipantObjectName" | "ParticipantObjectQuery" | "ParticipantObjectDescription",
        ) => {}
        (None, _) => return Err(DicomError::Root),
        (Some("AuditMessage"), "EventIdentification") => {
            *identified = true;
            event.action = attribute(element, "EventActionCode")?.map(primitives::Code::from);
            event.recorded = primitives::Instant::from(
                attribute(element, "EventDateTime")?.ok_or(DicomError::Missing("EventDateTime"))?,
            );
            event.outcome =
                attribute(element, "EventOutcomeIndicator")?.map(primitives::Code::from);
        }
        (Some("EventIdentification"), "EventID") => event.r#type = coding(element)?,
        (Some("EventIdentification"), "EventTypeCode") => event.subtype.push(coding(element)?),
        (Some("EventIdentification"), "PurposeOfUse") => {
            event.purpose_of_event.push(CodeableConcept {
                coding: vec![coding(element)?],
                ..CodeableConcept::default()
            });
        }
        (Some("AuditMessage"), "ActiveParticipant") => event.agent.push(agent(element)?),
        (Some("ActiveParticipant"), "RoleIDCode") => {
            let code = coding(element)?;
            let agent = event
                .agent
                .last_mut()
                .ok_or(DicomError::Missing("ActiveParticipant"))?;
            if agent.r#type.is_none() && is_agent_type(&code) {
                agent.r#type = Some(CodeableConcept {
                    coding: vec![code],
                    ..CodeableConcept::default()
                });
            } else {
                agent.role.push(CodeableConcept {
                    coding: vec![code],
                    ..CodeableConcept::default()
                });
            }
        }
        (Some("MediaIdentifier"), "MediaType") => {
            let code = coding(element)?;
            event
                .agent
                .last_mut()
                .ok_or(DicomError::Missing("ActiveParticipant"))?
                .media = Some(code);
        }
        (Some("AuditMessage"), "AuditSourceIdentification") => {
            *source = true;
            event.source = AuditEventSource {
                site: attribute(element, "AuditEnterpriseSiteID")?.map(primitives::String::from),
                observer: Reference {
                    identifier: Some(Box::new(identifier(
                        attribute(element, "AuditSourceID")?
                            .ok_or(DicomError::Missing("AuditSourceID"))?,
                    ))),
                    ..Reference::default()
                },
                ..AuditEventSource::default()
            };
        }
        (Some("AuditSourceIdentification"), "AuditSourceTypeCode") => {
            let code = attribute(element, "csd-code")?.ok_or(DicomError::Missing("csd-code"))?;
            let system_name = attribute(element, "codeSystemName")?;
            // NOTE: DICOM PS3.15 A.5.1 writes the codes 1 to 9 alone, which FHIR R4 codes in
            // security-source-type.
            let system = system(system_name.as_deref()).or_else(|| {
                (system_name.is_none() && code.len() == 1 && ("1"..="9").contains(&code.as_str()))
                    .then(|| SOURCE_TYPE.to_owned())
            });
            event.source.r#type.push(Coding {
                system: system.map(primitives::Uri::from),
                code: Some(primitives::Code::from(code)),
                display: attribute(element, "originalText")?.map(primitives::String::from),
                ..Coding::default()
            });
        }
        (Some("AuditMessage"), "ParticipantObjectIdentification") => {
            event.entity.push(entity(element)?);
        }
        (Some("ParticipantObjectIdentification"), "ParticipantObjectIDTypeCode") => {
            let code = coding(element)?;
            let entity = last_entity(event)?;
            if let Some(what) = entity.what.as_mut()
                && let Some(identifier) = what.identifier.as_mut()
            {
                identifier.r#type = Some(CodeableConcept {
                    coding: vec![code],
                    ..CodeableConcept::default()
                });
            }
        }
        (Some("ParticipantObjectIdentification"), "ParticipantObjectDetail") => {
            let kind = attribute(element, "type")?.ok_or(DicomError::Missing("type"))?;
            let value = attribute(element, "value")?.ok_or(DicomError::Missing("value"))?;
            last_entity(event)?.detail.push(AuditEventEntityDetail {
                id: None,
                extension: Vec::new(),
                modifier_extension: Vec::new(),
                r#type: primitives::String::from(kind),
                value: AuditEventEntityDetailValue::Base64Binary(primitives::Base64Binary::from(
                    value,
                )),
            });
        }
        (_, other) => return Err(DicomError::Unmapped(other.to_owned())),
    }
    Ok(())
}

/// Takes the text of the element at the end of `path` into `event`.
fn content(event: &mut AuditEvent, path: &[String], text: String) -> Result<(), DicomError> {
    match path.last().map(String::as_str) {
        Some("EventOutcomeDescription") => {
            event.outcome_desc = Some(primitives::String::from(text));
        }
        Some("ParticipantObjectName") => {
            last_entity(event)?.name = Some(primitives::String::from(text));
        }
        Some("ParticipantObjectQuery") => {
            last_entity(event)?.query = Some(primitives::Base64Binary::from(text));
        }
        // NOTE: RESTful ATNA Table 3.81.4.2.2.1-1 Note 2 carries the description in the
        // auditevent extensions, which name no plain text: our own design keeps it as
        // entity.description.
        Some("ParticipantObjectDescription") => {
            last_entity(event)?.description = Some(primitives::String::from(text));
        }
        Some(other) => return Err(DicomError::Unmapped(other.to_owned())),
        None => return Err(DicomError::Root),
    }
    Ok(())
}

/// The agent of one `ActiveParticipant`.
fn agent(element: &BytesStart<'_>) -> Result<AuditEventAgent, DicomError> {
    let user = attribute(element, "UserID")?.ok_or(DicomError::Missing("UserID"))?;
    let requestor = attribute(element, "UserIsRequestor")?.as_deref() == Some("true");
    let address = attribute(element, "NetworkAccessPointID")?;
    let kind = attribute(element, "NetworkAccessPointTypeCode")?;
    let network = (address.is_some() || kind.is_some()).then(|| AuditEventAgentNetwork {
        address: address.map(primitives::String::from),
        r#type: kind.map(primitives::Code::from),
        ..AuditEventAgentNetwork::default()
    });
    Ok(AuditEventAgent {
        who: Some(Reference {
            identifier: Some(Box::new(identifier(user))),
            ..Reference::default()
        }),
        alt_id: attribute(element, "AlternativeUserID")?.map(primitives::String::from),
        name: attribute(element, "UserName")?.map(primitives::String::from),
        requestor: primitives::Boolean::from(requestor),
        network,
        ..AuditEventAgent::default()
    })
}

/// The entity of one `ParticipantObjectIdentification`.
fn entity(element: &BytesStart<'_>) -> Result<AuditEventEntity, DicomError> {
    let id = attribute(element, "ParticipantObjectID")?
        .ok_or(DicomError::Missing("ParticipantObjectID"))?;
    Ok(AuditEventEntity {
        what: Some(Reference {
            identifier: Some(Box::new(identifier(id))),
            ..Reference::default()
        }),
        r#type: attribute(element, "ParticipantObjectTypeCode")?
            .map(|code| plain(ENTITY_TYPE, code)),
        role: attribute(element, "ParticipantObjectTypeCodeRole")?
            .map(|code| plain(OBJECT_ROLE, code)),
        lifecycle: attribute(element, "ParticipantObjectDataLifeCycle")?
            .map(|code| plain(LIFECYCLE, code)),
        security_label: attribute(element, "ParticipantObjectSensitivity")?
            .map(|code| Coding {
                code: Some(primitives::Code::from(code)),
                ..Coding::default()
            })
            .into_iter()
            .collect(),
        ..AuditEventEntity::default()
    })
}

/// An identifier of `value` in no system.
fn identifier(value: String) -> Identifier {
    Identifier {
        value: Some(primitives::String::from(value)),
        ..Identifier::default()
    }
}

/// The entity last opened.
fn last_entity(event: &mut AuditEvent) -> Result<&mut AuditEventEntity, DicomError> {
    event
        .entity
        .last_mut()
        .ok_or(DicomError::Missing("ParticipantObjectIdentification"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MESSAGE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<AuditMessage>
  <EventIdentification EventActionCode="R" EventDateTime="2026-05-19T08:30:00Z" EventOutcomeIndicator="0">
    <EventID csd-code="110110" codeSystemName="DCM" originalText="Patient Record"/>
    <EventTypeCode csd-code="ITI-81" codeSystemName="IHE Transactions" originalText="Retrieve ATNA Audit Event"/>
    <EventOutcomeDescription>fine</EventOutcomeDescription>
    <PurposeOfUse csd-code="TREAT" codeSystemName="http://terminology.hl7.org/CodeSystem/v3-ActReason" originalText="treatment"/>
  </EventIdentification>
  <ActiveParticipant UserID="Qz7-user" AlternativeUserID="4242" UserName="Qz7-name" UserIsRequestor="true" NetworkAccessPointID="192.0.2.7" NetworkAccessPointTypeCode="2">
    <RoleIDCode csd-code="110153" codeSystemName="DCM" originalText="Source Role ID"/>
    <RoleIDCode csd-code="doctor" codeSystemName="local" originalText="doctor"/>
  </ActiveParticipant>
  <AuditSourceIdentification AuditEnterpriseSiteID="site-a" AuditSourceID="arr.example.org">
    <AuditSourceTypeCode csd-code="4"/>
  </AuditSourceIdentification>
  <ParticipantObjectIdentification ParticipantObjectID="Qz7-patient" ParticipantObjectTypeCode="1" ParticipantObjectTypeCodeRole="1" ParticipantObjectSensitivity="N">
    <ParticipantObjectIDTypeCode csd-code="2" codeSystemName="RFC-3881" originalText="Patient Number"/>
    <ParticipantObjectName>Qz7-patient-name</ParticipantObjectName>
    <ParticipantObjectDetail type="kind" value="YQ=="/>
  </ParticipantObjectIdentification>
</AuditMessage>"#;

    #[test]
    fn every_element_the_mapping_names_is_carried() {
        let event = audit_event(MESSAGE.as_bytes()).expect("it reads");
        assert_eq!(
            event.r#type,
            plain(DCM, "110110".to_owned()).with_display("Patient Record")
        );
        assert_eq!(
            event.subtype[0]
                .system
                .as_ref()
                .and_then(|s| s.value.as_deref()),
            Some(IHE_TRANSACTIONS)
        );
        assert_eq!(event.action.and_then(|a| a.value).as_deref(), Some("R"));
        assert_eq!(
            event.recorded.value.as_deref(),
            Some("2026-05-19T08:30:00Z")
        );
        assert_eq!(event.outcome.and_then(|o| o.value).as_deref(), Some("0"));
        assert_eq!(
            event.outcome_desc.and_then(|o| o.value).as_deref(),
            Some("fine")
        );
        assert_eq!(event.purpose_of_event.len(), 1);
        let agent = &event.agent[0];
        assert_eq!(agent.requestor.value, Some(true));
        assert_eq!(
            agent.r#type.as_ref().map(|t| t.coding[0].code.clone()),
            Some(Some(primitives::Code::from("110153"))),
            "Note 1: a known role is agent.type"
        );
        assert_eq!(agent.role.len(), 1, "Note 1: any other role is agent.role");
        assert_eq!(
            agent.alt_id.as_ref().and_then(|a| a.value.as_deref()),
            Some("4242")
        );
        assert_eq!(
            agent
                .network
                .as_ref()
                .and_then(|n| n.address.as_ref())
                .and_then(|a| a.value.as_deref()),
            Some("192.0.2.7")
        );
        assert_eq!(
            event.source.site.as_ref().and_then(|s| s.value.as_deref()),
            Some("site-a")
        );
        assert_eq!(
            event.source.r#type[0]
                .system
                .as_ref()
                .and_then(|s| s.value.as_deref()),
            Some(SOURCE_TYPE)
        );
        let entity = &event.entity[0];
        assert_eq!(
            entity.name.as_ref().and_then(|n| n.value.as_deref()),
            Some("Qz7-patient-name")
        );
        assert_eq!(entity.security_label.len(), 1);
        assert_eq!(entity.detail.len(), 1);
        let identifier = entity
            .what
            .as_ref()
            .and_then(|w| w.identifier.as_ref())
            .expect("what");
        assert_eq!(
            identifier.value.as_ref().and_then(|v| v.value.as_deref()),
            Some("Qz7-patient")
        );
        assert!(identifier.r#type.is_some(), "ParticipantObjectIDTypeCode");
    }

    #[test]
    fn an_element_the_mapping_does_not_carry_is_refused_by_name() {
        let message = MESSAGE.replace(
            "<ParticipantObjectName>",
            "<Accession Number=\"x\"/><ParticipantObjectName>",
        );
        assert_eq!(
            audit_event(message.as_bytes()),
            Err(DicomError::Unmapped("Accession".to_owned()))
        );
    }

    #[test]
    fn a_message_that_is_no_audit_message_is_refused() {
        assert_eq!(audit_event(b"<Other/>"), Err(DicomError::Root));
        assert_eq!(
            audit_event(b"<AuditMessage>"),
            Err(DicomError::Missing("EventIdentification"))
        );
        assert_eq!(audit_event(b"\xff"), Err(DicomError::Xml));
    }

    trait WithDisplay {
        fn with_display(self, display: &str) -> Self;
    }

    impl WithDisplay for Coding {
        fn with_display(mut self, display: &str) -> Self {
            self.display = Some(primitives::String::from(display));
            self
        }
    }
}
