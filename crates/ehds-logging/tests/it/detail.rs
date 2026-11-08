// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The names both renderings write, reached with no feature on: a recording
//! system that writes its own DICOM message names the binding and the
//! relayed members as the crate does (FerroHEALTH/FerroAUDIT#161).

use ehds_logging::detail;

/// Every name the module defines, with its spelling on the wire.
const NAMES: [(&str, &str); 107] = [
    (detail::ACCESSOR_ENTITY, "ehds-accessor"),
    (detail::RELAYED_ENTITY, "ehds-relayed"),
    (detail::CATEGORY, "ehds-category"),
    (detail::BASIS, "ehds-category-basis"),
    (detail::CATEGORY_VERSION, "ehds-category-version"),
    (detail::NO_CATEGORY, "ehds-no-category"),
    (detail::UNCLASSIFIED, "ehds-unclassified"),
    (detail::TEMPLATE_ID, "template-id"),
    (detail::ARCHETYPE_ID, "archetype-id"),
    (detail::VERSION_UID, "version-uid"),
    (detail::UNMAPPED_ID, "unmapped-id"),
    (detail::TEMPLATE_ID_COUNT, "template-id-count"),
    (detail::ARCHETYPE_ID_COUNT, "archetype-id-count"),
    (detail::VERSION_UID_COUNT, "version-uid-count"),
    (detail::UNMAPPED_ID_COUNT, "unmapped-id-count"),
    (detail::NO_CATEGORY_KIND, "ehds-no-category-kind"),
    (detail::NO_CATEGORY_BASIS, "ehds-no-category-basis"),
    (detail::ORIGIN_COUNT, "ehds-origin-count"),
    (detail::MAP_DIGEST, "category-map-digest"),
    (detail::DELIVERED, "delivered"),
    (detail::STORED_QUERY, "stored-query"),
    (detail::NODE, "node"),
    (detail::SYSTEM_ID, "system-id"),
    (detail::STATUS, "status"),
    (detail::ROWS, "rows"),
    (detail::WRITTEN_VERSION, "written-version"),
    (detail::WRITTEN_VERSION_UNREAD, "written-version-unread"),
    (detail::CREATED_EHR, "created-ehr"),
    (detail::CREATED_EHR_UNREAD, "created-ehr-unread"),
    (detail::ENDPOINT, "endpoint"),
    (detail::PATIENT_LOOKUP, "patient-lookup"),
    (detail::CORRELATION, "correlation-id"),
    (detail::CORRELATION_SOURCE, "correlation-source"),
    (detail::RELAYED_VIA, "relayed-via"),
    (detail::RELAYED_ISSUER, "relayed-issuer"),
    (detail::ASSERTED, "asserted"),
    (detail::PROFESSIONAL_NAMED, "professional-named"),
    (detail::HP_NAME, "hp-name"),
    (detail::HP_PROVIDER_IDENTIFIER, "hp-provider-identifier"),
    (detail::HP_NAME_ID, "hp-name-id"),
    (detail::HP_AUTHENTICATION_METHOD, "hp-authentication-method"),
    (detail::HP_ON_BEHALF_OF, "hp-on-behalf-of"),
    (detail::HP_PERMISSION, "hp-permission"),
    (detail::PROVIDER_FACILITY_TYPE, "provider-facility-type"),
    (
        detail::PROVIDER_HOME_COMMUNITY_ID,
        "provider-home-community-id",
    ),
    (
        detail::PROVIDER_ORGANISATION_TYPE,
        "provider-organisation-type",
    ),
    (detail::POINT_OF_CARE, "point-of-care"),
    (detail::IDENTITY_ASSERTION_ID, "identity-assertion-id"),
    (detail::TRC_ASSERTION_ID, "trc-assertion-id"),
    (detail::REQUEST_MESSAGE_ID, "request-message-id"),
    (detail::CONSUMER_NCP, "consumer-ncp"),
    (detail::PROVIDER_NCP, "provider-ncp"),
    (detail::AGREEMENT_NAME, "agreement-name"),
    (detail::AGREEMENT_VERSION, "agreement-version"),
    (detail::AGREEMENT_LEGAL_GROUND, "agreement-legal-ground"),
    (detail::AGREEMENT_CONSENT, "agreement-consent"),
    (detail::AGREEMENT_ACCESS_CLASS, "agreement-access-class"),
    (detail::BINDING, "binding"),
    (detail::GUEST, "guest"),
    (detail::SECURITY_EVENT, "security-event"),
    (detail::PARTY, "party"),
    (detail::GROUP, "group"),
    (detail::BINDING_CLAIM, "binding-claim"),
    (detail::BINDING_FIELD, "binding-field"),
    (detail::ACTOR, "actor"),
    (detail::REPRESENTS, "represents"),
    (detail::REPRESENTS_GROUND, "represents-ground"),
    (detail::REPRESENTS_RELATIONSHIP, "represents-relationship"),
    (detail::ACTS_FOR, "acts-for"),
    (detail::FACILITY, "facility"),
    (detail::ASSERTED_FIELD, "asserted-field"),
    (detail::COUNTRY_CODE, "country-code"),
    (detail::FAMILY_NAME, "hp-family-name"),
    (detail::GIVEN_NAME, "hp-given-name"),
    (detail::HP_IDENTIFIER, "hp-identifier"),
    (detail::HP_ISSUING_AUTHORITY, "hp-issuing-authority"),
    (detail::HP_ROLE, "hp-professional-role"),
    (detail::PROVIDER_IDENTIFIER, "provider-identifier"),
    (
        detail::PROVIDER_ISSUING_AUTHORITY,
        "provider-issuing-authority",
    ),
    (detail::PROVIDER_NAME, "provider-name"),
    (detail::PROVIDER_ADDRESS, "provider-address"),
    (detail::RETENTION_YEARS, "ehds-retention-years"),
    (detail::RETENTION_PERIOD, "ehds-retention-period"),
    (detail::RETENTION_FLOOR, "ehds-retention-floor"),
    (detail::RETENTION_ENDS, "ehds-retention-ends"),
    (detail::RETENTION_GROUND, "ehds-retention-ground"),
    (detail::EMERGENCY_ACCESS, "ehds-emergency-access"),
    (detail::EMERGENCY_PURPOSE, "ehds-emergency-purpose"),
    (detail::CONSENT_SET_ASIDE, "ehds-consent-set-aside"),
    (detail::POPULATION, "ehds-population"),
    (detail::POPULATION_REASON, "ehds-population-reason"),
    (detail::POPULATION_EHRS, "ehds-population-ehrs"),
    (detail::POPULATION_LIMIT, "ehds-population-limit"),
    (
        detail::TRANSFORMATION_DIRECTION,
        "ehds-transformation-direction",
    ),
    (
        detail::TRANSFORMATION_CATEGORY,
        "ehds-transformation-category",
    ),
    (detail::MAPPING_CONTEXT, "mapping-context"),
    (detail::MAPPING_TEMPLATE, "mapping-template"),
    (detail::MAPPING_PROFILE, "mapping-profile"),
    (detail::MAPPING_FILE, "mapping-file"),
    (detail::MAPPING_PACKAGE, "mapping-package"),
    (detail::TERMINOLOGY_VERSION, "terminology-version"),
    (detail::TRANSCODED, "ehds-transcoded"),
    (detail::TRANSCODED_PATH, "ehds-transcoded-path"),
    (detail::UNTRANSCODED, "ehds-untranscoded"),
    (detail::UNTRANSCODED_PATH, "ehds-untranscoded-path"),
    (detail::DROPPED, "ehds-dropped"),
    (detail::DROPPED_PATH, "ehds-dropped-path"),
];

/// Each name is reachable without the `balp` feature and keeps its wire
/// spelling, and no two names share a spelling.
#[test]
fn every_name_is_reached_with_its_spelling() {
    let mut seen = std::collections::BTreeSet::new();
    for (name, spelling) in NAMES {
        assert_eq!(name, spelling);
        assert!(seen.insert(name), "{name} is defined twice");
    }
}

/// Each name of the `balp` rendering beside the module's.
#[cfg(feature = "balp")]
const BALP_NAMES: [(&str, &str); 91] = [
    (ehds_logging::balp::ACCESSOR_ENTITY, detail::ACCESSOR_ENTITY),
    (ehds_logging::balp::RELAYED_ENTITY, detail::RELAYED_ENTITY),
    (ehds_logging::balp::detail::CATEGORY, detail::CATEGORY),
    (ehds_logging::balp::detail::BASIS, detail::BASIS),
    (
        ehds_logging::balp::detail::CATEGORY_VERSION,
        detail::CATEGORY_VERSION,
    ),
    (ehds_logging::balp::detail::NO_CATEGORY, detail::NO_CATEGORY),
    (
        ehds_logging::balp::detail::UNCLASSIFIED,
        detail::UNCLASSIFIED,
    ),
    (ehds_logging::balp::detail::TEMPLATE_ID, detail::TEMPLATE_ID),
    (
        ehds_logging::balp::detail::ARCHETYPE_ID,
        detail::ARCHETYPE_ID,
    ),
    (ehds_logging::balp::detail::VERSION_UID, detail::VERSION_UID),
    (ehds_logging::balp::detail::UNMAPPED_ID, detail::UNMAPPED_ID),
    (
        ehds_logging::balp::detail::TEMPLATE_ID_COUNT,
        detail::TEMPLATE_ID_COUNT,
    ),
    (
        ehds_logging::balp::detail::ARCHETYPE_ID_COUNT,
        detail::ARCHETYPE_ID_COUNT,
    ),
    (
        ehds_logging::balp::detail::VERSION_UID_COUNT,
        detail::VERSION_UID_COUNT,
    ),
    (
        ehds_logging::balp::detail::UNMAPPED_ID_COUNT,
        detail::UNMAPPED_ID_COUNT,
    ),
    (
        ehds_logging::balp::detail::NO_CATEGORY_KIND,
        detail::NO_CATEGORY_KIND,
    ),
    (
        ehds_logging::balp::detail::NO_CATEGORY_BASIS,
        detail::NO_CATEGORY_BASIS,
    ),
    (
        ehds_logging::balp::detail::ORIGIN_COUNT,
        detail::ORIGIN_COUNT,
    ),
    (ehds_logging::balp::detail::MAP_DIGEST, detail::MAP_DIGEST),
    (ehds_logging::balp::detail::DELIVERED, detail::DELIVERED),
    (
        ehds_logging::balp::detail::STORED_QUERY,
        detail::STORED_QUERY,
    ),
    (ehds_logging::balp::detail::NODE, detail::NODE),
    (ehds_logging::balp::detail::SYSTEM_ID, detail::SYSTEM_ID),
    (ehds_logging::balp::detail::STATUS, detail::STATUS),
    (ehds_logging::balp::detail::ROWS, detail::ROWS),
    (
        ehds_logging::balp::detail::WRITTEN_VERSION,
        detail::WRITTEN_VERSION,
    ),
    (ehds_logging::balp::detail::CREATED_EHR, detail::CREATED_EHR),
    (
        ehds_logging::balp::detail::CREATED_EHR_UNREAD,
        detail::CREATED_EHR_UNREAD,
    ),
    (ehds_logging::balp::detail::ENDPOINT, detail::ENDPOINT),
    (
        ehds_logging::balp::detail::PATIENT_LOOKUP,
        detail::PATIENT_LOOKUP,
    ),
    (ehds_logging::balp::detail::CORRELATION, detail::CORRELATION),
    (
        ehds_logging::balp::detail::CORRELATION_SOURCE,
        detail::CORRELATION_SOURCE,
    ),
    (ehds_logging::balp::detail::RELAYED_VIA, detail::RELAYED_VIA),
    (
        ehds_logging::balp::detail::RELAYED_ISSUER,
        detail::RELAYED_ISSUER,
    ),
    (ehds_logging::balp::detail::ASSERTED, detail::ASSERTED),
    (
        ehds_logging::balp::detail::PROFESSIONAL_NAMED,
        detail::PROFESSIONAL_NAMED,
    ),
    (ehds_logging::balp::detail::HP_NAME, detail::HP_NAME),
    (ehds_logging::balp::detail::HP_NAME_ID, detail::HP_NAME_ID),
    (
        ehds_logging::balp::detail::HP_ON_BEHALF_OF,
        detail::HP_ON_BEHALF_OF,
    ),
    (
        ehds_logging::balp::detail::HP_PERMISSION,
        detail::HP_PERMISSION,
    ),
    (
        ehds_logging::balp::detail::POINT_OF_CARE,
        detail::POINT_OF_CARE,
    ),
    (
        ehds_logging::balp::detail::TRC_ASSERTION_ID,
        detail::TRC_ASSERTION_ID,
    ),
    (
        ehds_logging::balp::detail::REQUEST_MESSAGE_ID,
        detail::REQUEST_MESSAGE_ID,
    ),
    (
        ehds_logging::balp::detail::CONSUMER_NCP,
        detail::CONSUMER_NCP,
    ),
    (
        ehds_logging::balp::detail::PROVIDER_NCP,
        detail::PROVIDER_NCP,
    ),
    (
        ehds_logging::balp::detail::AGREEMENT_NAME,
        detail::AGREEMENT_NAME,
    ),
    (
        ehds_logging::balp::detail::AGREEMENT_VERSION,
        detail::AGREEMENT_VERSION,
    ),
    (
        ehds_logging::balp::detail::AGREEMENT_CONSENT,
        detail::AGREEMENT_CONSENT,
    ),
    (ehds_logging::balp::detail::BINDING, detail::BINDING),
    (ehds_logging::balp::detail::GUEST, detail::GUEST),
    (
        ehds_logging::balp::detail::SECURITY_EVENT,
        detail::SECURITY_EVENT,
    ),
    (ehds_logging::balp::detail::PARTY, detail::PARTY),
    (ehds_logging::balp::detail::GROUP, detail::GROUP),
    (
        ehds_logging::balp::detail::BINDING_CLAIM,
        detail::BINDING_CLAIM,
    ),
    (
        ehds_logging::balp::detail::BINDING_FIELD,
        detail::BINDING_FIELD,
    ),
    (ehds_logging::balp::detail::ACTOR, detail::ACTOR),
    (ehds_logging::balp::detail::REPRESENTS, detail::REPRESENTS),
    (
        ehds_logging::balp::detail::REPRESENTS_GROUND,
        detail::REPRESENTS_GROUND,
    ),
    (ehds_logging::balp::detail::ACTS_FOR, detail::ACTS_FOR),
    (ehds_logging::balp::detail::FACILITY, detail::FACILITY),
    (
        ehds_logging::balp::detail::ASSERTED_FIELD,
        detail::ASSERTED_FIELD,
    ),
    (
        ehds_logging::balp::detail::COUNTRY_CODE,
        detail::COUNTRY_CODE,
    ),
    (ehds_logging::balp::detail::FAMILY_NAME, detail::FAMILY_NAME),
    (ehds_logging::balp::detail::GIVEN_NAME, detail::GIVEN_NAME),
    (
        ehds_logging::balp::detail::HP_IDENTIFIER,
        detail::HP_IDENTIFIER,
    ),
    (ehds_logging::balp::detail::HP_ROLE, detail::HP_ROLE),
    (
        ehds_logging::balp::detail::PROVIDER_NAME,
        detail::PROVIDER_NAME,
    ),
    (
        ehds_logging::balp::detail::PROVIDER_ADDRESS,
        detail::PROVIDER_ADDRESS,
    ),
    (
        ehds_logging::balp::detail::RETENTION_YEARS,
        detail::RETENTION_YEARS,
    ),
    (
        ehds_logging::balp::detail::RETENTION_PERIOD,
        detail::RETENTION_PERIOD,
    ),
    (
        ehds_logging::balp::detail::RETENTION_FLOOR,
        detail::RETENTION_FLOOR,
    ),
    (
        ehds_logging::balp::detail::RETENTION_ENDS,
        detail::RETENTION_ENDS,
    ),
    (
        ehds_logging::balp::detail::RETENTION_GROUND,
        detail::RETENTION_GROUND,
    ),
    (
        ehds_logging::balp::detail::EMERGENCY_ACCESS,
        detail::EMERGENCY_ACCESS,
    ),
    (
        ehds_logging::balp::detail::EMERGENCY_PURPOSE,
        detail::EMERGENCY_PURPOSE,
    ),
    (
        ehds_logging::balp::detail::CONSENT_SET_ASIDE,
        detail::CONSENT_SET_ASIDE,
    ),
    (ehds_logging::balp::detail::POPULATION, detail::POPULATION),
    (
        ehds_logging::balp::detail::POPULATION_REASON,
        detail::POPULATION_REASON,
    ),
    (
        ehds_logging::balp::detail::POPULATION_EHRS,
        detail::POPULATION_EHRS,
    ),
    (
        ehds_logging::balp::detail::POPULATION_LIMIT,
        detail::POPULATION_LIMIT,
    ),
    (
        ehds_logging::balp::detail::MAPPING_CONTEXT,
        detail::MAPPING_CONTEXT,
    ),
    (
        ehds_logging::balp::detail::MAPPING_TEMPLATE,
        detail::MAPPING_TEMPLATE,
    ),
    (
        ehds_logging::balp::detail::MAPPING_PROFILE,
        detail::MAPPING_PROFILE,
    ),
    (
        ehds_logging::balp::detail::MAPPING_FILE,
        detail::MAPPING_FILE,
    ),
    (
        ehds_logging::balp::detail::MAPPING_PACKAGE,
        detail::MAPPING_PACKAGE,
    ),
    (ehds_logging::balp::detail::TRANSCODED, detail::TRANSCODED),
    (
        ehds_logging::balp::detail::TRANSCODED_PATH,
        detail::TRANSCODED_PATH,
    ),
    (
        ehds_logging::balp::detail::UNTRANSCODED,
        detail::UNTRANSCODED,
    ),
    (
        ehds_logging::balp::detail::UNTRANSCODED_PATH,
        detail::UNTRANSCODED_PATH,
    ),
    (ehds_logging::balp::detail::DROPPED, detail::DROPPED),
    (
        ehds_logging::balp::detail::DROPPED_PATH,
        detail::DROPPED_PATH,
    ),
];

/// The `balp` rendering's names are the module's, one definition each.
#[cfg(feature = "balp")]
#[test]
fn the_balp_names_are_the_modules() {
    for (balp, shared) in BALP_NAMES {
        assert_eq!(balp, shared);
    }
}
