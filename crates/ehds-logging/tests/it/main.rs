// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The integration tests of `ehds-logging`, one binary with a module per
//! subject.
//!
//! The modules carry `#[cfg(test)]`, which an integration test always has, so
//! the test-scoped relaxations of `clippy.toml` reach their helpers too.

#[cfg(test)]
mod archetype_id;
#[cfg(test)]
#[cfg(feature = "balp")]
mod balp;
#[cfg(test)]
mod category;
#[cfg(test)]
mod classify;
#[cfg(test)]
mod detail;
#[cfg(test)]
mod emergency;
#[cfg(test)]
mod features;
#[cfg(test)]
mod label;
#[cfg(test)]
mod map;
#[cfg(test)]
mod property;
#[cfg(test)]
mod relayed;
#[cfg(test)]
mod retention;
#[cfg(test)]
#[cfg(feature = "schema")]
mod schema;
#[cfg(test)]
mod withheld;

/// A record of a read by a local user, with no feature's types.
#[cfg(test)]
mod balp_free {
    use ehds_logging::classify::{Basis, Evidence};
    use ehds_logging::label::Labels;
    use ehds_logging::record::{
        AccessRecord, Accessor, Action, DataSubject, Outcome, Professional, Request,
    };
    use ehds_logging::retention::RetentionPolicy;

    use super::support::map;

    pub(crate) fn record() -> AccessRecord {
        let categories = map().classify(&Evidence::reached(Basis::Returned, Vec::new()));
        AccessRecord {
            action: Action::Read,
            recorded: jiff::Timestamp::UNIX_EPOCH,
            outcome: Outcome::Success,
            outcome_description: None,
            accessor: Accessor {
                issuer: None,
                subject: "Qz7-subject-75".to_owned(),
                client_id: None,
                audience: None,
                provider: None,
                acting: None,
                roles: Vec::new(),
                token_id: None,
                assurance: None,
                professional: Professional::default(),
                alt_id: None,
                purposes: Vec::new(),
                policies: Vec::new(),
                relayed: None,
                binding: None,
                actors: Vec::new(),
                represents: None,
                acts_for: None,
                facilities: Vec::new(),
            },
            subject: DataSubject::default(),
            retention: RetentionPolicy::default().retain(
                jiff::Timestamp::UNIX_EPOCH,
                &categories,
                [],
            ),
            categories,
            delivered: None,
            origins: Vec::new(),
            origin_count: None,
            request: Request {
                id: None,
                operation: "composition_get".to_owned(),
                operation_system: None,
                resource: None,
                query: None,
                stored_query: None,
                client_address: None,
                correlation: None,
            },
            labels: Labels::default(),
            emergency: None,
            inserted: None,
            transformations: Vec::new(),
        }
    }
}

#[cfg(test)]
mod support {
    use std::collections::BTreeMap;

    use ehds_logging::map::{CategoryMap, Declared};
    use ehds_logging::record::{
        AccessClass, Agreement, AuthenticationMethod, Binding, BoundParty, Coded, Gateways, NameId,
        PartyRef, RelayedParts, RelayedProfessional, RelayedProvider, Via,
    };

    /// A synthetic lab report template, mapped to medical test results.
    pub(crate) const LAB_REPORT: &str = "Example Lab Report.v1";
    /// A synthetic discharge template, mapped to discharge reports.
    pub(crate) const DISCHARGE: &str = "Example Discharge.v1";
    /// A synthetic administrative template, mapped to no category.
    pub(crate) const ADMIN: &str = "Example Admin Note.v1";
    /// A synthetic template the map holds no key for.
    pub(crate) const UNMAPPED: &str = "Example Unmapped.v1";
    /// An archetype mapped to medical test results.
    pub(crate) const LAB_ARCHETYPE: &str = "openEHR-EHR-OBSERVATION.laboratory_test_result.v1";
    /// A synthetic national category, in a system under the example domain.
    pub(crate) const NATIONAL: &str =
        "https://example.org/fhir/CodeSystem/national-category|nl-example";
    /// An archetype mapped to patient summaries and the national category.
    pub(crate) const SUMMARY_ARCHETYPE: &str = "openEHR-EHR-EVALUATION.problem_diagnosis.v1";

    fn codes(codes: &[&str]) -> Declared {
        Declared::Codes(codes.iter().map(|code| (*code).to_owned()).collect())
    }

    /// The map the tests classify with.
    pub(crate) fn map() -> CategoryMap {
        let templates = BTreeMap::from([
            (LAB_REPORT.to_owned(), codes(&["Laboratory-Reports"])),
            (DISCHARGE.to_owned(), codes(&["Discharge-Reports"])),
            (ADMIN.to_owned(), Declared::Word("none".to_owned())),
        ]);
        let archetypes = BTreeMap::from([
            (LAB_ARCHETYPE.to_owned(), codes(&["Laboratory-Reports"])),
            (
                SUMMARY_ARCHETYPE.to_owned(),
                codes(&["Patient-Summaries", NATIONAL]),
            ),
        ]);
        CategoryMap::declare(&[NATIONAL.to_owned()], &templates, &archetypes)
            .expect("the test map")
            .with_digest("sha256:test".to_owned())
    }

    /// The synthetic values of a relayed access, each unlike any other text.
    pub(crate) mod relayed {
        pub(crate) const CONTACT_POINT: &str = "https://ncp-a.example.org";
        pub(crate) const FEDERATION: &str = "https://federation.example.org";
        pub(crate) const NAME: &str = "Qz7-hp-name-51";
        pub(crate) const FAMILY: &str = "Qz7-family-52";
        pub(crate) const GIVEN: &str = "Qz7-given-53";
        pub(crate) const HP_ID: &str = "Qz7-hp-54";
        pub(crate) const HP_AUTHORITY: &str = "Qz7-hp-authority-55";
        pub(crate) const ROLE: &str = "221";
        pub(crate) const ISCO: &str = "2.16.840.1.113883.2.9.6.2.7";
        pub(crate) const HP_PROVIDER: &str = "Qz7-hp-provider-56";
        pub(crate) const NAME_ID: &str = "Qz7-nameid-57";
        pub(crate) const SP_PROVIDED_ID: &str = "Qz7-spid-58";
        pub(crate) const IDA_ISSUER: &str = "https://ncp-b.example.org/idp";
        pub(crate) const PERMISSION: &str =
            "urn:oasis:names:tc:xspa:1.0:subject:hl7:permission:PRD-006";
        pub(crate) const HCP_ID: &str = "urn:oid:2.999.56";
        pub(crate) const HCP_AUTHORITY: &str = "Qz7-hcp-authority-57";
        pub(crate) const HCP_NAME: &str = "Qz7-hcp-name-58";
        pub(crate) const HCP_ADDRESS: &str = "Qz7-hcp-address-59";
        pub(crate) const FACILITY: &str = "Hospital";
        pub(crate) const FACILITY_SYSTEM: &str = "1.3.6.1.4.1.12559.11.10.1.3.2.2.2";
        pub(crate) const HOME_COMMUNITY: &str = "urn:oid:2.999.60";
        pub(crate) const ORGANISATION_TYPE: &str = "Qz7-org-type-61";
        pub(crate) const POINT_OF_CARE: &str = "Qz7-point-of-care-62";
        pub(crate) const IDA: &str = "urn:uuid:0f1e2d3c-4b5a-4968-8776-655443322110";
        pub(crate) const TRC: &str = "urn:uuid:1a2b3c4d-5e6f-4a7b-8c9d-0e1f2a3b4c5d";
        pub(crate) const MESSAGE: &str = "urn:uuid:9f8e7d6c-5b4a-4392-8170-6f5e4d3c2b1a";
        pub(crate) const CONSUMER_NCP: &str = "ncp-b.example.org";
        pub(crate) const PROVIDER_NCP: &str = "ncp-a.example.org";
        pub(crate) const AGREEMENT: &str = "Qz7-agreement-63";
        pub(crate) const AGREEMENT_VERSION: &str = "2.1";
        pub(crate) const GROUND: &str = "Qz7-ground-64";
        pub(crate) const CONSENT: &str = "Qz7-consent-65";
    }

    /// What a MyHealth@EU national contact point relays for a named
    /// professional, with every eHDSI member.
    pub(crate) fn contact_point() -> RelayedParts {
        use relayed::*;
        RelayedParts {
            via: Via::ContactPoint {
                issuer: CONTACT_POINT.to_owned(),
            },
            country_code: Some("XA".to_owned()),
            professional: Some(RelayedProfessional {
                name: Some(NAME.to_owned()),
                family_name: Some(FAMILY.to_owned()),
                given_name: Some(GIVEN.to_owned()),
                identifier: Some(HP_ID.to_owned()),
                issuing_authority: Some(HP_AUTHORITY.to_owned()),
                roles: vec![Coded {
                    system: Some(ISCO.to_owned()),
                    code: ROLE.to_owned(),
                }],
                healthcare_provider_identifier: Some(HP_PROVIDER.to_owned()),
                name_id: Some(NameId {
                    value: NAME_ID.to_owned(),
                    sp_provided_id: Some(SP_PROVIDED_ID.to_owned()),
                    issuer: IDA_ISSUER.to_owned(),
                }),
                authentication_method: Some(AuthenticationMethod::SmartcardPki),
                on_behalf_of: None,
                permissions: vec![PERMISSION.to_owned()],
            }),
            provider: Some(RelayedProvider {
                identifier: HCP_ID.to_owned(),
                issuing_authority: Some(HCP_AUTHORITY.to_owned()),
                name: Some(HCP_NAME.to_owned()),
                address: Some(HCP_ADDRESS.to_owned()),
                facility_type: Some(Coded {
                    system: Some(FACILITY_SYSTEM.to_owned()),
                    code: FACILITY.to_owned(),
                }),
                home_community_id: Some(HOME_COMMUNITY.to_owned()),
                organisation_type: None,
            }),
            agreement: None,
            point_of_care: Some(POINT_OF_CARE.to_owned()),
            identity_assertion_id: Some(IDA.to_owned()),
            trc_assertion_id: Some(TRC.to_owned()),
            message_id: Some(MESSAGE.to_owned()),
            gateways: Gateways {
                consumer: Some(CONSUMER_NCP.to_owned()),
                provider: Some(PROVIDER_NCP.to_owned()),
            },
            asserted: BTreeMap::new(),
        }
    }

    /// An agreement of a domestic federation, with its legal ground, the
    /// consent decision and an access class.
    pub(crate) fn agreement(access_class: Option<AccessClass>) -> Agreement {
        use relayed::*;
        Agreement {
            name: AGREEMENT.to_owned(),
            version: AGREEMENT_VERSION.to_owned(),
            legal_ground: vec![
                Coded {
                    system: Some("https://example.org/gdpr".to_owned()),
                    code: "art-9-2-h".to_owned(),
                },
                Coded {
                    system: None,
                    code: GROUND.to_owned(),
                },
            ],
            consent: Some(CONSENT.to_owned()),
            access_class,
        }
    }

    /// An organisation a domestic federation admits without a named person.
    pub(crate) fn organisation(access_class: AccessClass) -> RelayedParts {
        use relayed::*;
        RelayedParts {
            via: Via::Federation {
                issuer: FEDERATION.to_owned(),
            },
            professional: None,
            agreement: Some(agreement(Some(access_class))),
            point_of_care: None,
            identity_assertion_id: None,
            trc_assertion_id: None,
            message_id: None,
            gateways: Gateways::default(),
            provider: contact_point().provider.map(|provider| RelayedProvider {
                organisation_type: Some(Coded {
                    system: Some("https://example.org/organisation-type".to_owned()),
                    code: ORGANISATION_TYPE.to_owned(),
                }),
                ..provider
            }),
            ..contact_point()
        }
    }

    /// A party in a demographic store, by its `PARTY_REF` triple.
    pub(crate) fn party(id: &str) -> PartyRef {
        PartyRef {
            namespace: "demographic".to_owned(),
            kind: "PERSON".to_owned(),
            id: id.to_owned(),
        }
    }

    /// A group in a demographic store.
    pub(crate) fn group(id: &str) -> PartyRef {
        PartyRef {
            kind: "GROUP".to_owned(),
            ..party(id)
        }
    }

    /// A person bound to a complete party, a member of two groups.
    pub(crate) fn bound(guest: bool) -> Binding {
        Binding::Bound {
            party: BoundParty {
                party: party("Qz7-party-66"),
                groups: vec![group("Qz7-group-67"), group("Qz7-group-68")],
            },
            guest,
        }
    }
}
