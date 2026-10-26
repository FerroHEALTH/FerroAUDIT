// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! Whom an audited exchange is made for: a user the client authenticated by
//! an OAuth 2.0 access token, or the client's own system with no user.
//!
//! An IHE transaction's audit record names the user who asked for it when one
//! is known: the BALP patterns have an `agent:user` slice, `0..1` (BALP 1.1.4
//! §3:5.7.3), which PIXm §2:3.83.5.2.1 asks to augment "to include agent
//! details from the OAuth Security token" as BALP §3:5.7.5.4 maps them, and the
//! ITI-55 audit message has a Human Requestor participant "if known" (ITI TF-2
//! §3.55.5.1.1). An exchange the system makes on its own behalf names no user.
//!
//! A user authenticated otherwise, by a local account or HTTP Basic, has no
//! token issuer and no client: [`User::local`] names them by their account
//! alone, and the record writes no value the authentication did not give.
//!
//! A [`User`] names a person or an application: `Debug` shows none of it, and a
//! client of this crate writes it into the audit record alone.

use std::fmt;

use crate::redact::REDACTED;

/// Whom an exchange is made for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OnBehalfOf {
    /// A user the client authenticated, who asked for the exchange: the
    /// record names the user and, when an OAuth 2.0 access token named
    /// them, the issuer and the client application it was issued to.
    User(User),
    /// The client's own system, with no user: the record names none, as the
    /// BALP examples of an event no user caused do (`ex-auditBasicReadNoUser`
    /// and the others named `NoUser`).
    System,
}

impl OnBehalfOf {
    /// Returns the user, when the exchange is made for one.
    #[must_use]
    pub fn user(&self) -> Option<&User> {
        match self {
            Self::User(user) => Some(user),
            Self::System => None,
        }
    }
}

/// A user as the verified access token names them (BALP 1.1.4 §3:5.7.5.4).
///
/// `Debug` shows that a user is named and none of the values.
#[derive(Clone, PartialEq, Eq)]
pub struct User {
    issuer: Option<String>,
    subject: String,
    client_id: Option<String>,
    audience: Option<String>,
    purposes: Vec<PurposeOfUse>,
    organisation: Option<String>,
    alt_id: Option<String>,
    stated: Box<Stated>,
}

/// What a token states about its user beyond who they are, boxed so an
/// [`OnBehalfOf`] stays small.
#[derive(Clone, Default, PartialEq, Eq)]
struct Stated {
    name: Option<String>,
    provider_identifier: Option<String>,
    assurance: Option<Code>,
    roles: Vec<Code>,
    token_id: Option<String>,
    policies: Vec<String>,
}

impl User {
    /// Returns the user `subject` (the token's `sub`) at the authorization
    /// server `issuer` (`iss`), who asked through the client application
    /// `client_id` (`client_id`).
    #[must_use]
    pub fn new(issuer: String, subject: String, client_id: String) -> Self {
        Self {
            issuer: Some(issuer),
            client_id: Some(client_id),
            ..Self::local(subject)
        }
    }

    /// Returns the user `subject`, authenticated with no OAuth 2.0 access
    /// token, such as by a local account or HTTP Basic: no issuer vouched for
    /// them and no client application acted for them, so the record writes
    /// neither (BALP 1.1.4 §3:5.7.5.4 maps each token field to an element,
    /// and a field that does not exist writes none).
    #[must_use]
    pub fn local(subject: String) -> Self {
        Self {
            issuer: None,
            subject,
            client_id: None,
            audience: None,
            purposes: Vec::new(),
            organisation: None,
            alt_id: None,
            stated: Box::default(),
        }
    }

    /// Returns this user, named `name`, the IHE IUA `subject_name`, which
    /// BALP 1.1.4 §3:5.7.5.4 writes as `agent[user].who.display`.
    #[must_use]
    pub fn with_name(mut self, name: Option<String>) -> Self {
        self.stated.name = name;
        self
    }

    /// Returns the user's name, when the token states one.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.stated.name.as_deref()
    }

    /// Returns this user, identified by `identifier` as well, the IHE IUA
    /// `national_provider_identifier`, which BALP 1.1.4 §3:5.7.5.4 writes as
    /// `agent[user].extension[otherId][npi]`.
    #[must_use]
    pub fn with_provider_identifier(mut self, identifier: Option<String>) -> Self {
        self.stated.provider_identifier = identifier;
        self
    }

    /// Returns the identifier the user's national authority issued them as a
    /// health care provider, when the token states one.
    #[must_use]
    pub fn provider_identifier(&self) -> Option<&str> {
        self.stated.provider_identifier.as_deref()
    }

    /// Returns this user, authenticated at the assurance level `assurance`
    /// codes, which BALP 1.1.4 writes as `agent[user].extension[assuranceLevel]`.
    #[must_use]
    pub fn with_assurance(mut self, assurance: Option<Code>) -> Self {
        self.stated.assurance = assurance;
        self
    }

    /// Returns the assurance level of the user's authentication, when it is
    /// known.
    #[must_use]
    pub fn assurance(&self) -> Option<&Code> {
        self.stated.assurance.as_ref()
    }

    /// Returns this user, acting under `roles`, which BALP 1.1.4
    /// §3:5.7.5.4 writes as `agent[user].role`, as it does the IHE IUA
    /// `subject_role`.
    #[must_use]
    pub fn with_roles(mut self, roles: Vec<Code>) -> Self {
        self.stated.roles = roles;
        self
    }

    /// Returns every role the user acts under.
    #[must_use]
    pub fn roles(&self) -> &[Code] {
        &self.stated.roles
    }

    /// Returns this user, acting for the organisation `organisation` names,
    /// such as the IHE IUA `subject_organization_id`.
    #[must_use]
    pub fn with_organisation(mut self, organisation: Option<String>) -> Self {
        self.organisation = organisation;
        self
    }

    /// Returns this user, known by `alt_id` as well, such as a professional
    /// registration number the token states beside its `sub`.
    #[must_use]
    pub fn with_alt_id(mut self, alt_id: Option<String>) -> Self {
        self.alt_id = alt_id;
        self
    }

    /// Returns the organisation the user acts for, when the token names one.
    #[must_use]
    pub fn organisation(&self) -> Option<&str> {
        self.organisation.as_deref()
    }

    /// Returns the user's alternative identity, when one is known.
    #[must_use]
    pub fn alt_id(&self) -> Option<&str> {
        self.alt_id.as_deref()
    }

    /// Returns this user, the token they presented naming `audience` in its
    /// `aud`: the alias an ATNA `UserName` begins with (IUA ITI TF-2
    /// §3.72.5.1).
    #[must_use]
    pub fn with_audience(mut self, audience: Option<String>) -> Self {
        self.audience = audience;
        self
    }

    /// Returns this user, asking for `purposes` (the IHE IUA
    /// `purpose_of_use`).
    #[must_use]
    pub fn with_purposes(mut self, purposes: Vec<PurposeOfUse>) -> Self {
        self.purposes = purposes;
        self
    }

    /// Returns this user, the token they presented identified by
    /// `token_id`, its `jti`, which BALP 1.1.4 §3:5.7.5.4 writes as
    /// `agent[user].policy`.
    #[must_use]
    pub fn with_token_id(mut self, token_id: Option<String>) -> Self {
        self.stated.token_id = token_id;
        self
    }

    /// Returns the identity of the token the user presented (`jti`), when it
    /// names one.
    #[must_use]
    pub fn token_id(&self) -> Option<&str> {
        self.stated.token_id.as_deref()
    }

    /// Returns this user, acting under `policies` beside the token: each the
    /// identifier of a policy or plan that authorised the exchange, such as
    /// the data permit of a health data access body (Regulation (EU)
    /// 2025/327 Art 68), which the record writes at `agent[user].policy`
    /// after the token's `jti` (FHIR R4 `AuditEvent.agent.policy`, 0..*).
    ///
    /// The DICOM PS3.15 audit message carries none: its `ActiveParticipant`
    /// has no element for a policy (`atna::message::ActiveParticipant`).
    #[must_use]
    pub fn with_policies(mut self, policies: Vec<String>) -> Self {
        self.stated.policies = policies;
        self
    }

    /// Returns every policy the user acts under beside the token, in order.
    #[must_use]
    pub fn policies(&self) -> &[String] {
        &self.stated.policies
    }

    /// Returns this user, vouched for by the authorization server `issuer`
    /// (`iss`), or by none.
    #[must_use]
    pub fn with_issuer(mut self, issuer: Option<String>) -> Self {
        self.issuer = issuer;
        self
    }

    /// Returns this user, asking through the client application `client_id`
    /// (`client_id`), or through none.
    #[must_use]
    pub fn with_client_id(mut self, client_id: Option<String>) -> Self {
        self.client_id = client_id;
        self
    }

    /// Returns the authorization server that issued the token (`iss`), when
    /// a token named the user.
    #[must_use]
    pub fn issuer(&self) -> Option<&str> {
        self.issuer.as_deref()
    }

    /// Returns the user (`sub`).
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the client application the token was issued to
    /// (`client_id`), when a token named one.
    #[must_use]
    pub fn client_id(&self) -> Option<&str> {
        self.client_id.as_deref()
    }

    /// Returns the audience the token names, when the client said which.
    #[must_use]
    pub fn audience(&self) -> Option<&str> {
        self.audience.as_deref()
    }

    /// Returns every purpose of use the token declares.
    #[must_use]
    pub fn purposes(&self) -> &[PurposeOfUse] {
        &self.purposes
    }

    /// The ATNA `UserName` of a user a JWT authenticated:
    /// `alias"<"user"@"issuer">"`, with the token's `aud` as the alias, its
    /// `sub` as the user and its `iss` as the issuer (IUA ITI TF-2
    /// §3.72.5.1); the alias is empty when the audience is not known. IUA
    /// defines the form for a token only, so a user no issuer vouched for is
    /// named by their account alone.
    #[must_use]
    pub fn atna_user_name(&self) -> String {
        match &self.issuer {
            Some(issuer) => format!(
                "{}<{}@{}>",
                self.audience.as_deref().unwrap_or_default(),
                self.subject,
                issuer
            ),
            None => self.subject.clone(),
        }
    }
}

impl fmt::Debug for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("User").field(&REDACTED).finish()
    }
}

/// A code an audit record writes about its user, such as a role or an
/// assurance level: the code and the system that defines it, when one does.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Code {
    /// The code system, when one defines the code.
    pub system: Option<String>,
    /// The code.
    pub code: String,
}

/// One purpose of use: a code and the system that defines it, the HL7 v3
/// `PurposeOfUse` coding of the IHE IUA `purpose_of_use` claim.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PurposeOfUse {
    /// The code system, when the token names one.
    pub system: Option<String>,
    /// The code.
    pub code: String,
}
