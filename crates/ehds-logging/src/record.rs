// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The access record: what the logging component records "on every access
//! event or group of events" (Regulation (EU) 2025/327 Annex II 3.2).
//!
//! Annex II 3.2 asks for (a) the healthcare provider or other individuals
//! that accessed the data, (b) the specific natural person or persons, (c)
//! the categories of data accessed, (d) the time and date, and (e) the
//! origin or origins of the data. An [`AccessRecord`] carries each, and
//! beside them the data subject whose data were accessed, the purpose of
//! use the access was made for, which Art 9(2) asks the patient be told, and
//! the policies it was authorised under, such as a data permit (Art 68). A
//! record whose accessor declared an emergency purpose carries the mark
//! Art 11(5) asks for ([`emergency`](crate::emergency)).
//!
//! The accessor names how the recording system bound it to a party
//! ([`Binding`]), its token's actor chain ([`Actor`]) and the representation
//! a representative acts under ([`Representation`]); an accessor no party
//! here names is what a contact point or a federation relays ([`Relayed`]),
//! held at construction to the rules [`RelayedError`] lists. Which fields
//! carry the binding is our own design: Annex II 3.2 names what a record
//! identifies, not how.
//!
//! A record names the patient and the person who accessed their data, so
//! `Debug` shows neither, nor any id the classification read: a record
//! leaves through an [`AccessSink`](crate::sink::AccessSink) alone.

use std::collections::{BTreeMap, BTreeSet};
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
    /// data only for the professional the token names; kept so a stored
    /// record that names it stays readable.
    Client,
    /// A client application acting for the person the token names, by a
    /// delegation such as an RFC 8693 token exchange.
    Delegated,
    /// A verified person or organisation of another party, whom a contact
    /// point or a federation relays ([`Relayed`]).
    External,
    /// A client application acting for nobody, as the recording system's
    /// own machine user.
    System,
}

impl Acting {
    /// The code a record writes the acting mode as: `person`, `client`,
    /// `delegated`, `external` or `system`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Person => "person",
            Self::Client => "client",
            Self::Delegated => "delegated",
            Self::External => "external",
            Self::System => "system",
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

/// How a relayed accessor reached the recording system: the issuer of the
/// token that relays them, and under which law.
///
/// The two cases fall under different law, so one field never carries both.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Via {
    /// A MyHealth@EU national contact point for digital health, whose
    /// relaying Implementing Regulation (EU) 2026/2099 Art 7 binds.
    ContactPoint {
        /// The issuer of the contact point's token.
        issuer: String,
    },
    /// A domestic federation issuer, such as a national gateway, which
    /// 2026/2099 does not bind: the access is made under the agreement of
    /// the federation ([`Agreement`]).
    Federation {
        /// The issuer of the federation's token.
        issuer: String,
    },
}

impl Via {
    /// The code a record writes the case as: `contact-point` or
    /// `federation`.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::ContactPoint { .. } => "contact-point",
            Self::Federation { .. } => "federation",
        }
    }

    /// The issuer of the relaying token.
    #[must_use]
    pub fn issuer(&self) -> &str {
        match self {
            Self::ContactPoint { issuer } | Self::Federation { issuer } => issuer,
        }
    }
}

/// The SAML `NameID` of the professional's identity assertion, with the
/// assertion's `Issuer` (eHDSI SAML Profile 10.0.0 §2.1).
///
/// `Debug` shows that a service provider id is present and no value.
#[derive(Clone, PartialEq, Eq)]
pub struct NameId {
    /// `Subject/NameID`: the professional's identifier, which the issuer
    /// "MUST guarantee ... can be long-term tracked back to an individual
    /// person".
    pub value: String,
    /// `NameID@SPProvidedID`, when the assertion names one.
    pub sp_provided_id: Option<String>,
    /// `Issuer`: the endpoint of the service that issued the assertion.
    pub issuer: String,
}

impl NameId {
    /// The user identifier the eHDSI Audit Trail Profile 10.1.0 §2.3.41
    /// fixes: `SPProvidedID<NameID@Issuer>`, the `SPProvidedID` empty when
    /// the assertion names none.
    #[must_use]
    pub fn user_id(&self) -> String {
        format!(
            "{}<{}@{}>",
            self.sp_provided_id.as_deref().unwrap_or_default(),
            self.value,
            self.issuer
        )
    }
}

impl fmt::Debug for NameId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NameId")
            .field("sp_provided_id", &self.sp_provided_id.is_some())
            .finish_non_exhaustive()
    }
}

/// How the professional authenticated, as its `AuthnContextClassRef` class.
///
/// One of the seven two-factor classes the eHDSI SAML Profile 10.0.0 §2.1
/// admits, recorded in place of an assurance level, which the assertion
/// does not carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationMethod {
    /// `urn:oasis:names:tc:SAML:2.0:ac:classes:MobileTwoFactorUnregistered`.
    MobileTwoFactorUnregistered,
    /// `urn:oasis:names:tc:SAML:2.0:ac:classes:MobileTwoFactorContract`.
    MobileTwoFactorContract,
    /// `urn:oasis:names:tc:SAML:2.0:ac:classes:X509`.
    X509,
    /// `urn:oasis:names:tc:SAML:2.0:ac:classes:SPKI`.
    Spki,
    /// `urn:oasis:names:tc:SAML:2.0:ac:classes:SmartcardPKI`.
    SmartcardPki,
    /// `urn:oasis:names:tc:SAML:2.0:ac:classes:SoftwarePKI`.
    SoftwarePki,
    /// `urn:oasis:names:tc:SAML:2.0:ac:classes:TLSClient`.
    TlsClient,
}

impl AuthenticationMethod {
    /// Every method, in the order the profile lists them.
    const ALL: [Self; 7] = [
        Self::MobileTwoFactorUnregistered,
        Self::MobileTwoFactorContract,
        Self::X509,
        Self::Spki,
        Self::SmartcardPki,
        Self::SoftwarePki,
        Self::TlsClient,
    ];

    /// The `AuthnContextClassRef` URI.
    #[must_use]
    pub const fn uri(self) -> &'static str {
        match self {
            Self::MobileTwoFactorUnregistered => {
                "urn:oasis:names:tc:SAML:2.0:ac:classes:MobileTwoFactorUnregistered"
            }
            Self::MobileTwoFactorContract => {
                "urn:oasis:names:tc:SAML:2.0:ac:classes:MobileTwoFactorContract"
            }
            Self::X509 => "urn:oasis:names:tc:SAML:2.0:ac:classes:X509",
            Self::Spki => "urn:oasis:names:tc:SAML:2.0:ac:classes:SPKI",
            Self::SmartcardPki => "urn:oasis:names:tc:SAML:2.0:ac:classes:SmartcardPKI",
            Self::SoftwarePki => "urn:oasis:names:tc:SAML:2.0:ac:classes:SoftwarePKI",
            Self::TlsClient => "urn:oasis:names:tc:SAML:2.0:ac:classes:TLSClient",
        }
    }

    /// The method whose `AuthnContextClassRef` is `uri`, or `None` for a
    /// class the profile does not admit.
    #[must_use]
    pub fn from_uri(uri: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|method| method.uri() == uri)
    }
}

/// What a contact point or a federation relays: the health professional and
/// the healthcare provider of another party, as the relaying issuer asserts
/// them.
///
/// The recording system verified the relaying issuer, never the
/// professional: Implementing Regulation (EU) 2026/2099 Art 6(1) and (2)
/// give their identification and authentication to the entity the Member
/// State of the professional lists, and a federation agreement gives them to
/// its member. [`Relayed`] is built from these parts, which it holds to the
/// rules [`RelayedError`] names.
#[derive(Clone, PartialEq, Eq)]
pub struct RelayedParts {
    /// How the accessor reached the recording system.
    pub via: Via,
    /// `country_code`: the ISO 3166-1 alpha-2 code of the Member State that
    /// issued the professional's identification data (2026/2099 Annex
    /// Table 1), which a contact point must state and a federation states
    /// when its agreement asks for it.
    pub country_code: Option<String>,
    /// The health professional (Annex Table 1), or `None` for an
    /// organisation a federation agreement admits without a named person.
    pub professional: Option<RelayedProfessional>,
    /// The healthcare provider (Annex Table 2), which a contact point must
    /// state.
    pub provider: Option<RelayedProvider>,
    /// The agreement the access is made under, which every federation
    /// access names (GDPR Art 5(2), Art 30(1)(b)).
    pub agreement: Option<Agreement>,
    /// The point of care, by name: the XSPA `locality` of the identity
    /// assertion (eHDSI SAML Profile 10.0.0 §2.3).
    pub point_of_care: Option<String>,
    /// The `@ID` of the professional's identity assertion, the `IdA`.
    pub identity_assertion_id: Option<String>,
    /// The `@ID` of the treatment relationship confirmation assertion (TRC).
    pub trc_assertion_id: Option<String>,
    /// The request message's UUID, the WS-Addressing `MessageID`, by which
    /// the record joins the contact point's Patient Privacy audit entry
    /// (eHDSI Audit Trail Profile 10.1.0 §2.3.12 to §2.3.32).
    pub message_id: Option<String>,
    /// The contact points the request passed, each by the CN of its TLS
    /// certificate (eHDSI Audit Trail Profile 10.1.0 §2.3.14, §2.3.15).
    pub gateways: Gateways,
    /// Each further field the relaying issuer asserted that no member above
    /// holds, such as a national professional number, by the name the
    /// recording system gives it, with every value it carries.
    pub asserted: BTreeMap<String, Vec<String>>,
}

/// The contact points a relayed request passed, each by the CN of its TLS
/// certificate (eHDSI Audit Trail Profile 10.1.0 §2.3.14, §2.3.15).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Gateways {
    /// The service consumer NCP, in the professional's country, which
    /// triggered the operation.
    pub consumer: Option<String>,
    /// The service provider NCP, in the patient's country, which processed
    /// it.
    pub provider: Option<String>,
}

/// The federation agreement a relayed access is made under (GDPR Art 5(2)
/// and Art 30(1)(b)).
///
/// `Debug` shows the access class and the counts, never a value.
#[derive(Clone, PartialEq, Eq)]
pub struct Agreement {
    /// The agreement's name.
    pub name: String,
    /// The agreement's version.
    pub version: String,
    /// Each legal ground the agreement rests on, coded: the GDPR Art 6(1)
    /// and Art 9(2) points and the national provision.
    pub legal_ground: Vec<Coded>,
    /// The reference of the consent decision taken for the patient, when
    /// the ground is consent.
    pub consent: Option<String>,
    /// How an organisation without a named person accesses the data, which
    /// an agreement that admits one declares.
    pub access_class: Option<AccessClass>,
}

impl fmt::Debug for Agreement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Agreement")
            .field("legal_ground", &self.legal_ground.len())
            .field("consent", &self.consent.is_some())
            .field("access_class", &self.access_class)
            .finish_non_exhaustive()
    }
}

/// How an organisation without a named person accesses the data under its
/// agreement (no specification governs the classes: our own design).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum AccessClass {
    /// A system of the organisation, with no person viewing the data.
    Automated,
    /// A person views the data at the requester, who keeps the log that
    /// names them, joined to this record by an attested correlation id.
    Chained,
}

impl AccessClass {
    /// The code a record writes the class as: `automated` or `chained`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Automated => "automated",
            Self::Chained => "chained",
        }
    }
}

/// Why [`RelayedParts`] are no relayed accessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RelayedError {
    /// A contact point relayed no professional: the MyHealth@EU
    /// Requirements Catalogue 10.0.0 requirement 01 asks for the HP's
    /// information, whose absence is `ERROR_HPI_NO_INFORMATION`.
    #[error("a contact point relayed no health professional")]
    ContactPointWithoutProfessional,
    /// A federation access names no agreement.
    #[error("a federation access names no agreement")]
    FederationWithoutAgreement,
    /// An organisation without a named person is relayed under an agreement
    /// that declares no access class for it.
    #[error("an organisation without a named person has no access class")]
    OrganisationWithoutAccessClass,
    /// A contact point's provider is named by no `urn:oid:` or `urn:hl7ii:`
    /// `organization-id` (eHDSI SAML Profile 10.0.0 §2.3).
    #[error("the provider's organization-id is no urn:oid: or urn:hl7ii: identifier")]
    ProviderIdentifierForm,
    /// A contact point relays a clerical support worker, ISCO `44`, and not
    /// whom they act for, which eHDSI SAML Profile 10.0.0 §2.3 makes
    /// mandatory.
    #[error("a clerical support worker is relayed without on-behalf-of")]
    OnBehalfOfMissing,
    /// A contact point relays no value for a field 2026/2099 Annex Table 1
    /// or Table 2 makes mandatory.
    #[error("a contact point relayed no {field}")]
    ContactPointFieldMissing {
        /// The field, such as `country_code`.
        field: &'static str,
    },
}

/// The ISCO code system the eHDSI roles are a subset of (eHDSI SAML Profile
/// 10.0.0 §2.3).
pub const ISCO: &str = "2.16.840.1.113883.2.9.6.2.7";

/// What a contact point or a federation relays, held to its rules.
///
/// `Debug` shows how it was relayed and the country, and none of the other
/// values.
#[derive(Clone, PartialEq, Eq)]
pub struct Relayed(RelayedParts);

impl Relayed {
    /// The parts of this relayed accessor.
    #[must_use]
    pub fn as_parts(&self) -> &RelayedParts {
        &self.0
    }

    /// The parts of this relayed accessor, taken back.
    #[must_use]
    pub fn into_parts(self) -> RelayedParts {
        self.0
    }

    /// Whether the professional is named, or an organisation accessed the
    /// data without a named person.
    #[must_use]
    pub fn names_a_professional(&self) -> bool {
        self.0.professional.is_some()
    }
}

impl TryFrom<RelayedParts> for Relayed {
    type Error = RelayedError;

    fn try_from(parts: RelayedParts) -> Result<Self, Self::Error> {
        match (&parts.via, &parts.professional) {
            (Via::ContactPoint { .. }, None) => {
                return Err(RelayedError::ContactPointWithoutProfessional);
            }
            (Via::ContactPoint { .. }, Some(professional)) => {
                let provider = contact_point_fields(&parts, professional)?;
                let identifier = &provider.identifier;
                if !identifier.starts_with("urn:oid:") && !identifier.starts_with("urn:hl7ii:") {
                    return Err(RelayedError::ProviderIdentifierForm);
                }
                let clerical = professional
                    .roles
                    .iter()
                    .any(|role| role.code == "44" && role.system.as_deref() == Some(ISCO));
                if clerical && professional.on_behalf_of.is_none() {
                    return Err(RelayedError::OnBehalfOfMissing);
                }
            }
            (Via::Federation { .. }, professional) => {
                let Some(agreement) = &parts.agreement else {
                    return Err(RelayedError::FederationWithoutAgreement);
                };
                if professional.is_none() && agreement.access_class.is_none() {
                    return Err(RelayedError::OrganisationWithoutAccessClass);
                }
            }
        }
        Ok(Self(parts))
    }
}

/// The provider of a contact point's `parts`, once every field 2026/2099
/// Annex Tables 1 and 2 make mandatory is there.
fn contact_point_fields<'a>(
    parts: &'a RelayedParts,
    professional: &RelayedProfessional,
) -> Result<&'a RelayedProvider, RelayedError> {
    let missing = |field| RelayedError::ContactPointFieldMissing { field };
    if parts.country_code.is_none() {
        return Err(missing("country_code"));
    }
    for (field, value) in [
        ("hp_name", &professional.name),
        ("hp_identifier", &professional.identifier),
        ("hp_issuing_authority_name", &professional.issuing_authority),
    ] {
        if value.is_none() {
            return Err(missing(field));
        }
    }
    let provider = parts
        .provider
        .as_ref()
        .ok_or_else(|| missing("healthcare_provider_identifier"))?;
    if provider.issuing_authority.is_none() {
        return Err(missing("provider_issuing_authority_name"));
    }
    Ok(provider)
}

impl fmt::Debug for Relayed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Relayed")
            .field("via", &self.0.via.code())
            .field("country_code", &self.0.country_code)
            .field("professional", &self.0.professional)
            .field("provider", &self.0.provider)
            .field("agreement", &self.0.agreement)
            .finish_non_exhaustive()
    }
}

impl fmt::Debug for RelayedParts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RelayedParts")
            .field("via", &self.via.code())
            .field("country_code", &self.country_code)
            .field("asserted", &self.asserted.len())
            .finish_non_exhaustive()
    }
}

/// The health professional a contact point or a federation relays
/// (2026/2099 Annex Table 1, with the eHDSI SAML Profile 10.0.0 §2.3
/// attributes).
#[derive(Clone, PartialEq, Eq)]
pub struct RelayedProfessional {
    /// The full name, the XSPA `subject-id`, which a contact point must
    /// state.
    pub name: Option<String>,
    /// `family_name`, for an issuer that sends it apart.
    pub family_name: Option<String>,
    /// `given_name`, for an issuer that sends it apart.
    pub given_name: Option<String>,
    /// `hp_identifier`, unique among the identifiers the Member State where
    /// the professional is registered issues, which a contact point must
    /// state.
    pub identifier: Option<String>,
    /// `issuing_authority_name`: the agency that issued `hp_identifier`,
    /// which a contact point must state.
    pub issuing_authority: Option<String>,
    /// `hp_professional_role`, as the codes the relaying issuer states,
    /// such as the ISCO role of the XACML `role` attribute.
    pub roles: Vec<Coded>,
    /// `healthcare_provider_identifier`: "An identifier of the healthcare
    /// provider organisation or healthcare facility where the health
    /// professional is providing treatment" (Annex Table 1).
    pub healthcare_provider_identifier: Option<String>,
    /// The identity assertion's `NameID`, which the DICOM rendering names
    /// the requester by (eHDSI Audit Trail Profile 10.1.0 §2.3.41).
    pub name_id: Option<NameId>,
    /// The `AuthnContextClassRef` of the professional's authentication.
    pub authentication_method: Option<AuthenticationMethod>,
    /// `on-behalf-of`: the role of the person the professional acts for,
    /// mandatory for an ISCO `44` clerical support worker.
    pub on_behalf_of: Option<Coded>,
    /// The XSPA `hl7:permission` URIs the issuer grants.
    pub permissions: Vec<String>,
}

impl fmt::Debug for RelayedProfessional {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RelayedProfessional")
            .field("roles", &self.roles.len())
            .field("name_id", &self.name_id)
            .field("authentication_method", &self.authentication_method)
            .field("on_behalf_of", &self.on_behalf_of.is_some())
            .field("permissions", &self.permissions.len())
            .finish_non_exhaustive()
    }
}

/// The healthcare provider a contact point or a federation relays
/// (2026/2099 Annex Table 2, with the eHDSI SAML Profile 10.0.0 §2.3
/// attributes), where the professional provides the treatment.
#[derive(Clone, PartialEq, Eq)]
pub struct RelayedProvider {
    /// `healthcare_provider_identifier`, unique in the Member State that
    /// issued it; from a contact point the XSPA `organization-id`, a
    /// `urn:oid:` or `urn:hl7ii:` identifier.
    pub identifier: String,
    /// `issuing_authority_name`: the agency that issued the identifier,
    /// which a contact point must state.
    pub issuing_authority: Option<String>,
    /// `healthcare_provider_name`, or the XSPA `organization`, when named.
    pub name: Option<String>,
    /// `healthcare_provider_address`: the official registered full address,
    /// when the issuer states it.
    pub address: Option<String>,
    /// The eHDSI `healthcare-facility-type`, such as `Hospital`.
    pub facility_type: Option<Coded>,
    /// The XCA `homeCommunityId` of the provider.
    pub home_community_id: Option<String>,
    /// The organisation type a national profile asks for, such as the
    /// Dutch one.
    pub organisation_type: Option<Coded>,
}

impl fmt::Debug for RelayedProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RelayedProvider")
            .field("facility_type", &self.facility_type)
            .field("organisation_type", &self.organisation_type)
            .finish_non_exhaustive()
    }
}

/// A reference to a party in a demographic store, the openEHR `PARTY_REF`
/// triple.
///
/// `Debug` shows the type and none of the other values.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PartyRef {
    /// `namespace`.
    pub namespace: String,
    /// `type`, such as `PERSON` or `GROUP`.
    pub kind: String,
    /// `id`.
    pub id: String,
}

impl fmt::Debug for PartyRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PartyRef")
            .field("kind", &self.kind)
            .finish_non_exhaustive()
    }
}

/// The party a person was bound to, with the groups they belong to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundParty {
    /// The party.
    pub party: PartyRef,
    /// Each group the person belongs to, as the lookup matched it.
    pub groups: Vec<PartyRef>,
}

/// How the recording system bound the authenticated caller to a party, one
/// outcome per record (no specification governs the outcomes: our own
/// design).
///
/// The party is named on `bound`, `provisional`, `refused-inactive` and
/// `refused-abandoned`, and on no other outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Binding {
    /// `bound`: a person bound to a complete party.
    Bound {
        /// The party and its groups.
        party: BoundParty,
        /// Whether the person is a guest of the issuer.
        guest: bool,
    },
    /// `provisional`: a person bound to a party not yet complete.
    Provisional {
        /// The party and its groups.
        party: BoundParty,
    },
    /// `external`: a verified person of another party ([`Relayed`]).
    External,
    /// `organisation`: an organisation under an agreement, with no named
    /// person.
    Organisation,
    /// `system`: a client acting for nobody.
    System,
    /// `self`: the patient, by a patient token.
    PartySelf,
    /// `refused-unbound`: no party carries the login.
    RefusedUnbound,
    /// `refused-ambiguous`: two or more parties carry the login.
    RefusedAmbiguous,
    /// `refused-inactive`: the party is inactive.
    RefusedInactive {
        /// The party.
        party: PartyRef,
    },
    /// `refused-abandoned`: the party is abandoned.
    RefusedAbandoned {
        /// The party.
        party: PartyRef,
    },
    /// `refused-key-claim-missing`: the token lacks the claim the binding
    /// keys on.
    RefusedKeyClaimMissing {
        /// The claim's name.
        claim: String,
    },
    /// `refused-groups-unavailable`: the group claim is unreadable or over
    /// its limit.
    RefusedGroupsUnavailable,
    /// `refused-guest`: a guest of the issuer.
    RefusedGuest,
    /// `refused-external-incomplete`: a federation token lacks or garbles a
    /// field it must carry.
    RefusedExternalIncomplete {
        /// The names of the fields.
        fields: Vec<String>,
    },
    /// `refused-trc-patient-mismatch`: the request's patient is not the
    /// treatment relationship confirmation's, recorded as a security event.
    RefusedTrcPatientMismatch,
    /// `refused-deprovisioned`: the login's key belongs to a deleted party.
    RefusedDeprovisioned,
    /// `refused-wrong-marker`: the party's binding marker does not fit the
    /// caller's kind.
    RefusedWrongMarker,
    /// `refused-representation-unrecorded`: no live representation
    /// relationship to the requested subject is recorded.
    RefusedRepresentationUnrecorded,
    /// `refused-trc-missing`: a contact point's request for patient data
    /// carries no treatment relationship confirmation, recorded as a
    /// security event.
    RefusedTrcMissing,
    /// `refused-system-personal-scope`: a system client holds a `user/` or
    /// `patient/` scope, which names a person it does not act for.
    RefusedSystemPersonalScope,
    /// `refused-no-agreement`: no agreement of the federation issuer admits
    /// the accessor's kind.
    RefusedNoAgreement,
    /// `refused-scope-beyond-agreement`: an external accessor holds a scope
    /// its agreement does not admit.
    RefusedScopeBeyondAgreement,
    /// `refused-correlation-missing`: a `chained` organisation's request
    /// carries no attested correlation id.
    RefusedCorrelationMissing,
    /// `refused-wabvpz-role`: a caller under a Dutch agreement asserts the
    /// UZI role of an occupational or insurance physician (Wabvpz Art.
    /// 15f(2)).
    RefusedWabvpzRole,
    /// `refused-wabvpz-organisation-type`: a caller under a Dutch agreement
    /// asserts no care provider type (Wabvpz Art. 15f(1)).
    RefusedWabvpzOrganisationType,
}

impl Binding {
    /// The code a record writes the outcome as.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Bound { .. } => "bound",
            Self::Provisional { .. } => "provisional",
            Self::External => "external",
            Self::Organisation => "organisation",
            Self::System => "system",
            Self::PartySelf => "self",
            Self::RefusedUnbound => "refused-unbound",
            Self::RefusedAmbiguous => "refused-ambiguous",
            Self::RefusedInactive { .. } => "refused-inactive",
            Self::RefusedAbandoned { .. } => "refused-abandoned",
            Self::RefusedKeyClaimMissing { .. } => "refused-key-claim-missing",
            Self::RefusedGroupsUnavailable => "refused-groups-unavailable",
            Self::RefusedGuest => "refused-guest",
            Self::RefusedExternalIncomplete { .. } => "refused-external-incomplete",
            Self::RefusedTrcPatientMismatch => "refused-trc-patient-mismatch",
            Self::RefusedDeprovisioned => "refused-deprovisioned",
            Self::RefusedWrongMarker => "refused-wrong-marker",
            Self::RefusedRepresentationUnrecorded => "refused-representation-unrecorded",
            Self::RefusedTrcMissing => "refused-trc-missing",
            Self::RefusedSystemPersonalScope => "refused-system-personal-scope",
            Self::RefusedNoAgreement => "refused-no-agreement",
            Self::RefusedScopeBeyondAgreement => "refused-scope-beyond-agreement",
            Self::RefusedCorrelationMissing => "refused-correlation-missing",
            Self::RefusedWabvpzRole => "refused-wabvpz-role",
            Self::RefusedWabvpzOrganisationType => "refused-wabvpz-organisation-type",
        }
    }

    /// The party the outcome names, when it names one.
    #[must_use]
    pub const fn party(&self) -> Option<&PartyRef> {
        match self {
            Self::Bound { party, .. } | Self::Provisional { party } => Some(&party.party),
            Self::RefusedInactive { party } | Self::RefusedAbandoned { party } => Some(party),
            _ => None,
        }
    }

    /// The groups the bound person belongs to.
    #[must_use]
    pub fn groups(&self) -> &[PartyRef] {
        match self {
            Self::Bound { party, .. } | Self::Provisional { party } => &party.groups,
            _ => &[],
        }
    }

    /// Whether the outcome is a `bound` guest of the issuer.
    #[must_use]
    pub const fn is_guest(&self) -> bool {
        matches!(self, Self::Bound { guest: true, .. })
    }

    /// Whether the outcome refused the request.
    #[must_use]
    pub const fn is_refusal(&self) -> bool {
        matches!(
            self,
            Self::RefusedUnbound
                | Self::RefusedAmbiguous
                | Self::RefusedInactive { .. }
                | Self::RefusedAbandoned { .. }
                | Self::RefusedKeyClaimMissing { .. }
                | Self::RefusedGroupsUnavailable
                | Self::RefusedGuest
                | Self::RefusedExternalIncomplete { .. }
                | Self::RefusedTrcPatientMismatch
                | Self::RefusedDeprovisioned
                | Self::RefusedWrongMarker
                | Self::RefusedRepresentationUnrecorded
                | Self::RefusedTrcMissing
                | Self::RefusedSystemPersonalScope
                | Self::RefusedNoAgreement
                | Self::RefusedScopeBeyondAgreement
                | Self::RefusedCorrelationMissing
                | Self::RefusedWabvpzRole
                | Self::RefusedWabvpzOrganisationType
        )
    }

    /// Whether the outcome is recorded as a security event.
    #[must_use]
    pub const fn is_security_event(&self) -> bool {
        matches!(
            self,
            Self::RefusedTrcPatientMismatch | Self::RefusedTrcMissing
        )
    }
}

/// One actor of a token's actor chain (RFC 8693 §4.1 `act`), recorded and
/// never decided on.
///
/// `Debug` shows that an issuer is named and no value.
#[derive(Clone, PartialEq, Eq)]
pub struct Actor {
    /// The actor's issuer, when the chain names one.
    pub issuer: Option<String>,
    /// The actor's subject.
    pub subject: String,
}

impl fmt::Debug for Actor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Actor")
            .field("issuer", &self.issuer.is_some())
            .finish_non_exhaustive()
    }
}

/// Whom a representative acts for, on which ground, and by which recorded
/// relationship.
///
/// `Debug` shows which members are present and none of their values.
#[derive(Clone, PartialEq, Eq)]
pub struct Representation {
    /// The person represented, by the identifier the request asserted.
    pub represented: String,
    /// The ground of the representation, when the request states one.
    pub ground: Option<Coded>,
    /// The `relationship` code of the representative's openEHR
    /// `PARTY_RELATED` committer, when the representation relationship
    /// records one.
    pub relationship: Option<Coded>,
}

impl fmt::Debug for Representation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Representation")
            .field("ground", &self.ground.is_some())
            .field("relationship", &self.relationship.is_some())
            .finish_non_exhaustive()
    }
}

/// An identifier under the issuer that issued it, such as a healthcare
/// provider's or a facility's.
///
/// `Debug` shows neither value.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct IssuedId {
    /// The identifier's issuer, such as an OID.
    pub issuer: String,
    /// The identifier.
    pub id: String,
}

impl fmt::Debug for IssuedId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IssuedId").finish_non_exhaustive()
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
    /// The identifier of every policy or plan the access was authorised
    /// under beside the token, such as the data permit of a health data
    /// access body (Art 68) a secondary-use caller declares it acts under,
    /// in order. The BALP record writes each at `agent[user].policy` (FHIR R4
    /// `AuditEvent.agent.policy`).
    pub policies: Vec<String>,
    /// The professional and the provider of another party a contact point
    /// or a federation relays, marked as asserted by it, when the client is
    /// one (Implementing Regulation (EU) 2026/2099 Art 7, or a federation
    /// agreement).
    pub relayed: Option<Relayed>,
    /// How the recording system bound the caller to a party, with the party
    /// and its groups where the outcome names them, when it bound at all.
    pub binding: Option<Binding>,
    /// The token's actor chain, outermost first: the relaying gateway, the
    /// contact point and the client, as RFC 8693 §4.1 nests them.
    pub actors: Vec<Actor>,
    /// Whom the person acts for as a representative, and on which ground.
    pub represents: Option<Representation>,
    /// The healthcare provider a bound person or a system client acts for,
    /// by its identifier under its issuer (Annex II 3.2(a)), when the
    /// recording system attributes the access to one.
    pub acts_for: Option<IssuedId>,
    /// The facilities the compositions a system client committed name, each
    /// by its identifier under its issuer.
    pub facilities: Vec<IssuedId>,
}

impl Accessor {
    /// The party the binding names, when it names one.
    #[must_use]
    pub fn party(&self) -> Option<&PartyRef> {
        self.binding.as_ref().and_then(Binding::party)
    }
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
            .field("policies", &self.policies.len())
            .field("relayed", &self.relayed)
            .field("binding", &self.binding)
            .field("actors", &self.actors)
            .field("represents", &self.represents)
            .field("acts_for", &self.acts_for.is_some())
            .field("facilities", &self.facilities.len())
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
    /// The resource the request addressed, when it addressed one: a FHIR
    /// literal reference (`[type]/[id]`) to a resource the recording system
    /// serves, or an id it issues, such as an openEHR version uid, which the
    /// BALP record names by an identifier (`balp::ObjectIds`, feature
    /// `balp`).
    pub resource: Option<String>,
    /// The query text, for a query.
    pub query: Option<SecretString>,
    /// The name of the stored query, for a stored-query execution.
    pub stored_query: Option<String>,
    /// The address the request came from, when known.
    pub client_address: Option<IpAddr>,
    /// The identifier the client correlates the request by in its own
    /// log, such as a national contact point's exchange log, with where the
    /// recording system read it, when the client sent one.
    pub correlation: Option<Correlation>,
}

/// The identifier a client correlates a request by in its own log, and where
/// the recording system read it.
///
/// `Debug` shows the source and not the value.
#[derive(Clone, PartialEq, Eq)]
pub struct Correlation {
    /// The identifier.
    pub value: String,
    /// Where it was read.
    pub source: CorrelationSource,
}

impl fmt::Debug for Correlation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Correlation")
            .field("source", &self.source)
            .finish_non_exhaustive()
    }
}

/// Where a correlation identifier was read (no specification governs the
/// sources: our own design).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CorrelationSource {
    /// A claim of the token the recording system's authorization server
    /// issued, which it attests.
    TokenClaim,
    /// A member of the token's `authorization_details`, which the
    /// authorization server attests.
    AuthorizationDetails,
    /// A request header, which nobody attests.
    Header,
}

impl CorrelationSource {
    /// The code a record writes the source as: `token-claim`,
    /// `authorization-details` or `header`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::TokenClaim => "token-claim",
            Self::AuthorizationDetails => "authorization-details",
            Self::Header => "header",
        }
    }

    /// Whether the recording system's authorization server attests the
    /// source.
    #[must_use]
    pub const fn is_attested(self) -> bool {
        matches!(self, Self::TokenClaim | Self::AuthorizationDetails)
    }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Request")
            .field("id", &self.id)
            .field("operation", &self.operation)
            .field("query", &self.query.is_some())
            .field("correlation", &self.correlation)
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

/// Why a record is not one a sink stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum InvalidRecord {
    /// An organisation accessed the data under a `chained` access class
    /// with no correlation id the recording system's authorization server
    /// attested, so the record cannot join the requester's log that names
    /// the person who viewed the data.
    #[error("a chained organisation access has no attested correlation id")]
    ChainedWithoutAttestedCorrelation,
}

impl AccessRecord {
    /// Holds the record to the rules a sink stores it under.
    ///
    /// # Errors
    ///
    /// An [`InvalidRecord`] naming the rule the record breaks.
    pub fn validate(&self) -> Result<(), InvalidRecord> {
        let chained = self.accessor.relayed.as_ref().is_some_and(|relayed| {
            let parts = relayed.as_parts();
            parts.professional.is_none()
                && parts
                    .agreement
                    .as_ref()
                    .is_some_and(|agreement| agreement.access_class == Some(AccessClass::Chained))
        });
        let attested = self
            .request
            .correlation
            .as_ref()
            .is_some_and(|correlation| correlation.source.is_attested());
        if chained && !attested {
            return Err(InvalidRecord::ChainedWithoutAttestedCorrelation);
        }
        Ok(())
    }
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
