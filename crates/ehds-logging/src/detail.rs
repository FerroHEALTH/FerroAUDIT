// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The names a record's renderings write: the entities the BALP record adds
//! and the `detail` entries they carry, which the DICOM message writes as
//! participant objects and their `ParticipantObjectDetail`s.
//!
//! [`accessor`] and [`relayed`] give the entries of the `ehds-accessor` and
//! `ehds-relayed` entities, each value in the form both renderings write it.
//! The module needs no feature, so a recording system that writes its own
//! DICOM message names and formats the binding and the relayed members as
//! this crate does.

use std::fmt;

use crate::record::{
    Accessor, Agreement, Binding, Coded, IssuedId, PartyRef, Relayed, RelayedProfessional,
    RelayedProvider,
};

/// The name of the entity that carries how the accessor was bound to a
/// party: the binding outcome, the party and its groups, the actor chain,
/// the representation, the provider acted for and each facility.
pub const ACCESSOR_ENTITY: &str = "ehds-accessor";
/// The name of the entity that carries what a contact point or a federation
/// relayed.
pub const RELAYED_ENTITY: &str = "ehds-relayed";
/// A category as `<system>|<code>`, once per category.
pub const CATEGORY: &str = "ehds-category";
/// `<system>|<code>:<basis>`, once per basis of each category, the basis
/// after the last `:`.
pub const BASIS: &str = "ehds-category-basis";
/// `<system>|<version>`, once per code system a category is written in
/// whose version is known.
pub const CATEGORY_VERSION: &str = "ehds-category-version";
/// `true` when the access reached data of no category.
pub const NO_CATEGORY: &str = "ehds-no-category";
/// Why the access is unclassified.
pub const UNCLASSIFIED: &str = "ehds-unclassified";
/// A template id of the evidence.
pub const TEMPLATE_ID: &str = "template-id";
/// An archetype id of the evidence.
pub const ARCHETYPE_ID: &str = "archetype-id";
/// A version uid of a root object the access reached.
pub const VERSION_UID: &str = "version-uid";
/// An id the category map holds no key for.
pub const UNMAPPED_ID: &str = "unmapped-id";
/// How many distinct template ids the evidence held, when the map's evidence
/// limit cut the `template-id` entries.
pub const TEMPLATE_ID_COUNT: &str = "template-id-count";
/// How many distinct archetype ids the evidence held, when the limit cut
/// them.
pub const ARCHETYPE_ID_COUNT: &str = "archetype-id-count";
/// How many distinct version uids the evidence held, when the limit cut them.
pub const VERSION_UID_COUNT: &str = "version-uid-count";
/// How many distinct unmapped ids the evidence held, when the limit cut them.
pub const UNMAPPED_ID_COUNT: &str = "unmapped-id-count";
/// A kind of resource of no category the access reached, such as
/// `EHR_STATUS`, once per kind.
pub const NO_CATEGORY_KIND: &str = "ehds-no-category-kind";
/// What the "no category" was read from: `resource-kind`.
pub const NO_CATEGORY_BASIS: &str = "ehds-no-category-basis";
/// How many distinct origins the access reached, when the record names fewer.
pub const ORIGIN_COUNT: &str = "ehds-origin-count";
/// The digest of the category map.
pub const MAP_DIGEST: &str = "category-map-digest";
/// The rows or objects delivered.
pub const DELIVERED: &str = "delivered";
/// The stored query executed.
pub const STORED_QUERY: &str = "stored-query";
/// The node behind an origin.
pub const NODE: &str = "node";
/// The `system_id` of the node behind an origin.
pub const SYSTEM_ID: &str = "system-id";
/// How an origin answered.
pub const STATUS: &str = "status";
/// The rows an origin contributed.
pub const ROWS: &str = "rows";
/// The `OBJECT_VERSION_ID` of the version a write left at an origin, as
/// its answer named it.
pub const WRITTEN_VERSION: &str = "written-version";
/// Why the version a write left at an origin cannot be told: `unnamed`
/// when its answer named none, `malformed` when it named no
/// `OBJECT_VERSION_ID`.
pub const WRITTEN_VERSION_UNREAD: &str = "written-version-unread";
/// The `ehr_id` of the EHR a create left at an origin, as its answer
/// named it.
pub const CREATED_EHR: &str = "created-ehr";
/// Why the `ehr_id` an EHR create left at an origin cannot be told:
/// `unnamed` when its answer named none, `malformed` when it named no
/// `HIER_OBJECT_ID`, or two that differ.
pub const CREATED_EHR_UNREAD: &str = "created-ehr-unread";
/// The endpoint an `ehr_id` is held at.
pub const ENDPOINT: &str = "endpoint";
/// Whether the patient behind an `ehr_id` is named: `request-named`,
/// `found`, `not-found`, `unavailable`, `not-configured` or
/// `unsupported`.
pub const PATIENT_LOOKUP: &str = "patient-lookup";
/// The identifier the client correlates the request by in its own log.
pub const CORRELATION: &str = "correlation-id";
/// Where the correlation id was read: `token-claim`,
/// `authorization-details` or `header`.
pub const CORRELATION_SOURCE: &str = "correlation-source";
/// How the relayed accessor reached the recording system:
/// `contact-point` or `federation`.
pub const RELAYED_VIA: &str = "relayed-via";
/// The issuer of the token that relayed the professional and the provider.
pub const RELAYED_ISSUER: &str = "relayed-issuer";
/// That the relayed values are the relaying issuer's assertion: `true`.
pub const ASSERTED: &str = "asserted";
/// Whether the relayed accessor names a professional: `true`, or `false`
/// for an organisation without a named person.
pub const PROFESSIONAL_NAMED: &str = "professional-named";
/// The relayed professional's full name, the XSPA `subject-id`.
pub const HP_NAME: &str = "hp-name";
/// The relayed `healthcare_provider_identifier` of Annex Table 1.
pub const HP_PROVIDER_IDENTIFIER: &str = "hp-provider-identifier";
/// The identity assertion's `NameID` as `SPProvidedID<NameID@Issuer>`.
pub const HP_NAME_ID: &str = "hp-name-id";
/// The `AuthnContextClassRef` URI of the professional's authentication.
pub const HP_AUTHENTICATION_METHOD: &str = "hp-authentication-method";
/// `<system>|<code>` of the role the professional acts on behalf of.
pub const HP_ON_BEHALF_OF: &str = "hp-on-behalf-of";
/// An XSPA `hl7:permission` URI, once per permission.
pub const HP_PERMISSION: &str = "hp-permission";
/// `<system>|<code>` of the provider's eHDSI healthcare facility type.
pub const PROVIDER_FACILITY_TYPE: &str = "provider-facility-type";
/// The provider's XCA `homeCommunityId`.
pub const PROVIDER_HOME_COMMUNITY_ID: &str = "provider-home-community-id";
/// `<system>|<code>` of the provider's organisation type under a national
/// profile.
pub const PROVIDER_ORGANISATION_TYPE: &str = "provider-organisation-type";
/// The point of care by name, the XSPA `locality`.
pub const POINT_OF_CARE: &str = "point-of-care";
/// The `@ID` of the professional's identity assertion.
pub const IDENTITY_ASSERTION_ID: &str = "identity-assertion-id";
/// The `@ID` of the treatment relationship confirmation assertion.
pub const TRC_ASSERTION_ID: &str = "trc-assertion-id";
/// The request message's UUID, the WS-Addressing `MessageID`.
pub const REQUEST_MESSAGE_ID: &str = "request-message-id";
/// The CN of the service consumer NCP's TLS certificate.
pub const CONSUMER_NCP: &str = "consumer-ncp";
/// The CN of the service provider NCP's TLS certificate.
pub const PROVIDER_NCP: &str = "provider-ncp";
/// The name of the agreement the access was made under.
pub const AGREEMENT_NAME: &str = "agreement-name";
/// The version of that agreement.
pub const AGREEMENT_VERSION: &str = "agreement-version";
/// `<system>|<code>`, or `<code>`, once per legal ground of the agreement.
pub const AGREEMENT_LEGAL_GROUND: &str = "agreement-legal-ground";
/// The reference of the consent decision taken for the patient.
pub const AGREEMENT_CONSENT: &str = "agreement-consent";
/// The access class of an organisation without a named person:
/// `automated` or `chained`.
pub const AGREEMENT_ACCESS_CLASS: &str = "agreement-access-class";
/// The binding outcome, such as `bound` or `refused-unbound`.
pub const BINDING: &str = "binding";
/// That the bound person is a guest of the issuer: `true`.
pub const GUEST: &str = "guest";
/// That the outcome is a security event: `true`.
pub const SECURITY_EVENT: &str = "security-event";
/// The party the binding names, as `<namespace>|<type>|<id>`.
pub const PARTY: &str = "party";
/// A group the bound person belongs to, as `<namespace>|<type>|<id>`,
/// once per group.
pub const GROUP: &str = "group";
/// The claim a `refused-key-claim-missing` token lacked.
pub const BINDING_CLAIM: &str = "binding-claim";
/// A field a `refused-external-incomplete` token lacked or garbled, once
/// per field.
pub const BINDING_FIELD: &str = "binding-field";
/// One actor of the token's actor chain as `<issuer>|<subject>`, or
/// `<subject>`, outermost first.
pub const ACTOR: &str = "actor";
/// Whom a representative acts for, as `<namespace>|<type>|<id>`.
pub const REPRESENTS: &str = "represents";
/// `<system>|<code>`, or `<code>`, of the ground of the representation.
pub const REPRESENTS_GROUND: &str = "represents-ground";
/// `<system>|<code>`, or `<code>`, of the relationship the representative's
/// `PARTY_RELATED` committer records.
pub const REPRESENTS_RELATIONSHIP: &str = "represents-relationship";
/// The healthcare provider the accessor acts for, as `<issuer>|<id>`.
pub const ACTS_FOR: &str = "acts-for";
/// A facility a system client's commit names, as `<issuer>|<id>`.
pub const FACILITY: &str = "facility";
/// A further field the relaying issuer asserted, as `<name>=<value>`.
pub const ASSERTED_FIELD: &str = "asserted-field";
/// The relayed `country_code`.
pub const COUNTRY_CODE: &str = "country-code";
/// The relayed professional's `family_name`.
pub const FAMILY_NAME: &str = "hp-family-name";
/// The relayed professional's `given_name`.
pub const GIVEN_NAME: &str = "hp-given-name";
/// The relayed professional's `hp_identifier`.
pub const HP_IDENTIFIER: &str = "hp-identifier";
/// The agency that issued the relayed `hp_identifier`.
pub const HP_ISSUING_AUTHORITY: &str = "hp-issuing-authority";
/// `<system>|<code>`, or `<code>`, once per relayed
/// `hp_professional_role`.
pub const HP_ROLE: &str = "hp-professional-role";
/// The relayed `healthcare_provider_identifier`.
pub const PROVIDER_IDENTIFIER: &str = "provider-identifier";
/// The agency that issued the relayed provider identifier.
pub const PROVIDER_ISSUING_AUTHORITY: &str = "provider-issuing-authority";
/// The relayed `healthcare_provider_name`.
pub const PROVIDER_NAME: &str = "provider-name";
/// The relayed `healthcare_provider_address`.
pub const PROVIDER_ADDRESS: &str = "provider-address";
/// The years the record is kept, for a period in years.
pub const RETENTION_YEARS: &str = "ehds-retention-years";
/// The period the record is kept, as an ISO 8601 duration (`P3Y`, `P1100D`),
/// or `indefinite`.
pub const RETENTION_PERIOD: &str = "ehds-retention-period";
/// The legal ground of the floor the period stands over:
/// `eu-2025-327-art-9-2`, `national:<citation>` or
/// `outside-union:<citation>`.
pub const RETENTION_FLOOR: &str = "ehds-retention-floor";
/// The first UTC date the record may be deleted on, `YYYY-MM-DD`; absent for
/// a record kept without end.
pub const RETENTION_ENDS: &str = "ehds-retention-ends";
/// What called for the period: `default`, `unclassified`,
/// `category:<system>|<code>` or `origin:<endpoint>`.
pub const RETENTION_GROUND: &str = "ehds-retention-ground";
/// That the access is an emergency access (Art 11(5)): `true`.
pub const EMERGENCY_ACCESS: &str = "ehds-emergency-access";
/// `<system>|<code>`, or `<code>`, once per purpose of use that marked
/// the access an emergency access.
pub const EMERGENCY_PURPOSE: &str = "ehds-emergency-purpose";
/// A member whose consent pre-filter denial was set aside for the
/// emergency, once per member.
pub const CONSENT_SET_ASIDE: &str = "ehds-consent-set-aside";
/// That the access is a population access: `true`.
pub const POPULATION: &str = "ehds-population";
/// Why a population access's record names no `ehr_id`: `not-projected`,
/// `over-limit`, `not-delivered` or `unattributed`.
pub const POPULATION_REASON: &str = "ehds-population-reason";
/// How many `ehr_id`s the delivered rows of a population access named.
pub const POPULATION_EHRS: &str = "ehds-population-ehrs";
/// The most `ehr_id`s a record names.
pub const POPULATION_LIMIT: &str = "ehds-population-limit";
/// Which way a semantic transformation ran: `provided` or `received`.
pub const TRANSFORMATION_DIRECTION: &str = "ehds-transformation-direction";
/// The Art 14(1) category of the data a transformation transformed, as
/// `<system>|<code>`.
pub const TRANSFORMATION_CATEGORY: &str = "ehds-transformation-category";
/// The FHIRconnect context mapping a transformation ran.
pub const MAPPING_CONTEXT: &str = "mapping-context";
/// The openEHR template id a transformation's mapping maps.
pub const MAPPING_TEMPLATE: &str = "mapping-template";
/// The profile a transformation's mapping maps to.
pub const MAPPING_PROFILE: &str = "mapping-profile";
/// A file name of a transformation's model or context mappings, once per
/// file.
pub const MAPPING_FILE: &str = "mapping-file";
/// The FHIR package a received document was held to.
pub const MAPPING_PACKAGE: &str = "mapping-package";
/// A terminology release or code system version in force, as
/// `<name>|<version>`, once per version.
pub const TERMINOLOGY_VERSION: &str = "terminology-version";
/// How many coded elements a transformation carried in a preferred code
/// system.
pub const TRANSCODED: &str = "ehds-transcoded";
/// `<path>=<count>`, once per path of a transcoded element.
pub const TRANSCODED_PATH: &str = "ehds-transcoded-path";
/// How many coded elements a transformation carried in no preferred code
/// system (eHealth Network Guidelines on Patient Summary Art 10(7)).
pub const UNTRANSCODED: &str = "ehds-untranscoded";
/// `<path>=<count>`, once per path of an untranscoded element.
pub const UNTRANSCODED_PATH: &str = "ehds-untranscoded-path";
/// How many elements a transformation did not carry.
pub const DROPPED: &str = "ehds-dropped";
/// `<path>=<count>`, once per path of a dropped element that names one.
pub const DROPPED_PATH: &str = "ehds-dropped-path";

/// One `detail` entry of an entity: its name and its value.
///
/// `Debug` shows the name and no value.
#[derive(Clone, PartialEq, Eq)]
pub struct Entry {
    /// The name, one of this module's constants.
    pub name: &'static str,
    /// The value.
    pub value: String,
}

impl Entry {
    /// The entry `name` with the value `value`.
    #[must_use]
    pub fn new(name: &'static str, value: impl Into<String>) -> Self {
        Self {
            name,
            value: value.into(),
        }
    }
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

/// `<system>|<code>`, or `<code>` when no system is named.
pub(crate) fn coded(system: Option<&str>, code: &str) -> String {
    match system {
        Some(system) => format!("{system}|{code}"),
        None => code.to_owned(),
    }
}

/// `<system>|<code>` of a record's coded value, or `<code>`.
fn record_coded(value: &Coded) -> String {
    coded(value.system.as_deref(), &value.code)
}

/// `<namespace>|<type>|<id>` of `party`.
fn party_ref(party: &PartyRef) -> String {
    format!("{}|{}|{}", party.namespace, party.kind, party.id)
}

/// The entries of the relayed entity ([`RELAYED_ENTITY`]) of `relayed`.
///
/// They are marked as the relaying issuer's assertion, in the order the BALP
/// record writes them.
#[must_use]
pub fn relayed(relayed: &Relayed) -> Vec<Entry> {
    let parts = relayed.as_parts();
    let mut details = vec![
        Entry::new(RELAYED_VIA, parts.via.code()),
        Entry::new(RELAYED_ISSUER, parts.via.issuer()),
        Entry::new(ASSERTED, "true"),
    ];
    details.extend(optional(COUNTRY_CODE, parts.country_code.as_ref()));
    details.push(Entry::new(
        PROFESSIONAL_NAMED,
        if relayed.names_a_professional() {
            "true"
        } else {
            "false"
        },
    ));
    if let Some(professional) = &parts.professional {
        details.extend(professional_details(professional));
    }
    if let Some(provider) = &parts.provider {
        details.extend(provider_details(provider));
    }
    for (name, value) in [
        (POINT_OF_CARE, &parts.point_of_care),
        (IDENTITY_ASSERTION_ID, &parts.identity_assertion_id),
        (TRC_ASSERTION_ID, &parts.trc_assertion_id),
        (REQUEST_MESSAGE_ID, &parts.message_id),
        (CONSUMER_NCP, &parts.gateways.consumer),
        (PROVIDER_NCP, &parts.gateways.provider),
    ] {
        details.extend(optional(name, value.as_ref()));
    }
    if let Some(agreement) = &parts.agreement {
        details.extend(agreement_details(agreement));
    }
    details.extend(parts.asserted.iter().flat_map(|(name, values)| {
        values
            .iter()
            .map(move |value| Entry::new(ASSERTED_FIELD, format!("{name}={value}")))
    }));
    details
}

/// The `detail` entries of a relayed provider (2026/2099 Annex Table 2, eHDSI
/// SAML Profile 10.0.0 §2.3).
fn provider_details(provider: &RelayedProvider) -> Vec<Entry> {
    let mut details = vec![Entry::new(PROVIDER_IDENTIFIER, provider.identifier.clone())];
    details.extend(optional(
        PROVIDER_ISSUING_AUTHORITY,
        provider.issuing_authority.as_ref(),
    ));
    details.extend(optional(PROVIDER_NAME, provider.name.as_ref()));
    details.extend(optional(PROVIDER_ADDRESS, provider.address.as_ref()));
    details.extend(
        provider
            .facility_type
            .as_ref()
            .map(|kind| Entry::new(PROVIDER_FACILITY_TYPE, record_coded(kind))),
    );
    details.extend(optional(
        PROVIDER_HOME_COMMUNITY_ID,
        provider.home_community_id.as_ref(),
    ));
    details.extend(
        provider
            .organisation_type
            .as_ref()
            .map(|kind| Entry::new(PROVIDER_ORGANISATION_TYPE, record_coded(kind))),
    );
    details
}

/// The `detail` entries of the agreement a relayed access is made under.
fn agreement_details(agreement: &Agreement) -> Vec<Entry> {
    let mut details = vec![
        Entry::new(AGREEMENT_NAME, agreement.name.clone()),
        Entry::new(AGREEMENT_VERSION, agreement.version.clone()),
    ];
    details.extend(
        agreement
            .legal_ground
            .iter()
            .map(|ground| Entry::new(AGREEMENT_LEGAL_GROUND, record_coded(ground))),
    );
    details.extend(optional(AGREEMENT_CONSENT, agreement.consent.as_ref()));
    details.extend(
        agreement
            .access_class
            .map(|class| Entry::new(AGREEMENT_ACCESS_CLASS, class.code())),
    );
    details
}

/// `value` as the detail `name`, when there is one.
fn optional(name: &'static str, value: Option<&String>) -> Option<Entry> {
    value.map(|value| Entry::new(name, value.clone()))
}

/// The `detail` entries of a relayed professional (2026/2099 Annex Table 1,
/// eHDSI SAML Profile 10.0.0 §2.3).
fn professional_details(professional: &RelayedProfessional) -> Vec<Entry> {
    let mut details = Vec::new();
    details.extend(optional(HP_NAME, professional.name.as_ref()));
    details.extend(optional(FAMILY_NAME, professional.family_name.as_ref()));
    details.extend(optional(GIVEN_NAME, professional.given_name.as_ref()));
    details.extend(optional(HP_IDENTIFIER, professional.identifier.as_ref()));
    details.extend(optional(
        HP_ISSUING_AUTHORITY,
        professional.issuing_authority.as_ref(),
    ));
    details.extend(
        professional
            .roles
            .iter()
            .map(|role| Entry::new(HP_ROLE, record_coded(role))),
    );
    details.extend(optional(
        HP_PROVIDER_IDENTIFIER,
        professional.healthcare_provider_identifier.as_ref(),
    ));
    details.extend(
        professional
            .name_id
            .as_ref()
            .map(|name_id| Entry::new(HP_NAME_ID, name_id.user_id())),
    );
    details.extend(
        professional
            .authentication_method
            .map(|method| Entry::new(HP_AUTHENTICATION_METHOD, method.uri())),
    );
    details.extend(
        professional
            .on_behalf_of
            .as_ref()
            .map(|role| Entry::new(HP_ON_BEHALF_OF, record_coded(role))),
    );
    details.extend(
        professional
            .permissions
            .iter()
            .map(|permission| Entry::new(HP_PERMISSION, permission.clone())),
    );
    details
}

/// The entries of the accessor entity ([`ACCESSOR_ENTITY`]) of `accessor`.
///
/// They are the binding outcome with what it names, the actor chain
/// outermost first, the representation, the provider acted for and each
/// facility, in the order the BALP record writes them; empty when the
/// accessor carries none of them.
#[must_use]
pub fn accessor(accessor: &Accessor) -> Vec<Entry> {
    let mut details = Vec::new();
    if let Some(binding) = &accessor.binding {
        details.push(Entry::new(BINDING, binding.code()));
        if binding.is_guest() {
            details.push(Entry::new(GUEST, "true"));
        }
        if binding.is_security_event() {
            details.push(Entry::new(SECURITY_EVENT, "true"));
        }
        details.extend(
            binding
                .party()
                .map(|party| Entry::new(PARTY, party_ref(party))),
        );
        details.extend(
            binding
                .groups()
                .iter()
                .map(|group| Entry::new(GROUP, party_ref(group))),
        );
        match binding {
            Binding::RefusedKeyClaimMissing { claim } => {
                details.push(Entry::new(BINDING_CLAIM, claim.clone()));
            }
            Binding::RefusedExternalIncomplete { fields } => details.extend(
                fields
                    .iter()
                    .map(|field| Entry::new(BINDING_FIELD, field.clone())),
            ),
            _ => {}
        }
    }
    details.extend(
        accessor
            .actors
            .iter()
            .map(|actor| Entry::new(ACTOR, coded(actor.issuer.as_deref(), &actor.subject))),
    );
    if let Some(representation) = &accessor.represents {
        details.push(Entry::new(REPRESENTS, representation.represented.clone()));
        details.extend(
            representation
                .ground
                .as_ref()
                .map(|ground| Entry::new(REPRESENTS_GROUND, record_coded(ground))),
        );
        details.extend(
            representation.relationship.as_ref().map(|relationship| {
                Entry::new(REPRESENTS_RELATIONSHIP, record_coded(relationship))
            }),
        );
    }
    details.extend(
        accessor
            .acts_for
            .iter()
            .map(|provider| Entry::new(ACTS_FOR, issued(provider))),
    );
    details.extend(
        accessor
            .facilities
            .iter()
            .map(|facility| Entry::new(FACILITY, issued(facility))),
    );
    details
}

/// `id` as `<issuer>|<id>`.
fn issued(id: &IssuedId) -> String {
    format!("{}|{}", id.issuer, id.id)
}
