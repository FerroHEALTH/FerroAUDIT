// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! Retrieve ATNA Audit Event [ITI-81] answered over the repository's store
//! (`RESTful` ATNA Rev. 3.6 §3.81.4).
//!
//! [`matches()`] decides whether one record meets a [`Query`]: `date` against
//! `recorded` (§3.81.4.1.2.1), each parameter of §3.81.4.1.2.2 against the
//! element that section names, and the other FHIR R4 `AuditEvent` search
//! parameters by the FHIR rules for their type (§3.81.4.1.3). Parameters
//! combine with AND and the values of one with OR.
//!
//! [`retrieve`] reads the query, asks the store for the page, keeps the
//! "Audit Log Used" record §3.81.5.1 has the repository keep of the
//! retrieval, and only then returns the `searchset` Bundle, so no answer
//! leaves the repository unrecorded.

use fhir_search::string::{StringMatch, fold};
use fhir_search::token::Token;
use fhir_types::r4::audit_event::{
    AuditEvent, AuditEventAgent, AuditEventAgentNetwork, AuditEventEntity, AuditEventSource,
};
use fhir_types::r4::codeable_concept::CodeableConcept;
use fhir_types::r4::coding::Coding;
use fhir_types::r4::identifier::Identifier;
use fhir_types::r4::primitives;
use fhir_types::r4::reference::Reference;
use fhir_types::r4::resource::Resource;
use http::StatusCode;
use jiff::Timestamp;
use jiff::tz::TimeZone;

use super::feed::Answer;
use super::{AuditStore, Found, Requester, StoreError};
use crate::retrieve::date::Bound;
use crate::retrieve::{Criterion, Query, Value, bundle};

/// The code system of `AuditEvent.action` (FHIR R4).
const ACTION: &str = "http://hl7.org/fhir/audit-event-action";
/// The code system of `AuditEvent.outcome` (FHIR R4).
const OUTCOME: &str = "http://hl7.org/fhir/audit-event-outcome";
/// The DICOM code system.
const DCM: &str = "http://dicom.nema.org/resources/ontology/DCM";

/// Whether `event` meets every parameter of `query`, a `date` without an
/// offset read in `zone`, the repository's local time zone (FHIR R4 Search
/// §3.1.1.4.7).
#[must_use]
pub fn matches(query: &Query, event: &AuditEvent, zone: &TimeZone) -> bool {
    let Some(recorded) = event
        .recorded
        .value
        .as_deref()
        .and_then(|value| value.parse::<Timestamp>().ok())
    else {
        return false;
    };
    let dated = query.dates().iter().all(|date| match date.bounds(zone) {
        Ok(Bound::Within { from, until }) => {
            from.is_none_or(|from| from <= recorded) && until.is_none_or(|until| recorded < until)
        }
        Ok(Bound::Outside { from, until }) => recorded < from || recorded >= until,
        Ok(Bound::Empty) | Err(_) => false,
    });
    dated
        && query
            .all_criteria()
            .iter()
            .all(|criterion| meets(criterion, event))
}

/// Whether `event` meets one criterion: any of its values.
fn meets(criterion: &Criterion, event: &AuditEvent) -> bool {
    let code = criterion.parameter().code();
    criterion
        .any_of()
        .iter()
        .any(|value| matches_value(code, value, event))
}

/// Whether `event` meets one value of the parameter `code`.
#[expect(
    clippy::too_many_lines,
    reason = "one arm per search parameter of §3.81.4.1.2 and FHIR R4 AuditEvent"
)]
fn matches_value(code: &str, value: &Value, event: &AuditEvent) -> bool {
    let agents = || event.agent.iter();
    let entities = || event.entity.iter();
    match (code, value) {
        ("action", Value::Token(token)) => plain(
            token,
            ACTION,
            event.action.as_ref().and_then(|a| a.value.as_deref()),
        ),
        ("outcome", Value::Token(token)) => plain(
            token,
            OUTCOME,
            event.outcome.as_ref().and_then(|o| o.value.as_deref()),
        ),
        // NOTE: RESTful ATNA §3.81.4.1.2.2: address matches as a substring.
        ("address", Value::String(text)) => agents().any(|agent| {
            network_address(agent.network.as_ref())
                .is_some_and(|address| fold(address).contains(&fold(text)))
        }),
        ("agent-name", Value::String(text)) => agents().any(|agent| {
            agent
                .name
                .as_ref()
                .and_then(|name| name.value.as_deref())
                .is_some_and(|name| StringMatch::default().matches(name, text))
        }),
        ("entity-name", Value::String(text)) => entities().any(|entity| {
            entity
                .name
                .as_ref()
                .and_then(|name| name.value.as_deref())
                .is_some_and(|name| StringMatch::default().matches(name, text))
        }),
        ("agent-role", Value::Token(token)) => {
            agents().any(|agent| agent.role.iter().any(|role| concept(token, role)))
        }
        ("altid", Value::Token(token)) => agents().any(|agent| {
            plain(
                token,
                "",
                agent.alt_id.as_ref().and_then(|id| id.value.as_deref()),
            )
        }),
        ("entity-role", Value::Token(token)) => {
            entities().any(|entity| entity.role.as_ref().is_some_and(|role| coding(token, role)))
        }
        ("entity-type", Value::Token(token)) => entities().any(|entity| {
            entity
                .r#type
                .as_ref()
                .is_some_and(|kind| coding(token, kind))
        }),
        ("site", Value::Token(token)) => plain(
            token,
            "",
            event
                .source
                .site
                .as_ref()
                .and_then(|site| site.value.as_deref()),
        ),
        ("subtype", Value::Token(token)) => {
            event.subtype.iter().any(|subtype| coding(token, subtype))
        }
        ("type", Value::Token(token)) => coding(token, &event.r#type),
        ("policy", Value::Uri(uri)) => agents().any(|agent| {
            agent
                .policy
                .iter()
                .any(|policy| policy.value.as_deref() == Some(uri))
        }),
        ("agent", Value::Reference(target)) => agents().any(|agent| {
            agent
                .who
                .as_ref()
                .is_some_and(|who| reference(target, who, None))
        }),
        ("entity", Value::Reference(target)) => entities().any(|entity| {
            entity
                .what
                .as_ref()
                .is_some_and(|what| reference(target, what, None))
        }),
        ("source", Value::Reference(target)) => reference(target, &event.source.observer, None),
        ("patient", Value::Reference(target)) => {
            agents().any(|agent| {
                agent
                    .who
                    .as_ref()
                    .is_some_and(|who| reference(target, who, Some("Patient")))
            }) || entities().any(|entity| {
                entity
                    .what
                    .as_ref()
                    .is_some_and(|what| reference(target, what, Some("Patient")))
            })
        }
        ("agent.identifier", Value::Token(token)) => {
            agents().any(|agent| identified(token, agent.who.as_ref()))
        }
        ("entity.identifier", Value::Token(token)) => {
            entities().any(|entity| identified(token, entity.what.as_ref()))
        }
        ("source.identifier", Value::Token(token)) => {
            identified(token, Some(&event.source.observer))
        }
        // NOTE: RESTful ATNA §3.81.4.1.2.2 matches patient.identifier where the reference
        // resolves to a Patient: an agent who is one, or an entity in the Patient role.
        ("patient.identifier", Value::Token(token)) => {
            agents().any(|agent| is_patient_agent(agent) && identified(token, agent.who.as_ref()))
                || entities().any(|entity| {
                    is_patient_entity(entity) && identified(token, entity.what.as_ref())
                })
        }
        _ => false,
    }
}

/// The address of `network`.
fn network_address(network: Option<&AuditEventAgentNetwork>) -> Option<&str> {
    network?.address.as_ref()?.value.as_deref()
}

/// Whether `token` matches the code `value` of the fixed code system
/// `system`, or of none when `system` is empty (FHIR R4 Search §3.1.1.4.10).
fn plain(token: &Token, system: &str, value: Option<&str>) -> bool {
    let Some(value) = value else {
        return false;
    };
    match token {
        Token::Code(code) => code == value,
        Token::SystemCode {
            system: asked,
            code,
        } => !system.is_empty() && asked == system && code == value,
        Token::NoSystem(code) => system.is_empty() && code == value,
        Token::System(asked) => !system.is_empty() && asked == system,
    }
}

/// Whether `token` matches the system and code of `coding`.
fn coding(token: &Token, coding: &Coding) -> bool {
    pair(
        token,
        coding
            .system
            .as_ref()
            .and_then(|system| system.value.as_deref()),
        coding.code.as_ref().and_then(|code| code.value.as_deref()),
    )
}

/// Whether `token` matches any coding of `concept`.
fn concept(token: &Token, concept: &CodeableConcept) -> bool {
    concept.coding.iter().any(|one| coding(token, one))
}

/// Whether `token` matches the system and value of the identifier of
/// `reference`.
fn identified(token: &Token, reference: Option<&Reference>) -> bool {
    let Some(identifier) = reference.and_then(|reference| reference.identifier.as_deref()) else {
        return false;
    };
    identifier_pair(token, identifier)
}

fn identifier_pair(token: &Token, identifier: &Identifier) -> bool {
    pair(
        token,
        identifier
            .system
            .as_ref()
            .and_then(|system| system.value.as_deref()),
        identifier
            .value
            .as_ref()
            .and_then(|value| value.value.as_deref()),
    )
}

/// Whether `token` matches a system and a code (FHIR R4 Search §3.1.1.4.10).
fn pair(token: &Token, system: Option<&str>, code: Option<&str>) -> bool {
    match token {
        Token::Code(asked) => code == Some(asked.as_str()),
        Token::SystemCode {
            system: asked,
            code: value,
        } => system == Some(asked.as_str()) && code == Some(value.as_str()),
        Token::NoSystem(asked) => system.is_none() && code == Some(asked.as_str()),
        Token::System(asked) => system == Some(asked.as_str()),
    }
}

/// Whether the reference parameter value `target`, an id, a
/// `[type]/[id]` or an absolute URL, names what `reference` refers to
/// (FHIR R4 Search §3.1.1.4.12), of the resource type `of` when given.
fn reference(target: &str, reference: &Reference, of: Option<&str>) -> bool {
    let Some(literal) = reference
        .reference
        .as_ref()
        .and_then(|r| r.value.as_deref())
    else {
        return false;
    };
    let literal = literal.split("/_history/").next().unwrap_or(literal);
    let mut segments = literal.rsplit('/');
    let id = segments.next().unwrap_or_default();
    let kind = segments
        .next()
        .or_else(|| reference.r#type.as_ref().and_then(|t| t.value.as_deref()));
    if of.is_some_and(|of| kind != Some(of)) {
        return false;
    }
    if target.contains("://") {
        return literal == target;
    }
    match target.split_once('/') {
        Some((asked_kind, asked_id)) => kind == Some(asked_kind) && id == asked_id,
        None => id == target,
    }
}

/// Whether `agent` is the patient: its `who` refers to a `Patient`.
fn is_patient_agent(agent: &AuditEventAgent) -> bool {
    agent.who.as_ref().is_some_and(refers_to_patient)
}

/// Whether `entity` is the patient: in the `Patient` object role, or its
/// `what` refers to a `Patient`.
fn is_patient_entity(entity: &AuditEventEntity) -> bool {
    entity
        .role
        .as_ref()
        .and_then(|role| role.code.as_ref())
        .and_then(|code| code.value.as_deref())
        == Some("1")
        || entity.what.as_ref().is_some_and(refers_to_patient)
}

fn refers_to_patient(reference: &Reference) -> bool {
    reference.r#type.as_ref().and_then(|t| t.value.as_deref()) == Some("Patient")
        || reference
            .reference
            .as_ref()
            .and_then(|r| r.value.as_deref())
            .is_some_and(|r| r.starts_with("Patient/") || r.contains("/Patient/"))
}

/// One ITI-81 request as the product's HTTP server took it.
#[derive(Debug, Clone, Copy)]
pub struct Retrieval<'a> {
    /// The query string, the part after `?`.
    pub query: &'a str,
    /// The absolute URL of the repository's `AuditEvent` search, without a
    /// query, under which the `self` and `next` links are written.
    pub endpoint: &'a str,
    /// Who asks.
    pub requester: &'a Requester,
    /// The repository's `AuditSourceID`, the observer of its own record.
    pub observer: &'a str,
}

/// Answers the ITI-81 `retrieval` over `store` (§3.81.4.2).
///
/// A query that cannot be read is answered `400` with the human-readable
/// description §3.81.4.2.2 asks for; a store that refuses the requester,
/// `403`; a store that cannot be searched, or cannot keep the "Audit Log
/// Used" record, `503`, and the matches are then not returned.
pub async fn retrieve(store: &dyn AuditStore, retrieval: Retrieval<'_>) -> Answer {
    let query = match Query::parse(retrieval.query) {
        Ok(query) => query,
        Err(error) => {
            return refused(
                StatusCode::BAD_REQUEST,
                bundle::IssueType::Invalid,
                &error.to_string(),
            );
        }
    };
    let found = match store.search(&query, retrieval.requester).await {
        Ok(found) => found,
        Err(StoreError::Refused(_)) => {
            return refused(
                StatusCode::FORBIDDEN,
                bundle::IssueType::Forbidden,
                "the requester may not read these audit records",
            );
        }
        Err(StoreError::Unavailable(error)) => {
            tracing::warn!(target: crate::atna::targets::REPOSITORY, error = %crate::atna::chain(error.as_ref()), "the audit store could not be searched");
            return refused(
                StatusCode::SERVICE_UNAVAILABLE,
                bundle::IssueType::Transient,
                "the audit records cannot be searched now",
            );
        }
    };
    let used = audit_log_used(&found, &retrieval, Timestamp::now());
    if let Err(error) = store.keep(used).await {
        tracing::error!(target: crate::atna::targets::REPOSITORY, error = %crate::atna::chain(&error), "the Audit Log Used record could not be kept; the retrieval is refused");
        return refused(
            StatusCode::SERVICE_UNAVAILABLE,
            bundle::IssueType::Transient,
            "the retrieval cannot be recorded now",
        );
    }
    let self_url = format!("{}?{}", retrieval.endpoint, retrieval.query);
    let offset = query.offset().unwrap_or(0);
    let shown = u64::try_from(found.matches.len()).unwrap_or(u64::MAX);
    let next_offset = offset.saturating_add(shown);
    let next = (shown > 0 && next_offset < u64::from(found.total)).then(|| {
        format!(
            "{}?{}",
            retrieval.endpoint,
            with_offset(retrieval.query, next_offset)
        )
    });
    Answer {
        status: StatusCode::OK,
        location: None,
        body: Some(Resource::Bundle(Box::new(bundle::searchset(
            found.total,
            &self_url,
            next.as_deref(),
            found.matches,
        )))),
    }
}

fn refused(status: StatusCode, code: bundle::IssueType, diagnostics: &str) -> Answer {
    Answer {
        status,
        location: None,
        body: Some(Resource::OperationOutcome(Box::new(bundle::refusal(
            code,
            diagnostics,
        )))),
    }
}

/// `query` with its `_offset` set to `offset`.
fn with_offset(query: &str, offset: u64) -> String {
    let mut pairs: Vec<String> = query
        .split('&')
        .filter(|pair| !pair.is_empty() && !pair.starts_with("_offset=") && *pair != "_offset")
        .map(str::to_owned)
        .collect();
    pairs.push(format!("_offset={offset}"));
    pairs.join("&")
}

/// The "Audit Log Used" record of one retrieval (§3.81.5.1), in the
/// `AuditEvent` form Table 3.20.4.2.2.1-1 maps its DICOM fields to.
fn audit_log_used(found: &Found, retrieval: &Retrieval<'_>, at: Timestamp) -> AuditEvent {
    let dcm = |code: &str, display: &str| Coding {
        system: Some(primitives::Uri::from(DCM)),
        code: Some(primitives::Code::from(code)),
        display: Some(primitives::String::from(display)),
        ..Coding::default()
    };
    let concept = |coding: Coding| CodeableConcept {
        coding: vec![coding],
        ..CodeableConcept::default()
    };
    let who = |value: &str| Reference {
        identifier: Some(Box::new(Identifier {
            value: Some(primitives::String::from(value)),
            ..Identifier::default()
        })),
        ..Reference::default()
    };
    let network = |address: &str| AuditEventAgentNetwork {
        address: Some(primitives::String::from(address)),
        // NOTE: §3.81.5.1: "1" for a machine (DNS) name, "2" for an IP address.
        r#type: Some(primitives::Code::from(
            if address.parse::<std::net::IpAddr>().is_ok() {
                "2"
            } else {
                "1"
            },
        )),
        ..AuditEventAgentNetwork::default()
    };
    let requester = retrieval.requester;
    let consumer = requester.address.as_deref().unwrap_or("unknown");
    let mut agent = vec![AuditEventAgent {
        r#type: Some(concept(dcm("110153", "Source Role ID"))),
        who: Some(who(consumer)),
        requestor: primitives::Boolean::from(requester.user.is_none()),
        network: requester.address.as_deref().map(network),
        ..AuditEventAgent::default()
    }];
    if let Some(user) = &requester.user {
        agent.push(AuditEventAgent {
            who: Some(who(user)),
            requestor: primitives::Boolean::from(true),
            ..AuditEventAgent::default()
        });
    }
    let host = url::Url::parse(retrieval.endpoint)
        .ok()
        .and_then(|url| url.host_str().map(str::to_owned));
    agent.push(AuditEventAgent {
        r#type: Some(concept(dcm("110152", "Destination Role ID"))),
        who: Some(who(retrieval.endpoint)),
        requestor: primitives::Boolean::from(false),
        network: host.as_deref().map(network),
        ..AuditEventAgent::default()
    });
    let entity = found
        .matches
        .iter()
        .filter_map(|found| found.full_url.as_deref())
        .map(returned)
        .collect();
    AuditEvent {
        r#type: dcm("110101", "Audit Log Used"),
        subtype: vec![Coding {
            system: Some(primitives::Uri::from("urn:ihe:event-type-code")),
            code: Some(primitives::Code::from("ITI-81")),
            display: Some(primitives::String::from("Retrieve ATNA Audit Event")),
            ..Coding::default()
        }],
        action: Some(primitives::Code::from("R")),
        recorded: primitives::Instant::from(at.to_string()),
        outcome: Some(primitives::Code::from("0")),
        agent,
        source: AuditEventSource {
            observer: who(retrieval.observer),
            ..AuditEventSource::default()
        },
        entity,
        ..AuditEvent::default()
    }
}

/// The entity of one record a retrieval returned, by its URL (§3.81.5.1,
/// the `AuditEvent Message` participant).
fn returned(url: &str) -> AuditEventEntity {
    AuditEventEntity {
        what: Some(Reference {
            identifier: Some(Box::new(Identifier {
                // NOTE: §3.81.5.1: ParticipantObjectIDTypeCode EV("12", "RFC-3881", "URI").
                r#type: Some(CodeableConcept {
                    coding: vec![Coding {
                        code: Some(primitives::Code::from("12")),
                        display: Some(primitives::String::from("URI")),
                        ..Coding::default()
                    }],
                    ..CodeableConcept::default()
                }),
                value: Some(primitives::String::from(url)),
                ..Identifier::default()
            })),
            ..Reference::default()
        }),
        r#type: Some(Coding {
            system: Some(primitives::Uri::from(
                "http://terminology.hl7.org/CodeSystem/audit-entity-type",
            )),
            code: Some(primitives::Code::from("2")),
            ..Coding::default()
        }),
        role: Some(Coding {
            system: Some(primitives::Uri::from(
                "http://terminology.hl7.org/CodeSystem/object-role",
            )),
            code: Some(primitives::Code::from("13")),
            ..Coding::default()
        }),
        ..AuditEventEntity::default()
    }
}
