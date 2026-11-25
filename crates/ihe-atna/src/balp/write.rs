// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! How an [`Exchange`] is written as its FHIR R4 `AuditEvent`: its agents,
//! the user an exchange was made for, and its entities.

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use fhir_types::r4::audit_event::{
    AuditEvent, AuditEventAgent, AuditEventAgentNetwork, AuditEventEntity, AuditEventEntityDetail,
    AuditEventEntityDetailValue, AuditEventSource,
};
use fhir_types::r4::codeable_concept::CodeableConcept;
use fhir_types::r4::coding::Coding;
use fhir_types::r4::extension::{Extension, ExtensionValue};
use fhir_types::r4::identifier::Identifier;
use fhir_types::r4::meta::Meta;
use fhir_types::r4::primitives;
use fhir_types::r4::reference::Reference;
use secrecy::{ExposeSecret, SecretSlice};

use super::{
    APPLICATION, AuditEventResource, AuditEventValue, AuditRecord, Coded, Described, Direction,
    Entity, Exchange, INFORMATION_RECIPIENT, NetworkAddress, OAUTH_COMPREHENSIVE, Observer,
    Outcome, PATIENT_NAMESPACE, PATIENT_ROLE, PERSON, Peer, QUERY_ROLE, RecordError, SYSTEM_OBJECT,
    SecurityLabel, What,
};
use crate::user::{Code, PurposeOfUse, User};

/// The BALP extension that carries an agent's assurance level.
const ASSURANCE_LEVEL_EXTENSION: &str =
    "https://profiles.ihe.net/ITI/BALP/StructureDefinition/ihe-assuranceLevel";

/// The BALP extension that carries another identifier of an agent.
const OTHER_ID_EXTENSION: &str =
    "https://profiles.ihe.net/ITI/BALP/StructureDefinition/ihe-otherId";

/// The HL7 v2 identifier types (table 0203), which BALP's
/// `OtherIdentifierTypesVS` draws `NPI` and `PRN` from.
const IDENTIFIER_TYPES: &str = "http://terminology.hl7.org/CodeSystem/v2-0203";

/// The identifier type of a national provider identifier.
const NATIONAL_PROVIDER_IDENTIFIER: &str = "NPI";

/// The identifier type of a person's id in a person directory, which BALP
/// 1.1.4 §3:5.7.5.4 slices `provider-id`.
const PERSON_IDENTIFIER: &str = "PRN";

impl Exchange {
    /// The `AuditEvent` of this exchange as `observer` records it.
    ///
    /// # Errors
    ///
    /// A [`RecordError`] when the resource cannot be written as JSON, which
    /// writing to memory does not do.
    pub fn audit_event(&self, observer: &Observer) -> Result<AuditRecord, RecordError> {
        self.audit_event_resource(observer).to_record()
    }

    /// The `AuditEvent` of this exchange as `observer` records it, with the
    /// record of the exchange refused, for
    /// [`Reservation::push_or_withdraw`](crate::atna::spool::reserve::Reservation::push_or_withdraw).
    ///
    /// The refusal is the same record with the outcome serious failure
    /// (`8`). It is kept as its typed resource and written only when the
    /// message is withdrawn; its length is known now, since only the outcome
    /// code differs, so the spool reserves the larger of the two in advance.
    ///
    /// # Errors
    ///
    /// A [`RecordError`] when the resource cannot be written as JSON, which
    /// writing to memory does not do.
    pub fn withdrawable(
        &self,
        observer: &Observer,
    ) -> Result<crate::atna::spool::withdraw::Withdrawable, RecordError> {
        let mut resource = self.resource(observer);
        let message = serde_json::to_vec(&resource).map_err(RecordError)?;
        let refused = Outcome::SeriousFailure.code();
        let size = message
            .len()
            .saturating_sub(self.outcome.code().len())
            .saturating_add(refused.len());
        resource.outcome = Some(escaped(refused));
        Ok(crate::atna::spool::withdraw::Withdrawable::typed(
            SecretSlice::from(message),
            AuditEventResource(resource),
            u64::try_from(size).unwrap_or(u64::MAX),
        ))
    }

    /// The `AuditEvent` of this exchange as `observer` records it, as the
    /// JSON value [`Exchange::audit_event`] writes as bytes.
    ///
    /// # Errors
    ///
    /// A [`RecordError`] when the resource cannot be written as JSON, which
    /// writing to memory does not do.
    pub fn audit_event_value(&self, observer: &Observer) -> Result<AuditEventValue, RecordError> {
        self.audit_event_resource(observer).to_value()
    }

    /// The `AuditEvent` of this exchange as `observer` records it, as the
    /// typed FHIR R4 resource [`Exchange::audit_event`] writes as bytes.
    ///
    /// Every string it carries has gone through
    /// [`escape::untrusted`](crate::escape::untrusted) already, so a caller
    /// that needs the resource typed, such as an ITI-81 searchset Bundle,
    /// takes it from here and never reads the written record back.
    #[must_use]
    pub fn audit_event_resource(&self, observer: &Observer) -> AuditEventResource {
        AuditEventResource(self.resource(observer))
    }

    fn resource(&self, observer: &Observer) -> AuditEvent {
        let own = |kind: &Coded, network: NetworkAddress| AuditEventAgent {
            r#type: Some(concept(kind)),
            who: Some(display(&observer.source_id)),
            requestor: primitives::Boolean::from(false),
            network: Some(network_of(&network)),
            ..AuditEventAgent::default()
        };
        let other = |kind: &Coded, peer: &Peer| AuditEventAgent {
            r#type: Some(concept(kind)),
            who: Some(display(&peer.who)),
            requestor: primitives::Boolean::from(false),
            network: Some(network_of(&peer.network)),
            ..AuditEventAgent::default()
        };
        // NOTE: BALP's client examples (AuditEvent-ex-auditBasicQueryGetClient and
        // the others) set requestor false on both system agents; only a user is one.
        let mut agent = match &self.direction {
            Direction::Sent { server } => vec![
                own(&self.kind.client, observer.host.clone()),
                other(&self.kind.server, server),
            ],
            Direction::Received { client, endpoint } => vec![
                other(&self.kind.client, client),
                own(&self.kind.server, endpoint.clone()),
            ],
        };
        let mut profiles: Vec<&str> = Vec::new();
        if !self.kind.profile.is_empty() {
            profiles.push(self.kind.profile);
        }
        if let Some(user) = self.on_behalf.user() {
            let (person, application) = user_agents(user);
            agent.push(person);
            // NOTE: BALP 1.1.4 IHE.BasicAudit.Delete admits one 110150 agent, its client, which
            // then names the user's application, so a second one is not added.
            let named_client = match application {
                Some(application) if self.kind.client != APPLICATION => {
                    agent.push(application);
                    true
                }
                _ => false,
            };
            // NOTE: BALP 1.1.4 OAUTHaccessTokenUse.Comprehensive fixes agent:oUser.policy 1..1
            // (the jti) and an 110150 oClient with who.identifier.value, so only such a record
            // claims it: one whose user names a token and no other policy.
            if user.token_id().is_some() && user.policies().is_empty() && named_client {
                profiles.push(OAUTH_COMPREHENSIVE);
            }
            agent.extend(organisation_agent(user));
        }
        let purpose_of_event =
            purposes_of_event(self.on_behalf.user().into_iter().flat_map(User::purposes));
        AuditEvent {
            meta: (!profiles.is_empty() || !self.security.is_empty()).then(|| Meta {
                profile: profiles.into_iter().map(escaped).collect(),
                security: self.security.iter().map(label).collect(),
                ..Meta::default()
            }),
            r#type: coding(&self.kind.event_type),
            subtype: self.kind.subtypes.iter().map(coding).collect(),
            action: Some(escaped(self.kind.action)),
            recorded: escaped(self.recorded.to_string()),
            outcome: Some(escaped(self.outcome.code())),
            outcome_desc: self.outcome_description.as_deref().map(escaped),
            purpose_of_event,
            agent,
            source: AuditEventSource {
                site: observer.site.as_deref().map(escaped),
                // NOTE: RESTful ATNA Table 3.81.4.2.2.1-1 maps AuditSourceId to source.observer, and
                // §3.81.4.1.2.2 matches source.identifier against source.observer.identifier.
                observer: Reference {
                    identifier: Some(Box::new(Identifier {
                        value: Some(escaped(observer.source_id.as_str())),
                        ..Identifier::default()
                    })),
                    ..display(&observer.source_id)
                },
                r#type: observer.source_types.iter().map(coding).collect(),
                ..AuditEventSource::default()
            },
            entity: self.entities.iter().map(entity).collect(),
            ..AuditEvent::default()
        }
    }
}

/// `value` as a record writes it: through
/// [`escape::untrusted`](crate::escape::untrusted), as every string of a
/// written `AuditEvent` is.
fn escaped<P: From<String>>(value: impl AsRef<str>) -> P {
    P::from(crate::escape::untrusted(value.as_ref()).into_owned())
}

impl AuditEventResource {
    /// The resource as the FHIR JSON bytes of an [`AuditRecord`].
    ///
    /// # Errors
    ///
    /// A [`RecordError`] when the resource cannot be written as JSON, which
    /// writing to memory does not do.
    pub fn to_record(&self) -> Result<AuditRecord, RecordError> {
        // NOTE: no specification governs the member order: our own design. fhir-types writes
        // every object's keys sorted, the order 0.18.0 wrote through a value, in every build.
        serde_json::to_vec(&self.0)
            .map(|json| AuditRecord(SecretSlice::from(json)))
            .map_err(RecordError)
    }

    /// The resource as the JSON value of an [`AuditEventValue`], the value
    /// [`AuditEventResource::to_record`] writes as bytes.
    ///
    /// # Errors
    ///
    /// A [`RecordError`] when the resource cannot be written as JSON, which
    /// writing to memory does not do.
    pub fn to_value(&self) -> Result<AuditEventValue, RecordError> {
        self.json().map(AuditEventValue)
    }

    #[expect(
        clippy::disallowed_types,
        reason = "the record leaves as a JSON value through AuditEventValue"
    )]
    fn json(&self) -> Result<serde_json::Value, RecordError> {
        serde_json::to_value(&self.0).map_err(RecordError)
    }
}

/// The purposes of use the user an exchange was made for names, each once,
/// in the order they are first named, for `AuditEvent.purposeOfEvent`.
///
/// They are compared as stated, before the escape writes them, so two the
/// escape writes alike are both kept. The user's agent keeps its own
/// `purposeOfUse` (BALP 1.1.4 `agent:user`).
// NOTE: RESTful ATNA Rev. 3.6 Table 3.81.4.2.2.1-1 maps agent.purposeOfUse to "NOT PRESENT" and
// purposeOfEvent to EventIdentification.purposeOfUse, so a DICOM-form repository keeps only these.
fn purposes_of_event<'a>(
    stated: impl IntoIterator<Item = &'a PurposeOfUse>,
) -> Vec<CodeableConcept> {
    let mut seen: Vec<&PurposeOfUse> = Vec::new();
    for stated in stated {
        if !seen.contains(&stated) {
            seen.push(stated);
        }
    }
    seen.into_iter().map(purpose).collect()
}

/// The agents that name `user` (BALP 1.1.4 §3:5.7.5.4): the `agent:user`
/// slice with the token's `iss` and `sub` as `who.identifier`, as
/// `requestor`, with its purposes of use and the token's `jti` as `policy`
/// followed by every other policy it acts under, and, when a token named a
/// client, the Application agent with its
/// `client_id` as `who.identifier.value`.
fn user_agents(user: &User) -> (AuditEventAgent, Option<AuditEventAgent>) {
    let identified = |system: Option<&str>, value: &str| Reference {
        identifier: Some(Box::new(Identifier {
            system: system.map(escaped),
            value: Some(escaped(value)),
            ..Identifier::default()
        })),
        ..Reference::default()
    };
    // NOTE: BALP 1.1.4 agent:user fixes requestor true and allows no network; the
    // client's agent follows ex-auditBasicReadOServer, requestor false.
    let person = AuditEventAgent {
        extension: user_extensions(user),
        r#type: Some(concept(&INFORMATION_RECIPIENT)),
        role: user.roles().iter().map(coded).collect(),
        who: Some(Reference {
            display: user.name().map(escaped),
            ..identified(user.issuer(), user.subject())
        }),
        alt_id: user.alt_id().map(escaped),
        requestor: primitives::Boolean::from(true),
        policy: user
            .token_id()
            .into_iter()
            .chain(user.policies().iter().map(String::as_str))
            .map(escaped)
            .collect(),
        purpose_of_use: user.purposes().iter().map(purpose).collect(),
        ..AuditEventAgent::default()
    };
    let application = user.client_id().map(|client_id| AuditEventAgent {
        r#type: Some(concept(&APPLICATION)),
        who: Some(identified(None, client_id)),
        requestor: primitives::Boolean::from(false),
        ..AuditEventAgent::default()
    });
    (person, application)
}

/// The agent of the organisation `user` acts for, when one is named: `who`
/// an `Organization` by its identifier, with no `type`, so it fills no slice
/// a BALP pattern bounds, and not the `requestor`.
fn organisation_agent(user: &User) -> Option<AuditEventAgent> {
    let organisation = user.organisation()?;
    Some(AuditEventAgent {
        who: Some(Reference {
            r#type: Some(escaped("Organization")),
            identifier: Some(Box::new(Identifier {
                value: Some(escaped(organisation)),
                ..Identifier::default()
            })),
            ..Reference::default()
        }),
        requestor: primitives::Boolean::from(false),
        ..AuditEventAgent::default()
    })
}

/// The extensions of `user`'s agent: the assurance level of their
/// authentication (`ihe-assuranceLevel`), their provider identifier
/// (`ihe-otherId`, typed `NPI`) and their person id (`ihe-otherId`, typed
/// `PRN`), as BALP 1.1.4 §3:5.7.5.4 maps the IHE IUA
/// `national_provider_identifier` and `person_id` and the
/// `AuditEvent-ex-auditPoke-SAML-Comp` example writes the first two.
fn user_extensions(user: &User) -> Vec<Extension> {
    let assurance = user.assurance().map(|level| Extension {
        url: ASSURANCE_LEVEL_EXTENSION.to_owned(),
        value: Some(ExtensionValue::CodeableConcept(Box::new(coded(level)))),
        ..Extension::default()
    });
    let provider = user.provider_identifier().map(|identifier| Extension {
        url: OTHER_ID_EXTENSION.to_owned(),
        value: Some(ExtensionValue::Identifier(Box::new(Identifier {
            r#type: Some(identifier_type(NATIONAL_PROVIDER_IDENTIFIER)),
            value: Some(escaped(identifier)),
            ..Identifier::default()
        }))),
        ..Extension::default()
    });
    let person = user.person_id().map(|person| Extension {
        url: OTHER_ID_EXTENSION.to_owned(),
        value: Some(ExtensionValue::Identifier(Box::new(Identifier {
            r#type: Some(identifier_type(PERSON_IDENTIFIER)),
            system: person.system.as_deref().map(escaped),
            value: Some(escaped(person.value.as_str())),
            ..Identifier::default()
        }))),
        ..Extension::default()
    });
    assurance
        .into_iter()
        .chain(provider)
        .chain(person)
        .collect()
}

/// The `ihe-otherId` type `code` of HL7 v2 table 0203.
fn identifier_type(code: &str) -> CodeableConcept {
    CodeableConcept {
        coding: vec![Coding {
            system: Some(escaped(IDENTIFIER_TYPES)),
            code: Some(escaped(code)),
            ..Coding::default()
        }],
        ..CodeableConcept::default()
    }
}

/// A [`Code`] as a `CodeableConcept`.
fn coded(code: &Code) -> CodeableConcept {
    CodeableConcept {
        coding: vec![Coding {
            system: code.system.as_deref().map(escaped),
            code: Some(escaped(code.code.as_str())),
            ..Coding::default()
        }],
        ..CodeableConcept::default()
    }
}

/// A purpose of use as `agent.purposeOfUse` and `purposeOfEvent` code it,
/// with the user's own words as the concept's `text`.
fn purpose(purpose: &PurposeOfUse) -> CodeableConcept {
    CodeableConcept {
        coding: vec![Coding {
            system: purpose.system.as_deref().map(escaped),
            code: Some(escaped(purpose.code.as_str())),
            ..Coding::default()
        }],
        text: purpose.text.as_deref().map(escaped),
        ..CodeableConcept::default()
    }
}

/// A [`Coded`] as a `Coding`.
fn coding(code: &Coded) -> Coding {
    Coding {
        system: Some(escaped(&code.system)),
        code: Some(escaped(&code.code)),
        display: Some(escaped(&code.display)),
        ..Coding::default()
    }
}

fn concept(code: &Coded) -> CodeableConcept {
    CodeableConcept {
        coding: vec![coding(code)],
        ..CodeableConcept::default()
    }
}

fn display(text: &str) -> Reference {
    Reference {
        display: Some(escaped(text)),
        ..Reference::default()
    }
}

fn network_of(address: &NetworkAddress) -> AuditEventAgentNetwork {
    AuditEventAgentNetwork {
        address: Some(escaped(address.address())),
        r#type: Some(escaped(address.type_code())),
        ..AuditEventAgentNetwork::default()
    }
}

fn entity(entity: &Entity) -> AuditEventEntity {
    match entity {
        Entity::Query(request) => AuditEventEntity {
            r#type: Some(coding(&SYSTEM_OBJECT)),
            role: Some(coding(&QUERY_ROLE)),
            query: Some(escaped(STANDARD.encode(request.expose_secret()))),
            ..AuditEventEntity::default()
        },
        Entity::Patient {
            system,
            value,
            namespace,
        } => AuditEventEntity {
            what: Some(Reference {
                identifier: Some(Box::new(Identifier {
                    system: system.as_deref().map(escaped),
                    value: Some(escaped(value.expose_secret())),
                    ..Identifier::default()
                })),
                ..Reference::default()
            }),
            r#type: Some(coding(&PERSON)),
            role: Some(coding(&PATIENT_ROLE)),
            detail: namespace
                .iter()
                .map(|namespace| string_detail(PATIENT_NAMESPACE, namespace))
                .collect(),
            ..AuditEventEntity::default()
        },
        Entity::PatientReference(reference) => AuditEventEntity {
            what: Some(Reference {
                reference: Some(escaped(reference.expose_secret())),
                ..Reference::default()
            }),
            r#type: Some(coding(&PERSON)),
            role: Some(coding(&PATIENT_ROLE)),
            ..AuditEventEntity::default()
        },
        Entity::Resource {
            reference,
            resource_type,
            kind,
            role,
            name,
            query,
        } => AuditEventEntity {
            what: Some(Reference {
                reference: reference.as_deref().map(escaped),
                r#type: Some(escaped(*resource_type)),
                ..Reference::default()
            }),
            r#type: Some(coding(kind)),
            role: role.as_ref().map(coding),
            name: name.map(escaped),
            query: query.as_deref().map(|text| escaped(STANDARD.encode(text))),
            ..AuditEventEntity::default()
        },
        Entity::Described(described) => described_entity(described),
    }
}

/// The `entity` a [`Described`] entity is written as.
fn described_entity(described: &Described) -> AuditEventEntity {
    let what = described.what.as_ref().map(|what| match what {
        What::Reference {
            reference,
            resource_type,
        } => Reference {
            reference: Some(escaped(reference.as_str())),
            r#type: resource_type.as_deref().map(escaped),
            ..Reference::default()
        },
        What::Identifier { system, value } => Reference {
            identifier: Some(Box::new(Identifier {
                system: system.as_deref().map(escaped),
                value: Some(escaped(value.as_str())),
                ..Identifier::default()
            })),
            ..Reference::default()
        },
    });
    AuditEventEntity {
        what,
        r#type: Some(coding(&described.kind)),
        role: described.role.as_ref().map(coding),
        name: described.name.as_deref().map(escaped),
        description: described.description.as_deref().map(escaped),
        detail: described
            .details
            .iter()
            .map(|detail| string_detail(&detail.kind, &detail.value))
            .collect(),
        security_label: described.security_labels.iter().map(label).collect(),
        ..AuditEventEntity::default()
    }
}

/// One `entity.detail` of type `kind` with the `valueString` `value`.
fn string_detail(kind: &str, value: &str) -> AuditEventEntityDetail {
    AuditEventEntityDetail {
        id: None,
        extension: Vec::new(),
        modifier_extension: Vec::new(),
        r#type: escaped(kind),
        value: AuditEventEntityDetailValue::String(escaped(value)),
    }
}

/// The `Coding` a security label is written as.
fn label(label: &SecurityLabel) -> Coding {
    Coding {
        system: Some(escaped(label.system.as_str())),
        version: label.version.as_deref().map(escaped),
        code: Some(escaped(label.code.as_str())),
        ..Coding::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{purpose, purposes_of_event};
    use crate::user::PurposeOfUse;

    fn coded(code: &str) -> PurposeOfUse {
        PurposeOfUse {
            system: Some("http://terminology.hl7.org/CodeSystem/v3-ActReason".to_owned()),
            code: code.to_owned(),
            text: None,
        }
    }

    fn carrying(codes: &[&str]) -> Vec<PurposeOfUse> {
        codes.iter().map(|code| coded(code)).collect()
    }

    #[test]
    fn a_purpose_named_twice_appears_once_in_first_seen_order() {
        let stated = [
            carrying(&["BTG", "TREAT"]),
            carrying(&[]),
            carrying(&["TREAT", "ETREAT"]),
        ];
        let expected: Vec<_> = ["BTG", "TREAT", "ETREAT"]
            .iter()
            .map(|code| purpose(&coded(code)))
            .collect();
        assert_eq!(purposes_of_event(stated.iter().flatten()), expected);
    }

    #[test]
    fn two_purposes_the_escape_writes_alike_are_both_kept() {
        let stated = [coded("TREAT\u{7}"), coded(r"TREAT\u{7}")];
        let written = purposes_of_event(&stated);
        assert_eq!(written.len(), 2);
        assert_eq!(written[0], written[1]);
    }
}
