// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! An access record written as an IHE BALP `AuditEvent` (feature `balp`),
//! through the audit types of `ihe-atna`.
//!
//! The recording system received the request, so the record is the server's
//! side of the `RESTful` pattern its action names (BALP 1.1.4): a query is
//! `IHE.BasicAudit.Query`, a read `IHE.BasicAudit.Read`, a create
//! `IHE.BasicAudit.Create`, an update `IHE.BasicAudit.Update` and a delete
//! `IHE.BasicAudit.Delete`, each its `Patient` variant when the record names
//! one patient, as the implementation guide lists them: the one the request
//! named, or the one the identity service found behind the `ehr_id` it
//! reached. Its `entity:patient` slice holds one patient, so a record that
//! names several writes each and claims the plain pattern. The Delete pattern
//! fixes `outcome` to `0`, so a failed delete claims no profile. The person
//! and the client are the token's (BALP 1.1.4 §3:5.7.5.4), with the provider
//! as an agent of its own, the request id is `entity:transaction`, and the
//! query `entity:query`.
//!
//! The person's agent, `agent:user`, alone carries what Annex II 3.2(b) asks
//! about the natural person: the professional's name as `who.display` and
//! their identifier as an `ihe-otherId` extension typed `NPI`, as BALP
//! 1.1.4 §3:5.7.5.4 maps the IHE IUA `subject_name` and
//! `national_provider_identifier`; the assurance level of the
//! authentication, when one was established, as an `ihe-assuranceLevel`
//! extension coded `low`, `substantial` or `high` (Regulation (EU) No
//! 910/2014 Art 8(2)); and who acts, `person` or `client`, as its `role`.
//!
//! What Annex II 3.2 adds to a BALP record has no element of its own (BALP
//! defines none for a data category), so it rides in entities of type `4`,
//! which no slice of a pattern is discriminated by: one for the categories
//! and the record's retention, one per origin and one per `ehr_id`, each
//! with its `detail` entries named as [`detail`] lists them (no
//! specification governs the names: our own design). The professional and
//! the provider a national contact point relays (Implementing Regulation
//! (EU) 2026/2099 Annex Tables 1 and 2) ride the same way, in one entity
//! `ehds-relayed` that names the contact point and marks them `asserted`,
//! and a correlation identifier the client sent is a `detail` of the request
//! id's entity.
//!
//! The record's security labels ([`crate::label`], Annex II 3.4) are each
//! the `securityLabel` of the entity it belongs to: the categories' on the
//! `ehds-categories` entity and each origin's on its `origin` entity, which
//! the `RESTful` ATNA mapping keeps as the DICOM
//! `ParticipantObjectSensitivity` in both directions (`RESTful` ATNA Rev. 3.6
//! Tables 3.20.4.2.2.1-1 and 3.81.4.2.2.1-1), as BALP puts the labels of the
//! data on its entity (`IHE.BasicAudit.Read`, `entity:data.securityLabel`).
//! The categories are written there alone: that entity is the category
//! record of Annex II 3.2(c), and `meta`, which neither mapping carries,
//! would hold a second copy of all of them on every record. The origins'
//! labels are also the record's `meta.security`, which a FHIR repository
//! can limit access by.
//!
//! An emergency access (Regulation (EU) 2025/327 Art 11(5)) keeps its
//! purposes of use where every purpose is written, in `agent:user`
//! `purposeOfUse` and in `purposeOfEvent`, and is marked in one entity of
//! its own, `ehds-emergency-access`, whose `description` states the mark in words and
//! whose `detail` entries name the purposes that marked it, so a reader of
//! the record need not know which codes the deployment maps, and each member
//! whose consent pre-filter denial was set aside for it.
//!
//! A population access, a query over many patients' data whose record names
//! none of them, is marked in one entity `ehds-population`, whose
//! `description` says in words that the query ran over each origin the
//! record names and that the record confirms no read of any one person's
//! record, and whose `detail` entries name why no `ehr_id` is named and the
//! template and archetype ids the query read (Regulation (EU) 2025/327 Art
//! 9(1)). BALP 1:52.4.1.6 recommends one record per patient "driven by a
//! Policy"; this record names the scope, never a person.
//!
//! Each semantic transformation the access applied is one entity of its own
//! ([`transformation`]), in the same `AuditEvent` as the access, so it is
//! kept and found with it (eHealth Network General Guidelines Release 3.1
//! Art 6(3)).
//!
//! # Examples
//!
//! A record of a read, handed to the BALP sink, which writes it as an
//! `AuditEvent` and gives it to an `ihe-atna` recorder. The sink names the
//! recording system by `endpoint`: the address its requests arrive at, its
//! base URL or, when it knows only that, its host or IP address, written as
//! the network address of its own agent.
//!
//! ```
//! use std::collections::BTreeMap;
//! use std::sync::Arc;
//!
//! use ehds_logging::balp::BalpSink;
//! use ehds_logging::classify::{Basis, Evidence, RootObject};
//! use ehds_logging::label::LabelPolicy;
//! use ehds_logging::map::{CategoryMap, Declared};
//! use ehds_logging::record::{
//!     AccessRecord, Accessor, Acting, Action, DataSubject, Outcome, Professional, Request,
//! };
//! use ehds_logging::retention::RetentionPolicy;
//! use ehds_logging::sink::AccessSink;
//! use ihe_atna::balp::{AuditError, AuditRecorder, Exchange, NetworkAddress};
//!
//! /// A recorder that keeps nothing, where a deployment spools for its
//! /// Audit Record Repository.
//! struct Recorder;
//!
//! #[async_trait::async_trait]
//! impl AuditRecorder for Recorder {
//!     async fn record(&self, _exchange: Exchange) -> Result<(), AuditError> {
//!         Ok(())
//!     }
//! }
//!
//! # #[tokio::main(flavor = "current_thread")]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let templates = BTreeMap::from([(
//!     "Example Lab Report.v1".to_owned(),
//!     Declared::Codes(vec!["Laboratory-Reports".to_owned()]),
//! )]);
//! let map = CategoryMap::declare(&[], &templates, &BTreeMap::new())?;
//! let read = RootObject {
//!     template_id: Some("Example Lab Report.v1".to_owned()),
//!     ..RootObject::default()
//! };
//! let categories = map.classify(&Evidence::reached(Basis::Returned, vec![read]));
//! let recorded = jiff::Timestamp::now();
//! let record = AccessRecord {
//!     action: Action::Read,
//!     recorded,
//!     outcome: Outcome::Success,
//!     outcome_description: None,
//!     accessor: Accessor {
//!         issuer: Some("https://idp.example.org".to_owned()),
//!         subject: "Qz7-clinician".to_owned(),
//!         client_id: Some("ward-app".to_owned()),
//!         audience: None,
//!         provider: None,
//!         acting: Some(Acting::Person),
//!         roles: Vec::new(),
//!         token_id: None,
//!         assurance: None,
//!         professional: Professional::default(),
//!         alt_id: None,
//!         purposes: Vec::new(),
//!         relayed: None,
//!     },
//!     subject: DataSubject::default(),
//!     retention: RetentionPolicy::default().retain(recorded, &categories, []),
//!     labels: LabelPolicy::default().label(&categories, []),
//!     categories,
//!     delivered: Some(1),
//!     origins: Vec::new(),
//!     origin_count: None,
//!     request: Request {
//!         id: Some("Qz7-request".to_owned()),
//!         operation: "composition_get".to_owned(),
//!         operation_system: None,
//!         resource: None,
//!         query: None,
//!         stored_query: None,
//!         client_address: None,
//!         correlation: None,
//!     },
//!     emergency: None,
//!     inserted: None,
//!     transformations: Vec::new(),
//! };
//! let sink = BalpSink::new(Arc::new(Recorder), NetworkAddress::host("cdr.example.org"));
//! sink.store(record).await?;
//! # Ok(())
//! # }
//! ```

pub mod detail;
pub mod inserted;
pub mod transformation;

use std::collections::BTreeSet;
use std::sync::Arc;

use ihe_atna::balp::{
    APPLICATION, AuditRecorder, CREATE, CUSTODIAN, Coded, DELETE, DESTINATION_ROLE,
    DOMAIN_RESOURCE, Described, Detail, Direction, Entity, EventKind, Exchange, NetworkAddress,
    OTHER, Peer, READ, REQUEST_ID, REST, Reserved, SEARCH, SOURCE_ROLE, SYSTEM_OBJECT,
    SecurityLabel, UPDATE, What,
};
use ihe_atna::user::{Code, OnBehalfOf, PurposeOfUse, User};
use secrecy::{ExposeSecret as _, SecretString};
use url::Url;

use crate::classify::{Basis, Classification};
use crate::emergency::Emergency;
use crate::label::Label;
use crate::record::{
    AccessRecord, Action, Origin, Outcome, PatientIdentifier, PatientLookup, Population, Relayed,
};
use crate::retention::Retention;
use crate::sink::{AccessSink, Reservation, SinkError};

/// The name of the entity that marks an emergency access.
pub const EMERGENCY_ENTITY: &str = "ehds-emergency-access";

/// The `description` of the entity that marks an emergency access.
pub const EMERGENCY_DESCRIPTION: &str = "Emergency access: the accessor declared that the access \
     was necessary to protect the vital interests of the data subject \
     (Regulation (EU) 2025/327 Art 11(5)).";

/// The name of the entity that marks a population access.
pub const POPULATION_ENTITY: &str = "ehds-population";

/// The `description` of the entity that marks a population access.
pub const POPULATION_DESCRIPTION: &str = "Population access: a query over many \
     patients' data ran over each origin this record names; the record does not name \
     the persons whose records it read, and does not say that it read any one \
     person's record (Regulation (EU) 2025/327 Art 9(1)).";

/// The sink that writes each record as a BALP `AuditEvent` and hands it to
/// an `ihe-atna` audit recorder, such as one that spools it for an ATNA
/// Audit Record Repository.
pub struct BalpSink {
    recorder: Arc<dyn AuditRecorder>,
    endpoint: NetworkAddress,
}

impl std::fmt::Debug for BalpSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BalpSink").finish_non_exhaustive()
    }
}

impl BalpSink {
    /// The sink that records through `recorder`, naming the recording system
    /// by `endpoint`, the address its requests arrive at: the base URL it
    /// serves, or its host or IP address when it knows only that, written as
    /// the network address of the recording system's agent.
    #[must_use]
    pub fn new(recorder: Arc<dyn AuditRecorder>, endpoint: NetworkAddress) -> Self {
        Self { recorder, endpoint }
    }
}

#[async_trait::async_trait]
impl AccessSink for BalpSink {
    async fn store(&self, record: AccessRecord) -> Result<(), SinkError> {
        self.recorder
            .record(exchange(&record, &self.endpoint))
            .await
            .map_err(|error| SinkError(Box::new(error)))
    }

    async fn reserve(&self) -> Result<Box<dyn Reservation>, SinkError> {
        let place = self
            .recorder
            .reserve()
            .await
            .map_err(|error| SinkError(Box::new(error)))?;
        Ok(Box::new(BalpReservation {
            place,
            recorder: Arc::clone(&self.recorder),
            endpoint: self.endpoint.clone(),
        }))
    }
}

/// The place of one record [`BalpSink`] took: the recorder's own place, or
/// none when the recorder holds no bounded store.
struct BalpReservation {
    place: Option<Box<dyn Reserved>>,
    recorder: Arc<dyn AuditRecorder>,
    endpoint: NetworkAddress,
}

impl std::fmt::Debug for BalpReservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BalpReservation")
            .field("place", &self.place)
            .finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl Reservation for BalpReservation {
    async fn store(self: Box<Self>, record: AccessRecord) -> Result<(), SinkError> {
        let exchange = exchange(&record, &self.endpoint);
        match self.place {
            Some(place) => place.record(exchange).await,
            None => self.recorder.record(exchange).await,
        }
        .map_err(|error| SinkError(Box::new(error)))
    }
}

/// The BALP exchange `record` is written as, the recording system named by
/// `endpoint`, the address its requests arrive at.
#[must_use]
pub fn exchange(record: &AccessRecord, endpoint: &NetworkAddress) -> Exchange {
    let accessor = &record.accessor;
    // NOTE: no specification governs the acting codes: our own design; BALP 1.1.4 names no element
    // for who acts, and §3:5.7.5.4 writes the IUA subject_role at agent[user].role beside it.
    let roles = accessor
        .acting
        .map(|acting| code(acting.code()))
        .into_iter()
        .chain(accessor.roles.iter().map(|role| Code {
            system: role.system.clone(),
            code: role.code.clone(),
        }))
        .collect();
    let user = User::local(accessor.subject.clone())
        .with_issuer(accessor.issuer.clone())
        .with_client_id(accessor.client_id.clone())
        .with_token_id(accessor.token_id.clone())
        .with_audience(accessor.audience.clone())
        .with_purposes(
            accessor
                .purposes
                .iter()
                .map(|purpose| PurposeOfUse {
                    system: purpose.system.clone(),
                    code: purpose.code.clone(),
                })
                .collect(),
        )
        .with_organisation(accessor.provider.clone())
        .with_alt_id(accessor.alt_id.clone())
        .with_name(accessor.professional.name.clone())
        .with_provider_identifier(accessor.professional.identifier.clone())
        // NOTE: no specification governs these codes: our own design; BALP 1.1.4 leaves the
        // assuranceLevel vocabulary open.
        .with_assurance(accessor.assurance.map(|level| code(level.code())))
        .with_roles(roles);
    let client = Peer {
        who: accessor
            .client_id
            .clone()
            .unwrap_or_else(|| accessor.subject.clone()),
        network: record.request.client_address.map_or_else(
            || NetworkAddress::MachineName(String::from("unknown")),
            NetworkAddress::Ip,
        ),
    };
    Exchange {
        kind: kind(record),
        recorded: record.recorded,
        outcome: match record.outcome {
            Outcome::Success => ihe_atna::balp::Outcome::Success,
            Outcome::MinorFailure => ihe_atna::balp::Outcome::MinorFailure,
            Outcome::SeriousFailure => ihe_atna::balp::Outcome::SeriousFailure,
            Outcome::MajorFailure => ihe_atna::balp::Outcome::MajorFailure,
        },
        outcome_description: record.outcome_description.clone(),
        direction: Direction::Received {
            client,
            endpoint: endpoint.clone(),
        },
        on_behalf: OnBehalfOf::User(user),
        entities: entities(record),
        // NOTE: RESTful ATNA Rev. 3.6 Table 3.81.4.2.2.1-1 maps `entity.securityLabel`
        // and no `meta`, so the categories ride on their entity alone (#108).
        security: security(&record.labels.origins()),
    }
}

/// A code no system defines.
fn code(code: &str) -> Code {
    Code {
        system: None,
        code: code.to_owned(),
    }
}

/// What the pattern of `record`'s action fixes.
fn kind(record: &AccessRecord) -> EventKind {
    let patient = patients(record).len() == 1;
    // NOTE: every BALP 1.1.4 RESTful pattern slices subtype with rules open, so the operation
    // follows the pattern's restful-interaction code.
    let subtypes = |fixed: Coded| {
        let request = &record.request;
        let mut subtypes = vec![fixed];
        subtypes.extend(request.operation_system.as_ref().map(|system| {
            Coded::new(
                system.clone(),
                request.operation.clone(),
                request.operation.clone(),
            )
        }));
        subtypes
    };
    let profile = |plain: &'static str, with_patient: &'static str| {
        if patient { with_patient } else { plain }
    };
    match record.action {
        Action::Query => EventKind {
            profile: profile(
                "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Query",
                "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.PatientQuery",
            ),
            event_type: REST,
            subtypes: subtypes(SEARCH),
            action: "E",
            client: SOURCE_ROLE,
            server: DESTINATION_ROLE,
        },
        Action::Read => EventKind {
            profile: profile(
                "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Read",
                "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.PatientRead",
            ),
            event_type: REST,
            subtypes: subtypes(READ),
            action: "R",
            client: DESTINATION_ROLE,
            server: SOURCE_ROLE,
        },
        Action::Create => EventKind {
            profile: profile(
                "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Create",
                "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.PatientCreate",
            ),
            event_type: REST,
            subtypes: subtypes(CREATE),
            action: "C",
            client: SOURCE_ROLE,
            server: DESTINATION_ROLE,
        },
        Action::Update => EventKind {
            profile: profile(
                "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Update",
                "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.PatientUpdate",
            ),
            event_type: REST,
            subtypes: subtypes(UPDATE),
            action: "U",
            client: SOURCE_ROLE,
            server: DESTINATION_ROLE,
        },
        Action::Delete => EventKind {
            profile: if record.outcome == Outcome::Success {
                profile(
                    "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Delete",
                    "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.PatientDelete",
                )
            } else {
                ""
            },
            event_type: REST,
            subtypes: subtypes(DELETE),
            action: "D",
            client: APPLICATION,
            server: CUSTODIAN,
        },
    }
}

/// The entities of `record`: the request id, the query or the resource,
/// the patient, each `ehr_id`, the categories, and each origin.
fn entities(record: &AccessRecord) -> Vec<Entity> {
    let request = &record.request;
    let correlation = request
        .correlation
        .iter()
        .map(|correlation| Detail::new(detail::CORRELATION, correlation.clone()));
    // NOTE: BALP 1.1.4 entity:transaction is 0..1 and needs what.identifier.value, so a request
    // with no id writes none, and its correlation id rides on the categories entity.
    let mut entities: Vec<Entity> = request
        .id
        .iter()
        .map(|id| {
            Entity::Described(Described {
                what: Some(What::Identifier {
                    system: None,
                    value: id.clone(),
                }),
                kind: REQUEST_ID,
                role: None,
                name: None,
                description: None,
                details: correlation.clone().collect(),
                security_labels: Vec::new(),
            })
        })
        .collect();
    match record.action {
        Action::Query => entities.push(Entity::Query(
            request
                .query
                .clone()
                .unwrap_or_else(|| SecretString::from(request.operation.clone())),
        )),
        Action::Read | Action::Create | Action::Update | Action::Delete => {
            entities.push(Entity::Described(Described {
                what: Some(What::Reference {
                    reference: request
                        .resource
                        .clone()
                        .unwrap_or_else(|| request.operation.clone()),
                    resource_type: None,
                }),
                kind: SYSTEM_OBJECT,
                role: Some(DOMAIN_RESOURCE),
                name: Some(request.operation.clone()),
                description: None,
                details: Vec::new(),
                security_labels: Vec::new(),
            }));
        }
    }
    entities.extend(subject_entities(record));
    let mut details = categories(&record.categories);
    if let Some(delivered) = record.delivered {
        details.push(Detail::new(detail::DELIVERED, delivered.to_string()));
    }
    if let Some(name) = &request.stored_query {
        details.push(Detail::new(detail::STORED_QUERY, name.clone()));
    }
    if request.id.is_none() {
        details.extend(correlation);
    }
    if let Some(count) = record.origin_count {
        details.push(Detail::new(detail::ORIGIN_COUNT, count.to_string()));
    }
    details.extend(retention_details(&record.retention));
    entities.push(other(
        None,
        "ehds-categories",
        details,
        record.labels.categories(),
    ));
    for origin in &record.origins {
        let labels = origin
            .key()
            .map_or(&[][..], |key| record.labels.origin(key));
        entities.push(origin_entity(origin, labels));
    }
    if let Some(relayed) = &record.accessor.relayed {
        entities.push(other(None, "ehds-relayed", relayed_details(relayed), &[]));
    }
    entities.extend(record.subject.population.as_ref().map(population_entity));
    if let Some(emergency) = &record.emergency {
        entities.push(emergency_entity(emergency));
    }
    entities.extend(record.inserted.map(inserted::entity));
    entities.extend(record.transformations.iter().map(transformation::entity));
    entities
}

/// The `detail` entries of how long a record is kept: its years, for a
/// period in years, its period, its end date, unless it is kept without end,
/// what called for the period, and the ground of its floor.
fn retention_details(retention: &Retention) -> Vec<Detail> {
    let period = retention.period();
    let mut details: Vec<Detail> = period
        .years()
        .map(|years| Detail::new(detail::RETENTION_YEARS, years.get().to_string()))
        .into_iter()
        .collect();
    details.push(Detail::new(detail::RETENTION_PERIOD, period.code()));
    details.extend(
        retention
            .ends()
            .map(|ends| Detail::new(detail::RETENTION_ENDS, ends.to_string())),
    );
    details.extend([
        Detail::new(detail::RETENTION_GROUND, retention.ground().code()),
        Detail::new(detail::RETENTION_FLOOR, retention.floor().code()),
    ]);
    details
}

/// The entities of whose data `record` concerned: each patient it names, by
/// an identifier whose namespace is its system when it is a URI and a detail
/// when it is not, and each `ehr_id` it reached.
fn subject_entities(record: &AccessRecord) -> Vec<Entity> {
    let mut entities: Vec<Entity> = patients(record)
        .into_iter()
        .map(|patient| {
            let uri = patient
                .namespace
                .as_deref()
                .filter(|namespace| is_uri(namespace));
            Entity::Patient {
                system: uri.map(str::to_owned),
                value: patient.value.clone(),
                namespace: patient.namespace.clone().filter(|_| uri.is_none()),
            }
        })
        .collect();
    entities.extend(record.subject.ehrs.iter().map(|ehr| {
        other(
            Some(ehr.ehr_id.clone()),
            "ehr",
            vec![
                Detail::new(detail::ENDPOINT, ehr.endpoint.clone()),
                Detail::new(detail::PATIENT_LOOKUP, ehr.patient.code()),
            ],
            &[],
        )
    }));
    entities
}

/// The entity of `origin`, labelled `labels`: how it answered, its node, its
/// `system_id`, its rows, the version a write left there, the `ehr_id` an
/// EHR create left there and its categories.
fn origin_entity(origin: &Origin, labels: &[Label]) -> Entity {
    let mut details: Vec<Detail> = origin
        .status
        .iter()
        .map(|status| Detail::new(detail::STATUS, status.clone()))
        .collect();
    details.extend(
        origin
            .node
            .iter()
            .map(|node| Detail::new(detail::NODE, node.clone())),
    );
    details.extend(
        origin
            .system_id
            .iter()
            .map(|system_id| Detail::new(detail::SYSTEM_ID, system_id.clone())),
    );
    details.extend(
        origin
            .rows
            .map(|rows| Detail::new(detail::ROWS, rows.to_string())),
    );
    if let Some(version) = &origin.version {
        details.extend(
            version
                .named()
                .map(|named| Detail::new(detail::WRITTEN_VERSION, named)),
        );
        details.extend(
            version
                .unread()
                .map(|why| Detail::new(detail::WRITTEN_VERSION_UNREAD, why)),
        );
    }
    if let Some(created) = &origin.created_ehr {
        details.extend(
            created
                .named()
                .map(|ehr_id| Detail::new(detail::CREATED_EHR, ehr_id)),
        );
        details.extend(
            created
                .unread()
                .map(|why| Detail::new(detail::CREATED_EHR_UNREAD, why)),
        );
    }
    details.extend(origin.categories.iter().flat_map(categories));
    other(origin.key().map(str::to_owned), "origin", details, labels)
}

/// The entity that marks an emergency access, naming the purposes that
/// marked it and each member whose consent pre-filter denial was set aside.
fn emergency_entity(emergency: &Emergency) -> Entity {
    let mut details = vec![Detail::new(detail::EMERGENCY_ACCESS, "true")];
    details.extend(emergency.purposes().iter().map(|purpose| {
        Detail::new(
            detail::EMERGENCY_PURPOSE,
            coded(purpose.system.as_deref(), &purpose.code),
        )
    }));
    details.extend(
        emergency
            .consent_set_aside()
            .iter()
            .map(|member| Detail::new(detail::CONSENT_SET_ASIDE, member.clone())),
    );
    Entity::Described(Described {
        what: None,
        kind: OTHER,
        role: None,
        name: Some(EMERGENCY_ENTITY.to_owned()),
        description: Some(EMERGENCY_DESCRIPTION.to_owned()),
        details,
        security_labels: Vec::new(),
    })
}

/// The security labels `labels` are written as.
fn security(labels: &[Label]) -> Vec<SecurityLabel> {
    labels
        .iter()
        .map(|label| SecurityLabel {
            system: label.system().to_owned(),
            version: label.version().map(str::to_owned),
            code: label.code().to_owned(),
        })
        .collect()
}

/// The entity that marks a population access, with why it names no
/// `ehr_id` and the template and archetype ids the query read.
fn population_entity(population: &Population) -> Entity {
    let mut details = vec![
        Detail::new(detail::POPULATION, "true"),
        Detail::new(detail::POPULATION_REASON, population.reason.code()),
    ];
    details.extend(
        population
            .ehrs
            .map(|ehrs| Detail::new(detail::POPULATION_EHRS, ehrs.to_string())),
    );
    details.push(Detail::new(
        detail::POPULATION_LIMIT,
        population.limit.to_string(),
    ));
    for (name, ids) in [
        (detail::TEMPLATE_ID, &population.templates),
        (detail::ARCHETYPE_ID, &population.archetypes),
    ] {
        details.extend(ids.iter().map(|id| Detail::new(name, id.clone())));
    }
    Entity::Described(Described {
        what: None,
        kind: OTHER,
        role: None,
        name: Some(POPULATION_ENTITY.to_owned()),
        description: Some(POPULATION_DESCRIPTION.to_owned()),
        details,
        security_labels: Vec::new(),
    })
}

/// `<system>|<code>`, or `<code>` when no system is named.
fn coded(system: Option<&str>, code: &str) -> String {
    match system {
        Some(system) => format!("{system}|{code}"),
        None => code.to_owned(),
    }
}

/// The `detail` entries of what a national contact point relayed, marked
/// as its assertion.
fn relayed_details(relayed: &Relayed) -> Vec<Detail> {
    let professional = &relayed.professional;
    let provider = &relayed.provider;
    let mut details = vec![
        Detail::new(detail::CONTACT_POINT, relayed.contact_point.clone()),
        Detail::new(detail::ASSERTED, "true"),
        Detail::new(detail::COUNTRY_CODE, relayed.country_code.clone()),
        Detail::new(detail::FAMILY_NAME, professional.family_name.clone()),
        Detail::new(detail::GIVEN_NAME, professional.given_name.clone()),
        Detail::new(detail::HP_IDENTIFIER, professional.identifier.clone()),
        Detail::new(
            detail::HP_ISSUING_AUTHORITY,
            professional.issuing_authority.clone(),
        ),
    ];
    details.extend(
        professional
            .roles
            .iter()
            .map(|role| Detail::new(detail::HP_ROLE, coded(role.system.as_deref(), &role.code))),
    );
    details.extend([
        Detail::new(detail::PROVIDER_IDENTIFIER, provider.identifier.clone()),
        Detail::new(
            detail::PROVIDER_ISSUING_AUTHORITY,
            provider.issuing_authority.clone(),
        ),
        Detail::new(detail::PROVIDER_NAME, provider.name.clone()),
        Detail::new(detail::PROVIDER_ADDRESS, provider.address.clone()),
    ]);
    details
}

/// Every patient `record` names, each once: the one the request named, and
/// each one the identity service found behind an `ehr_id` the access
/// reached.
fn patients(record: &AccessRecord) -> Vec<&PatientIdentifier> {
    let found = record
        .subject
        .ehrs
        .iter()
        .flat_map(|ehr| match &ehr.patient {
            PatientLookup::Found(found) => found.as_slice(),
            _ => &[],
        });
    let mut named: Vec<&PatientIdentifier> = Vec::new();
    for patient in record.subject.patient.iter().chain(found) {
        let seen = named.iter().any(|kept: &&PatientIdentifier| {
            kept.namespace == patient.namespace
                && kept.value.expose_secret() == patient.value.expose_secret()
        });
        if !seen {
            named.push(patient);
        }
    }
    named
}

/// Whether `namespace` is an absolute URI (RFC 3986 §4.3), which FHIR R4
/// `Identifier.system` takes.
fn is_uri(namespace: &str) -> bool {
    Url::parse(namespace).is_ok()
}

/// An entity of type `4` named `name`, identified by `value` when given,
/// labelled `labels`.
fn other(value: Option<String>, name: &str, details: Vec<Detail>, labels: &[Label]) -> Entity {
    Entity::Described(Described {
        what: value.map(|value| What::Identifier {
            system: None,
            value,
        }),
        kind: OTHER,
        role: None,
        name: Some(name.to_owned()),
        description: None,
        details,
        security_labels: security(labels),
    })
}

/// The `detail` entries of `classified`.
fn categories(classified: &Classification) -> Vec<Detail> {
    let mut details = Vec::new();
    let mut versions = BTreeSet::new();
    for (category, bases) in classified.categories() {
        details.push(Detail::new(detail::CATEGORY, category.token()));
        for basis in bases {
            details.push(Detail::new(
                detail::BASIS,
                format!("{}:{}", category.token(), basis.code()),
            ));
        }
        if let Some(version) = category.version() {
            versions.insert(format!("{}|{version}", category.system()));
        }
    }
    details.extend(
        versions
            .into_iter()
            .map(|version| Detail::new(detail::CATEGORY_VERSION, version)),
    );
    if classified.is_no_category() {
        details.push(Detail::new(detail::NO_CATEGORY, "true"));
    }
    let kinds = classified.no_category_kinds();
    details.extend(
        kinds
            .iter()
            .map(|kind| Detail::new(detail::NO_CATEGORY_KIND, kind.code())),
    );
    if !kinds.is_empty() {
        details.push(Detail::new(
            detail::NO_CATEGORY_BASIS,
            Basis::ResourceKind.code(),
        ));
    }
    if let Some(unclassified) = classified.unclassified() {
        details.push(Detail::new(detail::UNCLASSIFIED, unclassified.code()));
    }
    let counts = classified.evidence_counts();
    for (name, count_name, ids, count) in [
        (
            detail::TEMPLATE_ID,
            detail::TEMPLATE_ID_COUNT,
            classified.templates(),
            counts.templates,
        ),
        (
            detail::ARCHETYPE_ID,
            detail::ARCHETYPE_ID_COUNT,
            classified.archetypes(),
            counts.archetypes,
        ),
        (
            detail::VERSION_UID,
            detail::VERSION_UID_COUNT,
            classified.versions(),
            counts.versions,
        ),
        (
            detail::UNMAPPED_ID,
            detail::UNMAPPED_ID_COUNT,
            classified.unmapped(),
            counts.unmapped,
        ),
    ] {
        details.extend(ids.iter().map(|id| Detail::new(name, id.clone())));
        if count > ids.len() {
            details.push(Detail::new(count_name, count.to_string()));
        }
    }
    if let Some(digest) = classified.digest() {
        details.push(Detail::new(detail::MAP_DIGEST, digest));
    }
    details
}
