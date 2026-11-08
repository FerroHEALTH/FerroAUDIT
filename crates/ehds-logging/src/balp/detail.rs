// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The names of the `detail` entries a record's added entities carry, each
//! as [`crate::detail`] defines it, which needs no feature.

/// The namespace of a patient identifier that is no URI, on the patient
/// entity (the name `ihe-atna` writes).
pub const PATIENT_NAMESPACE: &str = ihe_atna::balp::PATIENT_NAMESPACE;
/// [`crate::detail::CATEGORY`].
pub const CATEGORY: &str = crate::detail::CATEGORY;
/// [`crate::detail::BASIS`].
pub const BASIS: &str = crate::detail::BASIS;
/// [`crate::detail::CATEGORY_VERSION`].
pub const CATEGORY_VERSION: &str = crate::detail::CATEGORY_VERSION;
/// [`crate::detail::NO_CATEGORY`].
pub const NO_CATEGORY: &str = crate::detail::NO_CATEGORY;
/// [`crate::detail::UNCLASSIFIED`].
pub const UNCLASSIFIED: &str = crate::detail::UNCLASSIFIED;
/// [`crate::detail::TEMPLATE_ID`].
pub const TEMPLATE_ID: &str = crate::detail::TEMPLATE_ID;
/// [`crate::detail::ARCHETYPE_ID`].
pub const ARCHETYPE_ID: &str = crate::detail::ARCHETYPE_ID;
/// [`crate::detail::VERSION_UID`].
pub const VERSION_UID: &str = crate::detail::VERSION_UID;
/// [`crate::detail::UNMAPPED_ID`].
pub const UNMAPPED_ID: &str = crate::detail::UNMAPPED_ID;
/// [`crate::detail::TEMPLATE_ID_COUNT`].
pub const TEMPLATE_ID_COUNT: &str = crate::detail::TEMPLATE_ID_COUNT;
/// [`crate::detail::ARCHETYPE_ID_COUNT`].
pub const ARCHETYPE_ID_COUNT: &str = crate::detail::ARCHETYPE_ID_COUNT;
/// [`crate::detail::VERSION_UID_COUNT`].
pub const VERSION_UID_COUNT: &str = crate::detail::VERSION_UID_COUNT;
/// [`crate::detail::UNMAPPED_ID_COUNT`].
pub const UNMAPPED_ID_COUNT: &str = crate::detail::UNMAPPED_ID_COUNT;
/// [`crate::detail::NO_CATEGORY_KIND`].
pub const NO_CATEGORY_KIND: &str = crate::detail::NO_CATEGORY_KIND;
/// [`crate::detail::NO_CATEGORY_BASIS`].
pub const NO_CATEGORY_BASIS: &str = crate::detail::NO_CATEGORY_BASIS;
/// [`crate::detail::ORIGIN_COUNT`].
pub const ORIGIN_COUNT: &str = crate::detail::ORIGIN_COUNT;
/// [`crate::detail::MAP_DIGEST`].
pub const MAP_DIGEST: &str = crate::detail::MAP_DIGEST;
/// [`crate::detail::DELIVERED`].
pub const DELIVERED: &str = crate::detail::DELIVERED;
/// [`crate::detail::STORED_QUERY`].
pub const STORED_QUERY: &str = crate::detail::STORED_QUERY;
/// [`crate::detail::NODE`].
pub const NODE: &str = crate::detail::NODE;
/// [`crate::detail::SYSTEM_ID`].
pub const SYSTEM_ID: &str = crate::detail::SYSTEM_ID;
/// [`crate::detail::STATUS`].
pub const STATUS: &str = crate::detail::STATUS;
/// [`crate::detail::ROWS`].
pub const ROWS: &str = crate::detail::ROWS;
/// [`crate::detail::WRITTEN_VERSION`].
pub const WRITTEN_VERSION: &str = crate::detail::WRITTEN_VERSION;
/// [`crate::detail::WRITTEN_VERSION_UNREAD`].
pub const WRITTEN_VERSION_UNREAD: &str = crate::detail::WRITTEN_VERSION_UNREAD;
/// [`crate::detail::CREATED_EHR`].
pub const CREATED_EHR: &str = crate::detail::CREATED_EHR;
/// [`crate::detail::CREATED_EHR_UNREAD`].
pub const CREATED_EHR_UNREAD: &str = crate::detail::CREATED_EHR_UNREAD;
/// [`crate::detail::ENDPOINT`].
pub const ENDPOINT: &str = crate::detail::ENDPOINT;
/// [`crate::detail::PATIENT_LOOKUP`].
pub const PATIENT_LOOKUP: &str = crate::detail::PATIENT_LOOKUP;
/// [`crate::detail::CORRELATION`].
pub const CORRELATION: &str = crate::detail::CORRELATION;
/// [`crate::detail::CORRELATION_SOURCE`].
pub const CORRELATION_SOURCE: &str = crate::detail::CORRELATION_SOURCE;
/// [`crate::detail::RELAYED_VIA`].
pub const RELAYED_VIA: &str = crate::detail::RELAYED_VIA;
/// [`crate::detail::RELAYED_ISSUER`].
pub const RELAYED_ISSUER: &str = crate::detail::RELAYED_ISSUER;
/// [`crate::detail::ASSERTED`].
pub const ASSERTED: &str = crate::detail::ASSERTED;
/// [`crate::detail::PROFESSIONAL_NAMED`].
pub const PROFESSIONAL_NAMED: &str = crate::detail::PROFESSIONAL_NAMED;
/// [`crate::detail::HP_NAME`].
pub const HP_NAME: &str = crate::detail::HP_NAME;
/// [`crate::detail::HP_PROVIDER_IDENTIFIER`].
pub const HP_PROVIDER_IDENTIFIER: &str = crate::detail::HP_PROVIDER_IDENTIFIER;
/// [`crate::detail::HP_NAME_ID`].
pub const HP_NAME_ID: &str = crate::detail::HP_NAME_ID;
/// [`crate::detail::HP_AUTHENTICATION_METHOD`].
pub const HP_AUTHENTICATION_METHOD: &str = crate::detail::HP_AUTHENTICATION_METHOD;
/// [`crate::detail::HP_ON_BEHALF_OF`].
pub const HP_ON_BEHALF_OF: &str = crate::detail::HP_ON_BEHALF_OF;
/// [`crate::detail::HP_PERMISSION`].
pub const HP_PERMISSION: &str = crate::detail::HP_PERMISSION;
/// [`crate::detail::PROVIDER_FACILITY_TYPE`].
pub const PROVIDER_FACILITY_TYPE: &str = crate::detail::PROVIDER_FACILITY_TYPE;
/// [`crate::detail::PROVIDER_HOME_COMMUNITY_ID`].
pub const PROVIDER_HOME_COMMUNITY_ID: &str = crate::detail::PROVIDER_HOME_COMMUNITY_ID;
/// [`crate::detail::PROVIDER_ORGANISATION_TYPE`].
pub const PROVIDER_ORGANISATION_TYPE: &str = crate::detail::PROVIDER_ORGANISATION_TYPE;
/// [`crate::detail::POINT_OF_CARE`].
pub const POINT_OF_CARE: &str = crate::detail::POINT_OF_CARE;
/// [`crate::detail::IDENTITY_ASSERTION_ID`].
pub const IDENTITY_ASSERTION_ID: &str = crate::detail::IDENTITY_ASSERTION_ID;
/// [`crate::detail::TRC_ASSERTION_ID`].
pub const TRC_ASSERTION_ID: &str = crate::detail::TRC_ASSERTION_ID;
/// [`crate::detail::REQUEST_MESSAGE_ID`].
pub const REQUEST_MESSAGE_ID: &str = crate::detail::REQUEST_MESSAGE_ID;
/// [`crate::detail::CONSUMER_NCP`].
pub const CONSUMER_NCP: &str = crate::detail::CONSUMER_NCP;
/// [`crate::detail::PROVIDER_NCP`].
pub const PROVIDER_NCP: &str = crate::detail::PROVIDER_NCP;
/// [`crate::detail::AGREEMENT_NAME`].
pub const AGREEMENT_NAME: &str = crate::detail::AGREEMENT_NAME;
/// [`crate::detail::AGREEMENT_VERSION`].
pub const AGREEMENT_VERSION: &str = crate::detail::AGREEMENT_VERSION;
/// [`crate::detail::AGREEMENT_LEGAL_GROUND`].
pub const AGREEMENT_LEGAL_GROUND: &str = crate::detail::AGREEMENT_LEGAL_GROUND;
/// [`crate::detail::AGREEMENT_CONSENT`].
pub const AGREEMENT_CONSENT: &str = crate::detail::AGREEMENT_CONSENT;
/// [`crate::detail::AGREEMENT_ACCESS_CLASS`].
pub const AGREEMENT_ACCESS_CLASS: &str = crate::detail::AGREEMENT_ACCESS_CLASS;
/// [`crate::detail::BINDING`].
pub const BINDING: &str = crate::detail::BINDING;
/// [`crate::detail::GUEST`].
pub const GUEST: &str = crate::detail::GUEST;
/// [`crate::detail::SECURITY_EVENT`].
pub const SECURITY_EVENT: &str = crate::detail::SECURITY_EVENT;
/// [`crate::detail::PARTY`].
pub const PARTY: &str = crate::detail::PARTY;
/// [`crate::detail::GROUP`].
pub const GROUP: &str = crate::detail::GROUP;
/// [`crate::detail::BINDING_CLAIM`].
pub const BINDING_CLAIM: &str = crate::detail::BINDING_CLAIM;
/// [`crate::detail::BINDING_FIELD`].
pub const BINDING_FIELD: &str = crate::detail::BINDING_FIELD;
/// [`crate::detail::ACTOR`].
pub const ACTOR: &str = crate::detail::ACTOR;
/// [`crate::detail::REPRESENTS`].
pub const REPRESENTS: &str = crate::detail::REPRESENTS;
/// [`crate::detail::REPRESENTS_GROUND`].
pub const REPRESENTS_GROUND: &str = crate::detail::REPRESENTS_GROUND;
/// [`crate::detail::REPRESENTS_RELATIONSHIP`].
pub const REPRESENTS_RELATIONSHIP: &str = crate::detail::REPRESENTS_RELATIONSHIP;
/// [`crate::detail::ACTS_FOR`].
pub const ACTS_FOR: &str = crate::detail::ACTS_FOR;
/// [`crate::detail::FACILITY`].
pub const FACILITY: &str = crate::detail::FACILITY;
/// [`crate::detail::ASSERTED_FIELD`].
pub const ASSERTED_FIELD: &str = crate::detail::ASSERTED_FIELD;
/// [`crate::detail::COUNTRY_CODE`].
pub const COUNTRY_CODE: &str = crate::detail::COUNTRY_CODE;
/// [`crate::detail::FAMILY_NAME`].
pub const FAMILY_NAME: &str = crate::detail::FAMILY_NAME;
/// [`crate::detail::GIVEN_NAME`].
pub const GIVEN_NAME: &str = crate::detail::GIVEN_NAME;
/// [`crate::detail::HP_IDENTIFIER`].
pub const HP_IDENTIFIER: &str = crate::detail::HP_IDENTIFIER;
/// [`crate::detail::HP_ISSUING_AUTHORITY`].
pub const HP_ISSUING_AUTHORITY: &str = crate::detail::HP_ISSUING_AUTHORITY;
/// [`crate::detail::HP_ROLE`].
pub const HP_ROLE: &str = crate::detail::HP_ROLE;
/// [`crate::detail::PROVIDER_IDENTIFIER`].
pub const PROVIDER_IDENTIFIER: &str = crate::detail::PROVIDER_IDENTIFIER;
/// [`crate::detail::PROVIDER_ISSUING_AUTHORITY`].
pub const PROVIDER_ISSUING_AUTHORITY: &str = crate::detail::PROVIDER_ISSUING_AUTHORITY;
/// [`crate::detail::PROVIDER_NAME`].
pub const PROVIDER_NAME: &str = crate::detail::PROVIDER_NAME;
/// [`crate::detail::PROVIDER_ADDRESS`].
pub const PROVIDER_ADDRESS: &str = crate::detail::PROVIDER_ADDRESS;
/// [`crate::detail::RETENTION_YEARS`].
pub const RETENTION_YEARS: &str = crate::detail::RETENTION_YEARS;
/// [`crate::detail::RETENTION_PERIOD`].
pub const RETENTION_PERIOD: &str = crate::detail::RETENTION_PERIOD;
/// [`crate::detail::RETENTION_FLOOR`].
pub const RETENTION_FLOOR: &str = crate::detail::RETENTION_FLOOR;
/// [`crate::detail::RETENTION_ENDS`].
pub const RETENTION_ENDS: &str = crate::detail::RETENTION_ENDS;
/// [`crate::detail::RETENTION_GROUND`].
pub const RETENTION_GROUND: &str = crate::detail::RETENTION_GROUND;
/// [`crate::detail::EMERGENCY_ACCESS`].
pub const EMERGENCY_ACCESS: &str = crate::detail::EMERGENCY_ACCESS;
/// [`crate::detail::EMERGENCY_PURPOSE`].
pub const EMERGENCY_PURPOSE: &str = crate::detail::EMERGENCY_PURPOSE;
/// [`crate::detail::CONSENT_SET_ASIDE`].
pub const CONSENT_SET_ASIDE: &str = crate::detail::CONSENT_SET_ASIDE;
/// [`crate::detail::POPULATION`].
pub const POPULATION: &str = crate::detail::POPULATION;
/// [`crate::detail::POPULATION_REASON`].
pub const POPULATION_REASON: &str = crate::detail::POPULATION_REASON;
/// [`crate::detail::POPULATION_EHRS`].
pub const POPULATION_EHRS: &str = crate::detail::POPULATION_EHRS;
/// [`crate::detail::POPULATION_LIMIT`].
pub const POPULATION_LIMIT: &str = crate::detail::POPULATION_LIMIT;
/// [`crate::detail::TRANSFORMATION_DIRECTION`].
pub const TRANSFORMATION_DIRECTION: &str = crate::detail::TRANSFORMATION_DIRECTION;
/// [`crate::detail::TRANSFORMATION_CATEGORY`].
pub const TRANSFORMATION_CATEGORY: &str = crate::detail::TRANSFORMATION_CATEGORY;
/// [`crate::detail::MAPPING_CONTEXT`].
pub const MAPPING_CONTEXT: &str = crate::detail::MAPPING_CONTEXT;
/// [`crate::detail::MAPPING_TEMPLATE`].
pub const MAPPING_TEMPLATE: &str = crate::detail::MAPPING_TEMPLATE;
/// [`crate::detail::MAPPING_PROFILE`].
pub const MAPPING_PROFILE: &str = crate::detail::MAPPING_PROFILE;
/// [`crate::detail::MAPPING_FILE`].
pub const MAPPING_FILE: &str = crate::detail::MAPPING_FILE;
/// [`crate::detail::MAPPING_PACKAGE`].
pub const MAPPING_PACKAGE: &str = crate::detail::MAPPING_PACKAGE;
/// [`crate::detail::TERMINOLOGY_VERSION`].
pub const TERMINOLOGY_VERSION: &str = crate::detail::TERMINOLOGY_VERSION;
/// [`crate::detail::TRANSCODED`].
pub const TRANSCODED: &str = crate::detail::TRANSCODED;
/// [`crate::detail::TRANSCODED_PATH`].
pub const TRANSCODED_PATH: &str = crate::detail::TRANSCODED_PATH;
/// [`crate::detail::UNTRANSCODED`].
pub const UNTRANSCODED: &str = crate::detail::UNTRANSCODED;
/// [`crate::detail::UNTRANSCODED_PATH`].
pub const UNTRANSCODED_PATH: &str = crate::detail::UNTRANSCODED_PATH;
/// [`crate::detail::DROPPED`].
pub const DROPPED: &str = crate::detail::DROPPED;
/// [`crate::detail::DROPPED_PATH`].
pub const DROPPED_PATH: &str = crate::detail::DROPPED_PATH;
