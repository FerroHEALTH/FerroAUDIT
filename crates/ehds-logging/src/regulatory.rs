// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The regulatory metadata of an access: the legal basis of the processing
//! and the controllers and processors that carry it out.
//!
//! Xt-EHR D5.2 (2026-04-09) §7.2.7 asks logging metadata to "capture legal
//! basis, controller/processor roles, and retention policies". The legal
//! basis is the point of GDPR Art 6(1) that makes the processing lawful and,
//! for data concerning health, the point of Art 9(2) that lifts the
//! prohibition of Art 9(1); the controller determines the purposes and means
//! of the processing (Art 4(7), with joint controllers under Art 26) and a
//! processor processes on the controller's behalf (Art 4(8)).
//!
//! A deployment declares its [`Regulatory`] once in its configuration, and a
//! recording system that knows a different value for one access, such as
//! the legal basis it stored beside the access, overrides it for that record
//! with [`Regulatory::overridden_by`].

use serde::{Deserialize, Serialize};

/// The point of GDPR Art 6(1) that makes the processing lawful.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Lawfulness {
    /// Point (a): the data subject has given consent.
    Consent,
    /// Point (b): the performance of a contract with the data subject.
    Contract,
    /// Point (c): a legal obligation of the controller.
    LegalObligation,
    /// Point (d): the vital interests of the data subject or another person.
    VitalInterests,
    /// Point (e): a task in the public interest or in the exercise of
    /// official authority.
    PublicTask,
    /// Point (f): the legitimate interests of the controller or a third
    /// party.
    LegitimateInterests,
}

impl Lawfulness {
    /// The basis as a record writes it: `gdpr-art-6-1-<point>`.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Consent => "gdpr-art-6-1-a",
            Self::Contract => "gdpr-art-6-1-b",
            Self::LegalObligation => "gdpr-art-6-1-c",
            Self::VitalInterests => "gdpr-art-6-1-d",
            Self::PublicTask => "gdpr-art-6-1-e",
            Self::LegitimateInterests => "gdpr-art-6-1-f",
        }
    }
}

/// The point of GDPR Art 9(2) that lifts the prohibition of processing data
/// concerning health.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum Condition {
    /// Point (a): explicit consent.
    ExplicitConsent,
    /// Point (b): employment, social security and social protection law.
    Employment,
    /// Point (c): vital interests where the data subject cannot consent.
    VitalInterests,
    /// Point (d): a not-for-profit body's legitimate activities.
    NotForProfitBody,
    /// Point (e): data manifestly made public by the data subject.
    MadePublic,
    /// Point (f): legal claims or courts acting in their judicial capacity.
    LegalClaims,
    /// Point (g): substantial public interest.
    SubstantialPublicInterest,
    /// Point (h): preventive or occupational medicine, medical diagnosis,
    /// the provision of health or social care or treatment, or the
    /// management of health or social care systems.
    HealthOrSocialCare,
    /// Point (i): public interest in the area of public health.
    PublicHealth,
    /// Point (j): archiving in the public interest, scientific or
    /// historical research, or statistics.
    ArchivingOrResearch,
}

impl Condition {
    /// The condition as a record writes it: `gdpr-art-9-2-<point>`.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::ExplicitConsent => "gdpr-art-9-2-a",
            Self::Employment => "gdpr-art-9-2-b",
            Self::VitalInterests => "gdpr-art-9-2-c",
            Self::NotForProfitBody => "gdpr-art-9-2-d",
            Self::MadePublic => "gdpr-art-9-2-e",
            Self::LegalClaims => "gdpr-art-9-2-f",
            Self::SubstantialPublicInterest => "gdpr-art-9-2-g",
            Self::HealthOrSocialCare => "gdpr-art-9-2-h",
            Self::PublicHealth => "gdpr-art-9-2-i",
            Self::ArchivingOrResearch => "gdpr-art-9-2-j",
        }
    }
}

/// The legal basis of a processing: its lawfulness, and for data concerning
/// health the condition that lifts the prohibition of Art 9(1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct LegalBasis {
    /// The point of Art 6(1).
    pub lawfulness: Lawfulness,
    /// The point of Art 9(2), when one is named.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<Condition>,
}

/// The regulatory metadata of an access (Xt-EHR D5.2 §7.2.7): its legal
/// basis and the controllers and processors, each by an identifier the
/// deployment chooses, such as an organisation's registry number.
///
/// The default names none of them, and the record then writes none.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Regulatory {
    /// The legal basis of the processing, when it is stated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub legal_basis: Option<LegalBasis>,
    /// The controller, or each joint controller (GDPR Art 26), in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controllers: Vec<String>,
    /// Each processor that processes on a controller's behalf, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub processors: Vec<String>,
}

impl Regulatory {
    /// These metadata with each member `access` states taking the place of
    /// this one: the legal basis when `access` states one, and the
    /// controllers and the processors each when `access` names any.
    #[must_use]
    pub fn overridden_by(&self, access: &Self) -> Self {
        let pick = |own: &Vec<String>, theirs: &Vec<String>| {
            if theirs.is_empty() {
                own.clone()
            } else {
                theirs.clone()
            }
        };
        Self {
            legal_basis: access.legal_basis.or(self.legal_basis),
            controllers: pick(&self.controllers, &access.controllers),
            processors: pick(&self.processors, &access.processors),
        }
    }
}
