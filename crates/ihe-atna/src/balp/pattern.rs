// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The BALP 1.1.4 event patterns beside the `RESTful` ones.
//!
//! They are a consent authorization decision (§3:5.7.6), a privacy
//! disclosure at its source and at its recipient (§3:5.7.7), and the IUA Get
//! Access Token \[ITI-71\] (`IHE.IUA.71`).
//!
//! Each constructor returns the [`EventKind`] of its pattern; the
//! [`Exchange`](super::Exchange) built on it carries the agents and entities
//! the pattern asks for, as each constructor's documentation names them. The
//! token-use patterns (§3:5.7.4 and §3:5.7.5) are claimed from the user's
//! [`AccessToken`](crate::user::AccessToken) on any event.

use super::{
    APPLICATION, Coded, DCM, DESTINATION_ROLE, Described, EXPORT, Entity, EventKind,
    IHE_TRANSACTIONS, IMPORT, SOURCE_ROLE, USER_AUTHENTICATION, USER_OAUTH_AGENT, What,
};

/// The BALP pattern of a permit or deny decision a consent decision service
/// made (§3:5.7.6).
pub const AUTHZ_CONSENT: &str =
    "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.AuthZconsent";

/// The BALP pattern of a disclosure, recorded by the system that disclosed
/// the data (§3:5.7.7).
pub const PRIVACY_DISCLOSURE_SOURCE: &str =
    "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.PrivacyDisclosure.Source";

/// The BALP pattern of a disclosure, recorded by the system that received
/// the data (§3:5.7.7).
pub const PRIVACY_DISCLOSURE_RECIPIENT: &str = "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.BasicAudit.PrivacyDisclosure.Recipient";

/// The audit pattern of the IUA Get Access Token transaction \[ITI-71\].
pub const IUA_GET_ACCESS_TOKEN: &str =
    "https://profiles.ihe.net/ITI/BALP/StructureDefinition/IHE.IUA.71";

/// `AuditEvent.type` of an authorization decision: DICOM `110113`.
pub const SECURITY_ALERT: Coded = Coded::fixed(DCM, "110113", "Security Alert");

/// The subtype of a decision on roles and consent (BALP `AuthZsubType`).
pub const AUTHZ_CONSENT_DECISION: Coded = Coded::fixed(
    "https://profiles.ihe.net/ITI/BALP/CodeSystem/AuthZsubType",
    "AuthZ-Consent",
    "Authorization Decision using Roles and Consent",
);

/// The agent type of the service that made an authorization decision
/// (`agent:authorizer`).
pub const AUTHORIZATION_SERVER: Coded = Coded::fixed(
    "http://terminology.hl7.org/CodeSystem/extra-security-role-type",
    "authserver",
    "authorization server",
);

/// The subtype of a disclosure (`subtype:disclosure`, ISO 21089).
pub const DISCLOSE: Coded = Coded::fixed(
    "http://terminology.hl7.org/CodeSystem/iso-21089-lifecycle",
    "disclose",
    "Disclose Record Lifecycle Event",
);

/// The subtype of a Get Access Token transaction.
pub const ITI_71: Coded = Coded::fixed(IHE_TRANSACTIONS, "ITI-71", "Get Access Token");

/// The agent type of an OAuth authorization server (BALP `UserAgentTypes`,
/// `agent:auth-server` of `IHE.IUA.71`).
pub const AUTHZ_OAUTH_SERVICE: Coded = Coded::fixed(
    "https://profiles.ihe.net/ITI/BALP/CodeSystem/UserAgentTypes",
    "AuthzOauthService",
    "OAuth Authorization Server",
);

/// The entity type of a consent (`entity:consent`): the FHIR resource type
/// `Consent`.
pub const CONSENT: Coded = Coded::fixed("http://hl7.org/fhir/resource-types", "Consent", "Consent");

impl EventKind {
    /// The kind of a consent authorization decision (`IHE.BasicAudit.AuthZconsent`):
    /// type `110113`, subtype `AuthZ-Consent`, action `E`.
    ///
    /// The exchange is [`Direction::Decided`](super::Direction::Decided) for the
    /// client application that asked, made for a user who names the
    /// organisation they act for and no alternative id, and its entities are
    /// the patient, each consent the decision rested on
    /// ([`Entity::consent`]) and, when known, the token
    /// ([`Entity::token`]), since the pattern admits no other entity.
    #[must_use]
    pub fn authz_consent() -> Self {
        Self {
            profile: AUTHZ_CONSENT,
            event_type: SECURITY_ALERT,
            subtypes: vec![AUTHZ_CONSENT_DECISION],
            action: "E",
            client: APPLICATION,
            server: AUTHORIZATION_SERVER,
        }
    }

    /// The kind of a disclosure the recording system made
    /// (`IHE.BasicAudit.PrivacyDisclosure.Source`): type `110106` Export,
    /// subtype `disclose` followed by `subtypes`, action `action`.
    ///
    /// The exchange is [`Direction::Sent`](super::Direction::Sent) to the
    /// recipient, and its entities are the patient and the data disclosed.
    #[must_use]
    pub fn privacy_disclosure_source(action: &'static str, subtypes: Vec<Coded>) -> Self {
        Self::disclosure(PRIVACY_DISCLOSURE_SOURCE, EXPORT, action, subtypes)
    }

    /// The kind of a disclosure the recording system received
    /// (`IHE.BasicAudit.PrivacyDisclosure.Recipient`): type `110107` Import,
    /// subtype `disclose` followed by `subtypes`, action `action`.
    ///
    /// The exchange is [`Direction::Received`](super::Direction::Received) from
    /// the source, and its entities are the patient and the data disclosed.
    #[must_use]
    pub fn privacy_disclosure_recipient(action: &'static str, subtypes: Vec<Coded>) -> Self {
        Self::disclosure(PRIVACY_DISCLOSURE_RECIPIENT, IMPORT, action, subtypes)
    }

    fn disclosure(
        profile: &'static str,
        event_type: Coded,
        action: &'static str,
        subtypes: Vec<Coded>,
    ) -> Self {
        Self {
            profile,
            event_type,
            subtypes: std::iter::once(DISCLOSE).chain(subtypes).collect(),
            action,
            client: SOURCE_ROLE,
            server: DESTINATION_ROLE,
        }
    }

    /// The kind of a Get Access Token \[ITI-71\] (`IHE.IUA.71`): type `110114`
    /// User Authentication, subtype `ITI-71` alone, action `E`.
    ///
    /// The exchange is [`Direction::Received`](super::Direction::Received) by
    /// the authorization server from the client application, made for the
    /// user the token is for, who names no organisation, or for the system,
    /// since the pattern admits no other agent. Its one entity is the token
    /// request as sent, without its `code_verifier` ([`Entity::Query`]):
    /// `IHE.IUA.71` caps `entity` at one, so its `token-response` slice is
    /// never written.
    #[must_use]
    pub fn iua_get_access_token() -> Self {
        Self {
            profile: IUA_GET_ACCESS_TOKEN,
            event_type: USER_AUTHENTICATION,
            subtypes: vec![ITI_71],
            action: "E",
            client: APPLICATION,
            server: AUTHZ_OAUTH_SERVICE,
        }
    }
}

impl Entity {
    /// A consent an authorization decision rested on, by its literal
    /// reference such as `Consent/<id>` (`entity:consent`).
    #[must_use]
    pub fn consent(reference: String) -> Self {
        Self::Described(Described {
            what: Some(What::Reference {
                reference,
                resource_type: Some("Consent".to_owned()),
            }),
            kind: CONSENT,
            role: None,
            name: None,
            description: None,
            details: Vec::new(),
            security_labels: Vec::new(),
        })
    }

    /// The OAuth token a decision was made for, by its `jti`
    /// (`entity:token` of `IHE.BasicAudit.AuthZconsent`).
    #[must_use]
    pub fn token(jti: String) -> Self {
        Self::Described(Described {
            what: Some(What::Identifier {
                system: None,
                value: jti,
            }),
            kind: USER_OAUTH_AGENT,
            role: None,
            name: None,
            description: None,
            details: Vec::new(),
            security_labels: Vec::new(),
        })
    }
}
