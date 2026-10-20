// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The access record: what the logging component records "on every access
//! event or group of events" (Regulation (EU) 2025/327 Annex II 3.2).
//!
//! Annex II 3.2 asks for (a) the healthcare provider or other individuals
//! that accessed the data, (b) the specific natural person or persons, (c)
//! the categories of data accessed, (d) the time and date, and (e) the
//! origin or origins of the data. An [`AccessRecord`] carries each, and
//! beside them the data subject whose data were accessed and the purpose of
//! use the access was made for, which Art 9(2) asks the patient be told. A
//! record whose accessor declared an emergency purpose carries the mark
//! Art 11(5) asks for ([`emergency`](crate::emergency)).
//!
//! A record names the patient and the person who accessed their data, so
//! `Debug` shows neither, nor any id the classification read: a record
//! leaves through an [`AccessSink`](crate::sink::AccessSink) alone.

use std::collections::BTreeSet;
use std::fmt;
use std::net::IpAddr;

use jiff::Timestamp;
use secrecy::SecretString;

use crate::classify::Classification;
use crate::emergency::Emergency;
use crate::label::Labels;
use crate::retention::Retention;
use crate::transformation::Transformation;

/// What the access did to the data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Action {
    /// A query over the data, an ad hoc or a stored one.
    Query,
    /// A read of one resource.
    Read,
    /// The creation of a resource.
    Create,
    /// The update of a resource.
    Update,
    /// The deletion of a resource.
    Delete,
}

/// How the access ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Outcome {
    /// The data were delivered or written as asked.
    Success,
    /// The access was answered with a failure, or with less than asked.
    MinorFailure,
    /// The access failed on the side that records it.
    SeriousFailure,
    /// The access failed and left the recording system unavailable.
    MajorFailure,
}

/// One purpose of use the accessor declared: a code and the system that
/// defines it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Purpose {
    /// The code system, when one is named.
    pub system: Option<String>,
    /// The code.
    pub code: String,
}

/// Who acts behind the token an access was authenticated by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Acting {
    /// The natural person the token's `sub` names.
    Person,
    /// A client application, which the recording system admits to personal
    /// data only for the professional the token names.
    Client,
}

impl Acting {
    /// The code a record writes the acting mode as: `person` or `client`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Client => "client",
        }
    }
}

/// An assurance level of an electronic identification means, as Regulation
/// (EU) No 910/2014 Art 8(2) names them, lowest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AssuranceLevel {
    /// Level low, Art 8(2)(a).
    Low,
    /// Level substantial, Art 8(2)(b).
    Substantial,
    /// Level high, Art 8(2)(c).
    High,
}

impl AssuranceLevel {
    /// The code a record writes the level as: `low`, `substantial` or
    /// `high`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Substantial => "substantial",
            Self::High => "high",
        }
    }
}

/// The professional's identification, as the token the access was
/// authenticated by states it.
///
/// `Debug` shows which members are present and none of their values.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Professional {
    /// The professional's name.
    pub name: Option<String>,
    /// The identifier their national authority issued them as a health
    /// professional.
    pub identifier: Option<String>,
}

impl fmt::Debug for Professional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Professional")
            .field("name", &self.name.is_some())
            .field("identifier", &self.identifier.is_some())
            .finish()
    }
}

/// A coded value, such as a professional role: a code and the system that
/// defines it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Coded {
    /// The code system, when one is named.
    pub system: Option<String>,
    /// The code.
    pub code: String,
}

/// What a national contact point for digital health relays with a request.
///
/// The health professional and the healthcare provider of another Member
/// State, as the contact point asserts them (Implementing Regulation (EU)
/// 2026/2099 Art 7 and its Annex Tables 1 and 2). The recording system verified the contact point, never the professional:
/// Art 6(1) and (2) give their identification and authentication to the
/// entity the Member State of the professional lists. `Debug` shows the
/// country and none of the other values.
#[derive(Clone, PartialEq, Eq)]
pub struct Relayed {
    /// The contact point that asserted them, by the issuer of its token.
    pub contact_point: String,
    /// `country_code`: the ISO 3166-1 alpha-2 code of the Member State that
    /// issued the professional's identification data.
    pub country_code: String,
    /// The health professional (Annex Table 1).
    pub professional: RelayedProfessional,
    /// The healthcare provider (Annex Table 2).
    pub provider: RelayedProvider,
}

impl fmt::Debug for Relayed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Relayed")
            .field("country_code", &self.country_code)
            .finish_non_exhaustive()
    }
}

/// The health professional a contact point relays (2026/2099 Annex
/// Table 1).
#[derive(Clone, PartialEq, Eq)]
pub struct RelayedProfessional {
    /// `family_name`.
    pub family_name: String,
    /// `given_name`.
    pub given_name: String,
    /// `hp_identifier`, unique among the identifiers the Member State where
    /// the professional is registered issues.
    pub identifier: String,
    /// `issuing_authority_name`: the agency that issued `hp_identifier`.
    pub issuing_authority: String,
    /// `hp_professional_role`, as the codes the contact point states.
    pub roles: Vec<Coded>,
}

impl fmt::Debug for RelayedProfessional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RelayedProfessional")
            .field("roles", &self.roles.len())
            .finish_non_exhaustive()
    }
}

/// The healthcare provider a contact point relays (2026/2099 Annex
/// Table 2), where the professional provides the treatment.
#[derive(Clone, PartialEq, Eq)]
pub struct RelayedProvider {
    /// `healthcare_provider_identifier`, unique in the Member State that
    /// issued it.
    pub identifier: String,
    /// `issuing_authority_name`: the agency that issued the identifier.
    pub issuing_authority: String,
    /// `healthcare_provider_name`.
    pub name: String,
    /// `healthcare_provider_address`: the official registered full address.
    pub address: String,
}

impl fmt::Debug for RelayedProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RelayedProvider").finish_non_exhaustive()
    }
}

/// Who accessed the data (Annex II 3.2(a), (b)).
///
/// The person, as the token the access was authenticated by names them, or
/// as a local account names them when no token did, the client they used,
/// and the provider they act for.
///
/// Each value is the authentication's, as the recording system verified it:
/// the record infers none, so the issuer, the client, the acting mode and the
/// assurance level are absent unless the authentication established them.
#[derive(Clone, PartialEq, Eq)]
pub struct Accessor {
    /// The issuer that vouched for the person (`iss`), when a token named
    /// them; `None` for a user authenticated otherwise, such as by HTTP
    /// Basic.
    pub issuer: Option<String>,
    /// The person (`sub`, or the local account).
    pub subject: String,
    /// The client application they used (`client_id`), when a token named
    /// one.
    pub client_id: Option<String>,
    /// The audience the token named the recording system by.
    pub audience: Option<String>,
    /// The healthcare provider or other organisation the person acts for.
    pub provider: Option<String>,
    /// Who acts behind the token: the person, or a client application, when
    /// the recording system established it.
    pub acting: Option<Acting>,
    /// The roles or authorities the person acted under, such as the security
    /// roles of the recording system, which the record writes at
    /// `agent[user].role` (BALP 1.1.4 §3:5.7.5.4, the IUA `subject_role`).
    pub roles: Vec<Coded>,
    /// The identity of the access token (`jti`), when it names one.
    pub token_id: Option<String>,
    /// The assurance level the authentication reached, when the recording
    /// system established one from the token.
    pub assurance: Option<AssuranceLevel>,
    /// The professional the token names (3.2(b)), who acts in person or
    /// whom a client acts for.
    pub professional: Professional,
    /// Another identification of the person the token states beside its
    /// `sub`, such as a professional number a claim of the deployment's
    /// naming carries.
    pub alt_id: Option<String>,
    /// Every purpose of use the token declares.
    pub purposes: Vec<Purpose>,
    /// The professional and the provider of another Member State a national
    /// contact point relays, marked as asserted by it, when the client is
    /// one (Implementing Regulation (EU) 2026/2099 Art 7).
    pub relayed: Option<Relayed>,
}

impl fmt::Debug for Accessor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Accessor")
            .field("provider", &self.provider.is_some())
            .field("acting", &self.acting)
            .field("roles", &self.roles)
            .field("token_id", &self.token_id.is_some())
            .field("assurance", &self.assurance)
            .field("professional", &self.professional)
            .field("alt_id", &self.alt_id.is_some())
            .field("purposes", &self.purposes)
            .field("relayed", &self.relayed)
            .finish_non_exhaustive()
    }
}

/// The patient an access concerned, by the identifier the request named and
/// the namespace that issued it; `Debug` shows the namespace alone.
#[derive(Debug, Clone)]
pub struct PatientIdentifier {
    /// The issuing namespace, when one is known: a URI, which the record
    /// writes as the identifier's system, or another name, such as an openEHR
    /// `PARTY_REF.namespace`, which it writes beside the identifier.
    pub namespace: Option<String>,
    /// The identifier.
    pub value: SecretString,
}

/// Whether the patient behind an `ehr_id` is named, so a search by the
/// patient's identifier finds the access (Art 9(1)).
///
/// `Debug` shows the outcome and how many identifiers were found, never one.
#[derive(Clone)]
#[non_exhaustive]
pub enum PatientLookup {
    /// The request named the patient, so the record names them already.
    RequestNamed,
    /// The identity service holds these identifiers for the patient.
    Found(Vec<PatientIdentifier>),
    /// The identity service holds no identifier for the patient in a
    /// namespace asked.
    NotFound,
    /// The identity service could not be asked, or did not answer.
    Unavailable,
    /// The recording system names no namespace to ask in.
    NotConfigured,
    /// The recording system has no identity service that names a patient
    /// by an `ehr_id`.
    Unsupported,
}

impl PatientLookup {
    /// The outcome as a record writes it: `request-named`, `found`,
    /// `not-found`, `unavailable`, `not-configured` or `unsupported`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::RequestNamed => "request-named",
            Self::Found(_) => "found",
            Self::NotFound => "not-found",
            Self::Unavailable => "unavailable",
            Self::NotConfigured => "not-configured",
            Self::Unsupported => "unsupported",
        }
    }
}

impl fmt::Debug for PatientLookup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Found(found) => write!(f, "Found({})", found.len()),
            other => f.write_str(other.code()),
        }
    }
}

/// One `ehr_id` an access reached, at the endpoint that holds it, with
/// whether the patient it belongs to is named.
#[derive(Clone)]
pub struct EhrAt {
    /// The endpoint.
    pub endpoint: String,
    /// The `ehr_id` at that endpoint.
    pub ehr_id: String,
    /// Whether the patient behind the `ehr_id` is named.
    pub patient: PatientLookup,
}

impl fmt::Debug for EhrAt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EhrAt")
            .field("endpoint", &self.endpoint)
            .field("patient", &self.patient)
            .finish_non_exhaustive()
    }
}

/// Whose data an access concerned: the patient the request named, each
/// `ehr_id` the access reached, and, for an access over many patients' data
/// that names none of them, the scope it read.
#[derive(Clone, Default)]
pub struct DataSubject {
    /// The patient, when the request named one.
    pub patient: Option<PatientIdentifier>,
    /// The `ehr_id`s the access reached.
    pub ehrs: Vec<EhrAt>,
    /// The scope of an access over many patients' data whose record names
    /// none of them: a population access.
    pub population: Option<Population>,
}

impl fmt::Debug for DataSubject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataSubject")
            .field("patient", &self.patient.is_some())
            .field("ehrs", &self.ehrs.len())
            .field("population", &self.population)
            .finish()
    }
}

/// Why the record of an access over many patients' data names none of the
/// `ehr_id`s whose records it read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PopulationReason {
    /// The query projects no `ehr_id`, so no delivered row says whose
    /// record it came from.
    NotProjected,
    /// The delivered rows name more `ehr_id`s than a record names.
    OverLimit,
    /// No row was delivered, so no row names an `ehr_id`.
    NotDelivered,
    /// A delivered `ehr_id` was in no origin's own answer, so the record
    /// cannot say which origin holds it.
    Unattributed,
}

impl PopulationReason {
    /// The reason as a record writes it: `not-projected`, `over-limit`,
    /// `not-delivered` or `unattributed`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotProjected => "not-projected",
            Self::OverLimit => "over-limit",
            Self::NotDelivered => "not-delivered",
            Self::Unattributed => "unattributed",
        }
    }
}

/// The scope of a population access: what the access read, when the record
/// names none of the persons whose records it read.
///
/// The origins the access asked are the record's origins. A person's
/// search finds the record through an origin that holds their record, and
/// it says that a query over that origin ran, never that it read their
/// record. `Debug` shows the reason and the counts, never an id.
#[derive(Clone, PartialEq, Eq)]
pub struct Population {
    /// Why the record names no `ehr_id`.
    pub reason: PopulationReason,
    /// How many `ehr_id`s the delivered rows named, when they were read.
    pub ehrs: Option<usize>,
    /// The most `ehr_id`s a record names.
    pub limit: usize,
    /// The template ids the query constrains its data to.
    pub templates: BTreeSet<String>,
    /// The archetype ids the query constrains its data to.
    pub archetypes: BTreeSet<String>,
}

impl fmt::Debug for Population {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Population")
            .field("reason", &self.reason)
            .field("ehrs", &self.ehrs)
            .field("limit", &self.limit)
            .field("templates", &self.templates.len())
            .field("archetypes", &self.archetypes.len())
            .finish()
    }
}

/// One origin of the data (Annex II 3.2(e)).
///
/// An endpoint the access asked, with how it answered, or the system stored
/// data came from, such as the originating system an openEHR `FEEDER_AUDIT`
/// names, or both.
#[derive(Clone, PartialEq, Eq)]
pub struct Origin {
    /// The endpoint the access asked, as the registry names it, when it
    /// asked one.
    pub endpoint: Option<String>,
    /// The node behind it.
    pub node: Option<String>,
    /// The openEHR `system_id` of the node, or of the system stored data
    /// came from.
    pub system_id: Option<String>,
    /// How the endpoint answered, such as `active` or `node-error`, when an
    /// endpoint was asked.
    pub status: Option<String>,
    /// The rows or objects it contributed, when counted.
    pub rows: Option<usize>,
    /// The version a write left at the endpoint, as its answer named it, or
    /// `None` for an access that wrote nothing there.
    pub version: Option<WrittenVersion>,
    /// The `ehr_id` an EHR create left at the endpoint, as its answer named
    /// it, or `None` for an access that created no EHR there.
    pub created_ehr: Option<CreatedEhr>,
    /// The categories of what it contributed, when told apart from the
    /// other origins'.
    pub categories: Option<Classification>,
}

impl Origin {
    /// What the origin is keyed by for retention and labels: its endpoint,
    /// or its `system_id` when it names no endpoint.
    #[must_use]
    pub fn key(&self) -> Option<&str> {
        self.endpoint.as_deref().or(self.system_id.as_deref())
    }
}

impl fmt::Debug for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Origin")
            .field("endpoint", &self.endpoint)
            .field("node", &self.node)
            .field("status", &self.status)
            .field("rows", &self.rows)
            .field("version", &self.version)
            .field("created_ehr", &self.created_ehr)
            .field("categories", &self.categories)
            .finish_non_exhaustive()
    }
}

/// The version a write left at its origin, which is the data a write
/// accessed (Annex II 3.2), as the origin's answer named it.
///
/// The recording system holds the version id to the `OBJECT_VERSION_ID`
/// form before it is recorded, so a value the answer carried in its place,
/// whatever it holds, never reaches the record.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum WrittenVersion {
    /// The answer named this `OBJECT_VERSION_ID`.
    Named(String),
    /// The answer named no version, so the record cannot tell which one the
    /// write left.
    Unnamed,
    /// The answer named a version that is no `OBJECT_VERSION_ID`, so the
    /// record cannot tell which one the write left.
    Malformed,
}

impl WrittenVersion {
    /// The version id, when the answer named one.
    #[must_use]
    pub fn named(&self) -> Option<&str> {
        match self {
            Self::Named(version) => Some(version),
            Self::Unnamed | Self::Malformed => None,
        }
    }

    /// Why the record cannot tell the version, as a record writes it:
    /// `unnamed` or `malformed`, or `None` when the answer named it.
    #[must_use]
    pub fn unread(&self) -> Option<&'static str> {
        match self {
            Self::Named(_) => None,
            Self::Unnamed => Some("unnamed"),
            Self::Malformed => Some("malformed"),
        }
    }
}

impl fmt::Debug for WrittenVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(_) => f.write_str("Named(..)"),
            Self::Unnamed => f.write_str("Unnamed"),
            Self::Malformed => f.write_str("Malformed"),
        }
    }
}

/// The `ehr_id` of the EHR a create left at its origin, as the origin's
/// answer named it.
///
/// The recording system holds the `ehr_id` to the `HIER_OBJECT_ID` form
/// before it is recorded, so a value the answer carried in its place,
/// whatever it holds, never reaches the record.
#[derive(Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CreatedEhr {
    /// The answer named this `ehr_id`.
    Named(String),
    /// The answer named no `ehr_id`, so the record cannot tell which EHR the
    /// create left.
    Unnamed,
    /// The answer named an `ehr_id` that is no `HIER_OBJECT_ID`, or two
    /// that differ, so the record cannot tell which EHR the create left.
    Malformed,
}

impl CreatedEhr {
    /// The `ehr_id`, when the answer named one.
    #[must_use]
    pub fn named(&self) -> Option<&str> {
        match self {
            Self::Named(ehr_id) => Some(ehr_id),
            Self::Unnamed | Self::Malformed => None,
        }
    }

    /// Why the record cannot tell the `ehr_id`, as a record writes it:
    /// `unnamed` or `malformed`, or `None` when the answer named it.
    #[must_use]
    pub fn unread(&self) -> Option<&'static str> {
        match self {
            Self::Named(_) => None,
            Self::Unnamed => Some("unnamed"),
            Self::Malformed => Some("malformed"),
        }
    }
}

impl fmt::Debug for CreatedEhr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Named(_) => f.write_str("Named(..)"),
            Self::Unnamed => f.write_str("Unnamed"),
            Self::Malformed => f.write_str("Malformed"),
        }
    }
}

/// What the access asked for.
#[derive(Clone)]
pub struct Request {
    /// The id the recording system knows the request by, which its own log
    /// names too, when the request carried one.
    pub id: Option<String>,
    /// The operation, such as an ITS-REST `operationId`.
    pub operation: String,
    /// The code system the operation is a code of, such as the system of the
    /// ITS-REST operation ids: when it is named, the record writes the
    /// operation as a `subtype` in it.
    pub operation_system: Option<String>,
    /// The resource the request addressed, when it addressed one.
    pub resource: Option<String>,
    /// The query text, for a query.
    pub query: Option<SecretString>,
    /// The name of the stored query, for a stored-query execution.
    pub stored_query: Option<String>,
    /// The address the request came from, when known.
    pub client_address: Option<IpAddr>,
    /// The identifier the client correlates the request by in its own
    /// log, such as a national contact point's exchange log, when it sent
    /// one.
    pub correlation: Option<String>,
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("id", &self.id)
            .field("operation", &self.operation)
            .field("query", &self.query.is_some())
            .field("correlation", &self.correlation.is_some())
            .finish_non_exhaustive()
    }
}

/// One access event (Annex II 3.2).
#[derive(Clone)]
pub struct AccessRecord {
    /// What the access did.
    pub action: Action,
    /// When it ended (3.2(d)).
    pub recorded: Timestamp,
    /// How it ended.
    pub outcome: Outcome,
    /// Free text on how it ended, when the recording system gives one.
    pub outcome_description: Option<String>,
    /// Who accessed the data (3.2(a), (b)), with their purposes of use.
    pub accessor: Accessor,
    /// Whose data.
    pub subject: DataSubject,
    /// The categories of the data accessed (3.2(c)).
    pub categories: Classification,
    /// The rows or objects delivered to the accessor, when counted.
    pub delivered: Option<usize>,
    /// The origins of the data (3.2(e)), in the order the access asked them.
    pub origins: Vec<Origin>,
    /// How many distinct origins the access reached, when `origins` names
    /// fewer, such as when the recording system bounds one record's origins.
    pub origin_count: Option<usize>,
    /// What the access asked for.
    pub request: Request,
    /// How long the record is kept (Art 9(2), Annex II 3.4).
    pub retention: Retention,
    /// The security labels the record carries for its categories and its
    /// origins, so the store that holds it can limit who reads it (Annex II
    /// 3.4).
    pub labels: Labels,
    /// The mark of an access the accessor declared, through its purpose of
    /// use, as made to protect the vital interests of the data subject (Art
    /// 11(5)), when it did.
    pub emergency: Option<Emergency>,
    /// Who inserted the information the access wrote under the data
    /// subject's own grant, which the written data marks as inserted by
    /// them (Art 5), when it did.
    pub inserted: Option<Insertion>,
    /// Each semantic transformation the access applied between openEHR and
    /// the exchange format (eHealth Network General Guidelines Release 3.1
    /// Art 6(3)), in the order it applied them.
    pub transformations: Vec<Transformation>,
}

/// Who inserted the information a write carried under the data subject's
/// own grant, as the written data marks it (Art 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Insertion {
    /// The data subject, in person.
    Subject,
    /// A natural person who acts for the data subject, a proxy or a legal
    /// representative (Art 4(2)), whom the written data names.
    Representative,
}

impl fmt::Debug for AccessRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AccessRecord")
            .field("action", &self.action)
            .field("recorded", &self.recorded)
            .field("outcome", &self.outcome)
            .field("outcome_description", &self.outcome_description.is_some())
            .field("accessor", &self.accessor)
            .field("subject", &self.subject)
            .field("categories", &self.categories)
            .field("delivered", &self.delivered)
            .field("origins", &self.origins)
            .field("origin_count", &self.origin_count)
            .field("request", &self.request)
            .field("retention", &self.retention)
            .field("labels", &self.labels)
            .field("emergency", &self.emergency)
            .field("inserted", &self.inserted)
            .field("transformations", &self.transformations)
            .finish()
    }
}
