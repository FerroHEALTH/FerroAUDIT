// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! What a contact point or a federation relayed (Implementing Regulation
//! (EU) 2026/2099 Annex Tables 1 and 2, the eHDSI SAML Profile 10.0.0 §2.3
//! attributes), written in an entity of its own and marked asserted, with
//! the agreement it was made under and the correlation identifier with its
//! source. BALP 1.1.4 §3:5.7.5.4 maps none of these to an element of its own
//! for a person the token does not name, so each is a `detail`.

use ehds_logging::balp::exchange;
use ehds_logging::record::{
    AccessClass, AccessRecord, Action, Correlation, CorrelationSource, Outcome, Relayed,
};

use super::{details, entity, gateway, record, written};
use crate::support::relayed::*;
use crate::support::{contact_point, organisation};

const CORRELATION: &str = "Qz7-correlation-60";

/// A record of an access a contact point relayed, with the correlation
/// identifier its token attested.
fn relayed_record() -> AccessRecord {
    let mut record = record(Action::Query, Outcome::Success);
    record.accessor.relayed = Some(Relayed::try_from(contact_point()).expect("a relayed access"));
    record.request.correlation = Some(Correlation {
        value: CORRELATION.to_owned(),
        source: CorrelationSource::TokenClaim,
    });
    record
}

fn relayed_entity(record: &AccessRecord) -> serde_json::Value {
    let written = written(record);
    let relayed = entity(&written, "ehds-relayed");
    let [relayed] = relayed.as_slice() else {
        panic!("one relayed entity, got {relayed:?}");
    };
    (*relayed).clone()
}

/// Every relayed member is written, beside the issuer that asserted it, how
/// it reached the recording system and the mark that it is asserted.
#[test]
fn every_relayed_member_rides_in_its_own_entity_marked_asserted() {
    let relayed = relayed_entity(&relayed_record());
    assert_eq!(relayed["type"]["code"], "4");
    let encoded = format!("{SP_PROVIDED_ID}<{NAME_ID}@{IDA_ISSUER}>");
    for (kind, value) in [
        ("relayed-via", "contact-point"),
        ("relayed-issuer", CONTACT_POINT),
        ("asserted", "true"),
        ("country-code", "XA"),
        ("professional-named", "true"),
        ("hp-name", NAME),
        ("hp-family-name", FAMILY),
        ("hp-given-name", GIVEN),
        ("hp-identifier", HP_ID),
        ("hp-issuing-authority", HP_AUTHORITY),
        ("hp-provider-identifier", HP_PROVIDER),
        ("hp-name-id", encoded.as_str()),
        (
            "hp-authentication-method",
            "urn:oasis:names:tc:SAML:2.0:ac:classes:SmartcardPKI",
        ),
        ("hp-permission", PERMISSION),
        ("provider-identifier", HCP_ID),
        ("provider-issuing-authority", HCP_AUTHORITY),
        ("provider-name", HCP_NAME),
        ("provider-address", HCP_ADDRESS),
        ("provider-home-community-id", HOME_COMMUNITY),
        ("point-of-care", POINT_OF_CARE),
        ("identity-assertion-id", IDA),
        ("trc-assertion-id", TRC),
        ("request-message-id", MESSAGE),
        ("consumer-ncp", CONSUMER_NCP),
        ("provider-ncp", PROVIDER_NCP),
    ] {
        assert_eq!(details(&relayed, kind), [value], "{kind}");
    }
    assert_eq!(
        details(&relayed, "hp-professional-role"),
        [format!("{ISCO}|{ROLE}")]
    );
    assert_eq!(
        details(&relayed, "provider-facility-type"),
        [format!("{FACILITY_SYSTEM}|{FACILITY}")]
    );
    for absent in [
        "hp-on-behalf-of",
        "provider-organisation-type",
        "agreement-name",
    ] {
        assert_eq!(details(&relayed, absent), Vec::<&str>::new(), "{absent}");
    }
}

/// eHDSI Section I Security Policies 10.0.0 SR 5: the record keeps the
/// assertion and message ids, never a security header.
#[test]
fn the_record_carries_no_security_header() {
    let written = written(&relayed_record()).to_string();
    assert!(!written.contains("securityheader"), "{written}");
    assert!(!written.contains("Assertion>"), "{written}");
}

/// An organisation without a named person is recorded as such, with the
/// agreement, its legal ground, the consent decision and the access class.
#[test]
fn an_organisation_access_names_no_person_and_carries_its_agreement() {
    let mut record = record(Action::Read, Outcome::Success);
    record.accessor.relayed =
        Some(Relayed::try_from(organisation(AccessClass::Chained)).expect("an organisation"));
    record.request.correlation = Some(Correlation {
        value: CORRELATION.to_owned(),
        source: CorrelationSource::AuthorizationDetails,
    });
    let relayed = relayed_entity(&record);
    for (kind, value) in [
        ("relayed-via", "federation"),
        ("relayed-issuer", FEDERATION),
        ("professional-named", "false"),
        ("agreement-name", AGREEMENT),
        ("agreement-version", AGREEMENT_VERSION),
        ("agreement-consent", CONSENT),
        ("agreement-access-class", "chained"),
    ] {
        assert_eq!(details(&relayed, kind), [value], "{kind}");
    }
    assert_eq!(
        details(&relayed, "agreement-legal-ground"),
        ["https://example.org/gdpr|art-9-2-h", GROUND]
    );
    assert_eq!(
        details(&relayed, "provider-organisation-type"),
        [format!(
            "https://example.org/organisation-type|{ORGANISATION_TYPE}"
        )]
    );
    assert_eq!(details(&relayed, "hp-name"), Vec::<&str>::new());
    assert_eq!(details(&relayed, "hp-identifier"), Vec::<&str>::new());
}

/// The correlation identifier is a detail of the request id's entity, with
/// where the recording system read it, so the record joins the client's
/// own log.
#[test]
fn the_correlation_identifier_rides_with_the_request_id_and_its_source() {
    let written = written(&relayed_record());
    let transaction = written["entity"]
        .as_array()
        .expect("entities")
        .iter()
        .find(|entity| entity["type"]["code"] == "XrequestId")
        .expect("entity:transaction");
    assert_eq!(details(transaction, "correlation-id"), [CORRELATION]);
    assert_eq!(details(transaction, "correlation-source"), ["token-claim"]);
}

#[test]
fn a_record_no_contact_point_relayed_has_no_relayed_entity() {
    let written = written(&record(Action::Query, Outcome::Success));
    assert_eq!(
        entity(&written, "ehds-relayed"),
        Vec::<&serde_json::Value>::new()
    );
    assert!(!written.to_string().contains("correlation-id"));
}

#[test]
fn no_debug_shows_a_relayed_value() {
    let record = relayed_record();
    let shown = format!("{record:?} {:?}", exchange(&record, &gateway()));
    for value in [
        NAME,
        FAMILY,
        GIVEN,
        HP_ID,
        HP_AUTHORITY,
        HP_PROVIDER,
        NAME_ID,
        HCP_AUTHORITY,
        HCP_NAME,
        HCP_ADDRESS,
        POINT_OF_CARE,
        IDA,
        TRC,
        MESSAGE,
        CORRELATION,
    ] {
        assert!(!shown.contains(value), "{value} in {shown}");
    }
}
