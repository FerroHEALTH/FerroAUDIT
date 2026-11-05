// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! Who accessed the data when no party here names them (Regulation (EU)
//! 2025/327 Annex II 3.2(a), (b)): what a MyHealth@EU contact point or a
//! domestic federation relays, held to the rules that make it a record of an
//! access, the binding outcome with the party it names, and the codes a
//! record writes for each.

use ehds_logging::record::{
    AccessClass, Acting, AuthenticationMethod, Binding, Coded, Correlation, CorrelationSource,
    InvalidRecord, PartyRef, Relayed, RelayedError, RelayedParts, Via,
};

use super::support::{agreement, bound, contact_point, group, organisation, party, relayed};

/// Implementing Regulation (EU) 2026/2099 Art 7 with the MyHealth@EU
/// Requirements Catalogue 10.0.0 requirement 01: a contact point relays a
/// named professional, so one that names none is refused.
#[test]
fn a_contact_point_that_relays_no_professional_is_refused() {
    let parts = RelayedParts {
        professional: None,
        ..contact_point()
    };
    assert_eq!(
        Relayed::try_from(parts).err(),
        Some(RelayedError::ContactPointWithoutProfessional)
    );
}

#[test]
fn a_contact_point_that_relays_a_professional_constructs() {
    let relayed = Relayed::try_from(contact_point()).expect("a relayed access");
    assert_eq!(relayed.as_parts().via.code(), "contact-point");
    assert_eq!(relayed.as_parts().via.issuer(), relayed::CONTACT_POINT);
    assert!(relayed.as_parts().professional.is_some());
}

/// An organisation without a named person is admitted under a federation
/// agreement that declares its access class (no specification governs this:
/// our own design).
#[test]
fn a_federation_organisation_with_no_professional_constructs() {
    for class in [AccessClass::Automated, AccessClass::Chained] {
        let relayed = Relayed::try_from(organisation(class)).expect("an organisation access");
        assert_eq!(relayed.as_parts().via.code(), "federation");
        assert!(relayed.as_parts().professional.is_none());
    }
}

/// GDPR Art 5(2) and Art 30(1)(b): every federation access names the
/// agreement it is made under.
#[test]
fn a_federation_access_with_no_agreement_is_refused() {
    let parts = RelayedParts {
        agreement: None,
        ..organisation(AccessClass::Automated)
    };
    assert_eq!(
        Relayed::try_from(parts).err(),
        Some(RelayedError::FederationWithoutAgreement)
    );
    let named = RelayedParts {
        via: Via::Federation {
            issuer: relayed::FEDERATION.to_owned(),
        },
        ..contact_point()
    };
    assert_eq!(
        Relayed::try_from(named).err(),
        Some(RelayedError::FederationWithoutAgreement)
    );
}

#[test]
fn an_organisation_whose_agreement_declares_no_access_class_is_refused() {
    let parts = RelayedParts {
        agreement: Some(agreement(None)),
        ..organisation(AccessClass::Chained)
    };
    assert_eq!(
        Relayed::try_from(parts).err(),
        Some(RelayedError::OrganisationWithoutAccessClass)
    );
}

#[test]
fn a_federation_professional_under_an_agreement_constructs() {
    let parts = RelayedParts {
        via: Via::Federation {
            issuer: relayed::FEDERATION.to_owned(),
        },
        agreement: Some(agreement(None)),
        ..contact_point()
    };
    let relayed = Relayed::try_from(parts).expect("a federation professional");
    assert_eq!(
        relayed
            .as_parts()
            .agreement
            .as_ref()
            .map(|a| a.name.as_str()),
        Some(relayed::AGREEMENT)
    );
}

/// eHDSI SAML Profile 10.0.0 §2.3: the `organization-id` "SHALL be an OID
/// (urn:oid:1.2.3.4) or HL7II (urn:hl7ii:1.2.3.4:XYZ)".
#[test]
fn a_contact_point_provider_named_by_no_oid_or_hl7ii_is_refused() {
    for identifier in ["2.999.56", "https://example.org/hcp", ""] {
        let mut parts = contact_point();
        parts.provider.identifier = identifier.to_owned();
        assert_eq!(
            Relayed::try_from(parts).err(),
            Some(RelayedError::ProviderIdentifierForm),
            "{identifier}"
        );
    }
    let mut parts = contact_point();
    parts.provider.identifier = "urn:hl7ii:2.999.56:Qz7".to_owned();
    assert!(Relayed::try_from(parts).is_ok());
}

/// eHDSI SAML Profile 10.0.0 §2.3: `on-behalf-of` is "Mandatory if a
/// structural role is part of the category 'Clerical Support Workers'", the
/// ISCO `44`.
#[test]
fn a_clerical_role_without_whom_it_acts_for_is_refused() {
    let mut parts = contact_point();
    let professional = parts.professional.as_mut().expect("a professional");
    professional.roles = vec![Coded {
        system: Some(relayed::ISCO.to_owned()),
        code: "44".to_owned(),
    }];
    assert_eq!(
        Relayed::try_from(parts.clone()).err(),
        Some(RelayedError::OnBehalfOfMissing)
    );
    parts
        .professional
        .as_mut()
        .expect("a professional")
        .on_behalf_of = Some(Coded {
        system: Some(relayed::ISCO.to_owned()),
        code: relayed::ROLE.to_owned(),
    });
    assert!(Relayed::try_from(parts).is_ok());
}

/// eHDSI Audit Trail Profile 10.1.0 §2.3.41: `SPProvidedID<NameID@Issuer>`.
#[test]
fn the_name_id_is_encoded_as_the_audit_trail_profile_asks() {
    let parts = contact_point();
    let name_id = parts
        .professional
        .and_then(|professional| professional.name_id)
        .expect("a name id");
    assert_eq!(
        name_id.user_id(),
        format!(
            "{}<{}@{}>",
            relayed::SP_PROVIDED_ID,
            relayed::NAME_ID,
            relayed::IDA_ISSUER
        )
    );
    let bare = ehds_logging::record::NameId {
        sp_provided_id: None,
        ..name_id
    };
    assert_eq!(
        bare.user_id(),
        format!("<{}@{}>", relayed::NAME_ID, relayed::IDA_ISSUER)
    );
}

/// eHDSI SAML Profile 10.0.0 §2.1: the seven two-factor
/// `AuthnContextClassRef` classes.
#[test]
fn the_authentication_methods_are_the_seven_two_factor_classes() {
    let classes = [
        (
            AuthenticationMethod::MobileTwoFactorUnregistered,
            "MobileTwoFactorUnregistered",
        ),
        (
            AuthenticationMethod::MobileTwoFactorContract,
            "MobileTwoFactorContract",
        ),
        (AuthenticationMethod::X509, "X509"),
        (AuthenticationMethod::Spki, "SPKI"),
        (AuthenticationMethod::SmartcardPki, "SmartcardPKI"),
        (AuthenticationMethod::SoftwarePki, "SoftwarePKI"),
        (AuthenticationMethod::TlsClient, "TLSClient"),
    ];
    for (method, class) in classes {
        let uri = format!("urn:oasis:names:tc:SAML:2.0:ac:classes:{class}");
        assert_eq!(method.uri(), uri);
        assert_eq!(AuthenticationMethod::from_uri(&uri), Some(method));
    }
    assert_eq!(
        AuthenticationMethod::from_uri("urn:oasis:names:tc:SAML:2.0:ac:classes:Password"),
        None
    );
    assert_eq!(
        AuthenticationMethod::from_uri("urn:oasis:names:tc:SAML:2.0:ac:classes:PreviousSession"),
        None
    );
}

#[test]
fn each_binding_outcome_has_its_code_and_names_its_party_where_it_has_one() {
    let person = party("Qz7-party-69");
    let cases = [
        (bound(false), "bound", true, false),
        (
            Binding::Provisional {
                party: ehds_logging::record::BoundParty {
                    party: person.clone(),
                    groups: Vec::new(),
                },
            },
            "provisional",
            true,
            false,
        ),
        (Binding::External, "external", false, false),
        (Binding::Organisation, "organisation", false, false),
        (Binding::System, "system", false, false),
        (Binding::PartySelf, "self", false, false),
        (Binding::RefusedUnbound, "refused-unbound", false, true),
        (Binding::RefusedAmbiguous, "refused-ambiguous", false, true),
        (
            Binding::RefusedInactive {
                party: person.clone(),
            },
            "refused-inactive",
            true,
            true,
        ),
        (
            Binding::RefusedAbandoned {
                party: person.clone(),
            },
            "refused-abandoned",
            true,
            true,
        ),
        (
            Binding::RefusedKeyClaimMissing {
                claim: "oid".to_owned(),
            },
            "refused-key-claim-missing",
            false,
            true,
        ),
        (
            Binding::RefusedGroupsUnavailable,
            "refused-groups-unavailable",
            false,
            true,
        ),
        (Binding::RefusedGuest, "refused-guest", false, true),
        (
            Binding::RefusedExternalIncomplete {
                fields: vec!["hp_identifier".to_owned()],
            },
            "refused-external-incomplete",
            false,
            true,
        ),
        (
            Binding::RefusedTrcPatientMismatch,
            "refused-trc-patient-mismatch",
            false,
            true,
        ),
    ];
    for (binding, code, has_party, refused) in cases {
        assert_eq!(binding.code(), code);
        assert_eq!(binding.party().is_some(), has_party, "{code}");
        assert_eq!(binding.is_refusal(), refused, "{code}");
        assert_eq!(
            binding.is_security_event(),
            code == "refused-trc-patient-mismatch",
            "{code}"
        );
    }
}

#[test]
fn a_bound_person_names_their_groups_and_a_guest_mark() {
    let binding = bound(true);
    assert_eq!(
        binding.groups(),
        [group("Qz7-group-67"), group("Qz7-group-68")]
    );
    assert!(binding.is_guest());
    assert!(!bound(false).is_guest());
    assert_eq!(Binding::RefusedGuest.groups(), []);
}

#[test]
fn the_acting_modes_have_their_codes() {
    for (acting, code) in [
        (Acting::Person, "person"),
        (Acting::Client, "client"),
        (Acting::Delegated, "delegated"),
        (Acting::External, "external"),
        (Acting::System, "system"),
    ] {
        assert_eq!(acting.code(), code);
    }
}

#[test]
fn a_correlation_source_is_attested_unless_it_is_a_header() {
    for (source, code, attested) in [
        (CorrelationSource::TokenClaim, "token-claim", true),
        (
            CorrelationSource::AuthorizationDetails,
            "authorization-details",
            true,
        ),
        (CorrelationSource::Header, "header", false),
    ] {
        assert_eq!(source.code(), code);
        assert_eq!(source.is_attested(), attested, "{code}");
    }
    for (class, code) in [
        (AccessClass::Automated, "automated"),
        (AccessClass::Chained, "chained"),
    ] {
        assert_eq!(class.code(), code);
    }
}

/// A `chained` organisation access is recorded only with a correlation id
/// the node's authorization server attested, so the record joins the
/// requester's own log (no specification governs this: our own design).
#[test]
fn a_chained_organisation_record_needs_an_attested_correlation() {
    let mut record = super::balp_free::record();
    record.accessor.relayed =
        Some(Relayed::try_from(organisation(AccessClass::Chained)).expect("an organisation"));
    assert_eq!(
        record.validate(),
        Err(InvalidRecord::ChainedWithoutAttestedCorrelation)
    );
    record.request.correlation = Some(Correlation {
        value: "Qz7-correlation-70".to_owned(),
        source: CorrelationSource::Header,
    });
    assert_eq!(
        record.validate(),
        Err(InvalidRecord::ChainedWithoutAttestedCorrelation)
    );
    record.request.correlation = Some(Correlation {
        value: "Qz7-correlation-70".to_owned(),
        source: CorrelationSource::TokenClaim,
    });
    assert_eq!(record.validate(), Ok(()));
    record.accessor.relayed =
        Some(Relayed::try_from(organisation(AccessClass::Automated)).expect("an organisation"));
    record.request.correlation = None;
    assert_eq!(record.validate(), Ok(()));
}

#[test]
fn no_debug_shows_a_relayed_or_bound_value() {
    let mut record = super::balp_free::record();
    record.accessor.relayed = Some(Relayed::try_from(contact_point()).expect("a relayed access"));
    record.accessor.binding = Some(bound(true));
    record.accessor.actors = vec![ehds_logging::record::Actor {
        issuer: Some(relayed::FEDERATION.to_owned()),
        subject: "Qz7-actor-71".to_owned(),
    }];
    record.accessor.represents = Some(ehds_logging::record::Representation {
        represented: PartyRef {
            id: "Qz7-represented-72".to_owned(),
            ..party("")
        },
        ground: Coded {
            system: None,
            code: "Qz7-ground-73".to_owned(),
        },
    });
    record.request.correlation = Some(Correlation {
        value: "Qz7-correlation-74".to_owned(),
        source: CorrelationSource::TokenClaim,
    });
    let shown = format!("{record:?}");
    for value in [
        relayed::NAME,
        relayed::FAMILY,
        relayed::GIVEN,
        relayed::HP_ID,
        relayed::HP_AUTHORITY,
        relayed::HP_PROVIDER,
        relayed::NAME_ID,
        relayed::SP_PROVIDED_ID,
        relayed::HCP_AUTHORITY,
        relayed::HCP_NAME,
        relayed::HCP_ADDRESS,
        relayed::POINT_OF_CARE,
        relayed::IDA,
        relayed::TRC,
        relayed::MESSAGE,
        "Qz7-party-66",
        "Qz7-group-67",
        "Qz7-actor-71",
        "Qz7-represented-72",
        "Qz7-correlation-74",
    ] {
        assert!(!shown.contains(value), "{value} in {shown}");
    }
    assert!(shown.contains("Bound"), "{shown}");
}
