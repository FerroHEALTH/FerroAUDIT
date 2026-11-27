// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! A record of each BALP 1.1.4 pattern beside the `RESTful` ones, held to
//! the slices of its profile and written for the HL7 FHIR Validator lane:
//! the consent decision (§3:5.7.6), the privacy disclosure at its source and
//! at its recipient (§3:5.7.7), the IUA Get Access Token (`IHE.IUA.71`), and
//! the OAuth (§3:5.7.5) and SAML (§3:5.7.4) token-use patterns.
#![expect(
    clippy::disallowed_types,
    reason = "the test seam: the written record is read as a JSON value"
)]

use std::net::{IpAddr, Ipv4Addr};

use ihe_atna::balp::pattern::{
    AUTHZ_CONSENT, IUA_GET_ACCESS_TOKEN, PRIVACY_DISCLOSURE_RECIPIENT, PRIVACY_DISCLOSURE_SOURCE,
};
use ihe_atna::balp::{
    DESTINATION_ROLE, DOMAIN_RESOURCE, Direction, Entity, EventKind, Exchange, NetworkAddress,
    OAUTH_COMPREHENSIVE, OAUTH_MINIMAL, OAUTH_OPAQUE, Outcome, Peer, READ, REST,
    SAML_COMPREHENSIVE, SAML_MINIMAL, SOURCE_ROLE, SYSTEM_OBJECT,
};
use ihe_atna::user::{AccessToken, Code, OnBehalfOf, PersonId, User};
use secrecy::{ExposeSecret as _, SecretString};
use serde_json::Value;
use url::Url;

use super::{CLIENT, ISSUER, SUBJECT, emit, observer};

/// The synthetic values of these records, each unlike any other text.
const PATIENT: &str = "Qz7-patient-71";
const JTI: &str = "Qz7-jti-72";
const ORGANISATION: &str = "Qz7-organisation-73";
const ORGANISATION_NAME: &str = "Qz7 Organisation 74";
const ASSERTION: &str = "_Qz7-assertion-75";
const NPI: &str = "Qz7-npi-76";

fn read_kind() -> EventKind {
    EventKind {
        profile: "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.Read",
        event_type: REST,
        subtypes: vec![READ],
        action: "R",
        client: DESTINATION_ROLE,
        server: SOURCE_ROLE,
    }
}

fn patient() -> Entity {
    Entity::Patient {
        system: Some("urn:oid:2.999.7".to_owned()),
        value: SecretString::from(PATIENT),
        namespace: None,
    }
}

fn document() -> Entity {
    Entity::Resource {
        reference: Some("DocumentReference/Qz7-document-77".to_owned()),
        resource_type: "DocumentReference",
        kind: SYSTEM_OBJECT,
        role: Some(DOMAIN_RESOURCE),
        name: None,
        query: None,
    }
}

fn app() -> Peer {
    Peer {
        who: CLIENT.to_owned(),
        network: NetworkAddress::Ip(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 71))),
    }
}

fn endpoint() -> NetworkAddress {
    NetworkAddress::uri(&Url::parse("https://gateway.example.org/fhir").expect("a URL"))
}

fn exchange(kind: EventKind, direction: Direction, on_behalf: OnBehalfOf) -> Exchange {
    Exchange {
        kind,
        recorded: "2026-11-27T10:00:00Z".parse().expect("a timestamp"),
        outcome: Outcome::Success,
        outcome_description: None,
        direction,
        on_behalf,
        entities: vec![patient(), document()],
        security: Vec::new(),
    }
}

/// A read of a patient's document, received from the client application,
/// made for `user`.
fn read_for(user: User) -> Exchange {
    exchange(
        read_kind(),
        Direction::Received {
            client: app(),
            endpoint: endpoint(),
        },
        OnBehalfOf::User(user),
    )
}

/// The record of `exchange`, also written for the validator lane.
fn written(exchange: &Exchange) -> Value {
    let record = exchange
        .audit_event(&observer())
        .expect("the record is written")
        .into_bytes();
    emit(record.expose_secret());
    serde_json::from_slice(record.expose_secret()).expect("the record is JSON")
}

fn profiles(record: &Value) -> Vec<&str> {
    record["meta"]["profile"]
        .as_array()
        .map(|profiles| profiles.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

/// The agent whose `type` carries the code `code`.
fn agent<'a>(record: &'a Value, code: &str) -> &'a Value {
    record["agent"]
        .as_array()
        .expect("agents")
        .iter()
        .find(|agent| {
            agent["type"]["coding"]
                .as_array()
                .is_some_and(|codings| codings.iter().any(|coding| coding["code"] == code))
        })
        .unwrap_or_else(|| panic!("an agent of type {code}: {record}"))
}

fn types(agent: &Value) -> Vec<&str> {
    agent["type"]["coding"]
        .as_array()
        .expect("codings")
        .iter()
        .filter_map(|coding| coding["code"].as_str())
        .collect()
}

/// BALP 1.1.4 §3:5.7.6: a consent decision is `110113` `AuthZ-Consent`, with
/// the client application, the user, the user's organisation typed `PROV`
/// and the deciding service typed `authserver` with no network, and the
/// patient, the consent and the token as its only entities.
#[test]
fn authz_consent_names_the_client_the_user_the_organisation_and_the_authorizer() {
    let user = User::new(ISSUER.to_owned(), SUBJECT.to_owned(), CLIENT.to_owned())
        .with_organisation(Some(ORGANISATION.to_owned()));
    let mut decision = exchange(
        EventKind::authz_consent(),
        Direction::Decided { client: app() },
        OnBehalfOf::User(user),
    );
    decision.entities = vec![
        patient(),
        Entity::consent("Consent/Qz7-consent-78".to_owned()),
        Entity::token(JTI.to_owned()),
    ];
    let record = written(&decision);
    assert_eq!(profiles(&record), [AUTHZ_CONSENT]);
    assert_eq!(record["type"]["code"], "110113");
    assert_eq!(record["subtype"][0]["code"], "AuthZ-Consent");
    let authorizer = agent(&record, "authserver");
    assert_eq!(authorizer["who"]["display"], "gateway.example.org");
    assert!(authorizer.get("network").is_none(), "{record}");
    assert_eq!(agent(&record, "110150")["network"]["address"], "192.0.2.71");
    assert_eq!(agent(&record, "IRCP")["requestor"], true);
    assert_eq!(
        agent(&record, "PROV")["who"]["identifier"]["value"],
        ORGANISATION
    );
    let entities: Vec<&str> = record["entity"]
        .as_array()
        .expect("entities")
        .iter()
        .filter_map(|entity| entity["type"]["code"].as_str())
        .collect();
    assert_eq!(entities, ["1", "Consent", "UserOauthAgent"]);
}

/// BALP 1.1.4 §3:5.7.7: the system that discloses data records an Export
/// `110106` with the `disclose` subtype, itself the source and the receiver
/// the recipient, each with its network address.
#[test]
fn privacy_disclosure_source_is_an_export_from_the_recording_system() {
    let disclosure = exchange(
        EventKind::privacy_disclosure_source("R", vec![READ]),
        Direction::Sent {
            server: Peer::server(&Url::parse("https://recipient.example.org/fhir").expect("a URL")),
        },
        OnBehalfOf::System,
    );
    let record = written(&disclosure);
    assert_eq!(profiles(&record), [PRIVACY_DISCLOSURE_SOURCE]);
    assert_eq!(record["type"]["code"], "110106");
    assert_eq!(record["subtype"][0]["code"], "disclose");
    assert_eq!(record["subtype"][1]["code"], "read");
    assert_eq!(
        agent(&record, "110153")["who"]["display"],
        "gateway.example.org"
    );
    assert_eq!(
        agent(&record, "110152")["network"]["address"],
        "https://recipient.example.org/fhir"
    );
}

/// BALP 1.1.4 §3:5.7.7: the system that receives disclosed data records an
/// Import `110107` with the `disclose` subtype, the sender the source and
/// itself the recipient.
#[test]
fn privacy_disclosure_recipient_is_an_import_by_the_recording_system() {
    let disclosure = exchange(
        EventKind::privacy_disclosure_recipient("C", Vec::new()),
        Direction::Received {
            client: app(),
            endpoint: endpoint(),
        },
        OnBehalfOf::System,
    );
    let record = written(&disclosure);
    assert_eq!(profiles(&record), [PRIVACY_DISCLOSURE_RECIPIENT]);
    assert_eq!(record["type"]["code"], "110107");
    assert_eq!(record["subtype"][0]["code"], "disclose");
    assert_eq!(agent(&record, "110153")["who"]["display"], CLIENT);
    assert_eq!(
        agent(&record, "110152")["network"]["address"],
        "https://gateway.example.org/fhir"
    );
}

/// `IHE.IUA.71`: the authorization server records a Get Access Token as
/// `110114` with the subtype `ITI-71` alone, the client application and
/// itself typed `AuthzOauthService`, the user, and the token request as its
/// one entity.
#[test]
fn iua_71_records_the_token_request_at_the_authorization_server() {
    let mut token = exchange(
        EventKind::iua_get_access_token(),
        Direction::Received {
            client: app(),
            endpoint: NetworkAddress::uri(
                &Url::parse("https://auth.example.org/token").expect("a URL"),
            ),
        },
        OnBehalfOf::User(User::local(SUBJECT.to_owned())),
    );
    token.entities = vec![Entity::Query(SecretString::from(
        "POST /token grant_type=authorization_code&client_id=Qz7-client-32",
    ))];
    let record = written(&token);
    assert_eq!(profiles(&record), [IUA_GET_ACCESS_TOKEN]);
    assert_eq!(record["type"]["code"], "110114");
    assert_eq!(record["subtype"].as_array().map(Vec::len), Some(1));
    assert_eq!(record["subtype"][0]["code"], "ITI-71");
    assert_eq!(
        agent(&record, "AuthzOauthService")["network"]["address"],
        "https://auth.example.org/token"
    );
    assert_eq!(record["agent"].as_array().map(Vec::len), Some(3));
    assert_eq!(record["entity"].as_array().map(Vec::len), Some(1));
    assert_eq!(record["entity"][0]["role"]["code"], "24");
}

/// BALP 1.1.4 §3:5.7.5.2: a JWT whose client the record cannot name claims
/// the Minimal pattern: its `jti` is the user's one policy and the user
/// agent is typed `UserOauthAgent` beside `IRCP`.
#[test]
fn oauth_minimal_is_a_jwt_whose_client_is_not_named() {
    let user = User::local(SUBJECT.to_owned())
        .with_issuer(Some(ISSUER.to_owned()))
        .with_token(Some(AccessToken::Jwt(JTI.to_owned())));
    let record = written(&read_for(user));
    let claimed = profiles(&record);
    assert!(claimed.contains(&OAUTH_MINIMAL), "{record}");
    assert!(!claimed.contains(&OAUTH_COMPREHENSIVE), "{record}");
    let user = agent(&record, "IRCP");
    assert_eq!(types(user), ["IRCP", "UserOauthAgent"]);
    assert_eq!(user["policy"], serde_json::json!([JTI]));
}

/// BALP 1.1.4 §3:5.7.5.1: an opaque token claims the Opaque pattern by its
/// last 32 characters at most, never the whole token.
#[test]
fn oauth_opaque_keeps_the_last_32_characters_of_the_token() {
    let opaque = format!("Qz7-opaque-{}", "x".repeat(40));
    let token = AccessToken::opaque(&opaque);
    assert_eq!(token.policy().chars().count(), AccessToken::OPAQUE_TAIL);
    assert!(opaque.ends_with(token.policy()));
    let user = User::local(SUBJECT.to_owned()).with_token(Some(token));
    let record = written(&read_for(user));
    assert!(profiles(&record).contains(&OAUTH_OPAQUE), "{record}");
    let user = agent(&record, "IRCP");
    assert_eq!(types(user), ["IRCP", "UserOauthAgent"]);
    assert_eq!(user["policy"], serde_json::json!(["x".repeat(32)]));
    assert!(!record.to_string().contains("Qz7-opaque"), "{record}");
}

/// A token shorter than the tail is kept whole, and `Debug` shows no token.
#[test]
fn a_short_opaque_token_is_kept_whole_and_debug_shows_none() {
    let token = AccessToken::opaque("Qz7-short-79");
    assert_eq!(token.policy(), "Qz7-short-79");
    assert!(!format!("{token:?}").contains("Qz7"), "{token:?}");
}

/// BALP 1.1.4 §3:5.7.4.1: a SAML assertion claims the Minimal pattern: its
/// `ID` is the user's one policy, the user is named by the `Issuer` and the
/// `NameID`, and typed `UserSamlAgent` beside `IRCP`.
#[test]
fn saml_minimal_names_the_user_by_issuer_and_name_id() {
    let user = User::local(SUBJECT.to_owned())
        .with_issuer(Some(ISSUER.to_owned()))
        .with_token(Some(AccessToken::Saml(ASSERTION.to_owned())));
    let record = written(&read_for(user));
    let claimed = profiles(&record);
    assert!(claimed.contains(&SAML_MINIMAL), "{record}");
    assert!(!claimed.contains(&SAML_COMPREHENSIVE), "{record}");
    let user = agent(&record, "UserSamlAgent");
    assert_eq!(types(user), ["IRCP", "UserSamlAgent"]);
    assert_eq!(user["who"]["identifier"]["system"], ISSUER);
    assert_eq!(user["who"]["identifier"]["value"], SUBJECT);
    assert_eq!(user["policy"], serde_json::json!([ASSERTION]));
}

/// BALP 1.1.4 §3:5.7.4.2: a SAML assertion whose attributes the record
/// keeps beyond the Minimal ones claims the Comprehensive pattern: the
/// assurance level, the `npi` and `provider-identifier`, and the
/// organisation by its identifier and name as `agent:userorg`.
#[test]
fn saml_comprehensive_keeps_the_attributes_and_the_organisation() {
    let user = User::local(SUBJECT.to_owned())
        .with_issuer(Some(ISSUER.to_owned()))
        .with_token(Some(AccessToken::Saml(ASSERTION.to_owned())))
        .with_assurance(Some(Code {
            system: Some("urn:oasis:names:tc:SAML:2.0:ac:classes".to_owned()),
            code: "urn:oasis:names:tc:SAML:2.0:ac:classes:X509".to_owned(),
        }))
        .with_provider_identifier(Some(NPI.to_owned()))
        .with_person_id(Some(PersonId {
            system: Some("urn:oid:2.999.8".to_owned()),
            value: "Qz7-person-80".to_owned(),
        }))
        .with_organisation(Some(ORGANISATION.to_owned()))
        .with_organisation_name(Some(ORGANISATION_NAME.to_owned()));
    let record = written(&read_for(user));
    assert!(profiles(&record).contains(&SAML_COMPREHENSIVE), "{record}");
    let organisation = agent(&record, "PROV");
    assert_eq!(organisation["who"]["identifier"]["value"], ORGANISATION);
    assert_eq!(organisation["who"]["display"], ORGANISATION_NAME);
}

/// `agent:userorg` of the Comprehensive pattern needs the organisation's
/// name, so an organisation named by its identifier alone keeps the record
/// Minimal.
#[test]
fn a_saml_organisation_without_its_name_stays_minimal() {
    let user = User::local(SUBJECT.to_owned())
        .with_token(Some(AccessToken::Saml(ASSERTION.to_owned())))
        .with_organisation(Some(ORGANISATION.to_owned()));
    let record = written(&read_for(user));
    let claimed = profiles(&record);
    assert!(claimed.contains(&SAML_MINIMAL), "{record}");
    assert!(!claimed.contains(&SAML_COMPREHENSIVE), "{record}");
}

/// Each token-use pattern fixes the user's `policy` to `1..1`, so a user who
/// acts under another policy beside the token claims none of them.
#[test]
fn a_policy_beside_the_token_claims_no_token_pattern() {
    for token in [
        AccessToken::Jwt(JTI.to_owned()),
        AccessToken::opaque("Qz7-opaque-81"),
        AccessToken::Saml(ASSERTION.to_owned()),
    ] {
        let user = User::local(SUBJECT.to_owned())
            .with_token(Some(token))
            .with_policies(vec!["Qz7-permit-82".to_owned()]);
        let record = written(&read_for(user));
        assert_eq!(
            profiles(&record).len(),
            1,
            "the Read pattern alone: {record}"
        );
        assert_eq!(types(agent(&record, "IRCP")), ["IRCP"]);
    }
}
