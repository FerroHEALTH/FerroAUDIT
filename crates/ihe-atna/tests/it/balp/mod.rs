// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! BALP: the `AuditEvent` an exchange is written as; the ATX: FHIR Feed that
//! sends it (the `RESTful` ATNA supplement, ITI TF-2 §3.20.4.2); the hygiene
//! of the patient identifier a record carries, what a described entity and a
//! user add to it, and where it carries the purposes of use.

mod described;
mod feed;
mod hygiene;
mod purposes;

use ihe_atna::balp::{NetworkAddress, Observer};
use ihe_atna::user::{OnBehalfOf, PurposeOfUse, User};

/// The system the tests record as.
pub(crate) fn observer() -> Observer {
    Observer {
        source_id: "gateway.example.org".to_owned(),
        site: Some("example site".to_owned()),
        host: NetworkAddress::host("gateway.example.org"),
        source_types: Vec::new(),
    }
}

/// The issuer of the synthetic user's token.
pub(crate) const ISSUER: &str = "https://issuer.example.test";

/// The synthetic user's `sub`, unlike any other text.
pub(crate) const SUBJECT: &str = "Qz7-clinician-31";

/// The synthetic user's `client_id`, unlike any other text.
pub(crate) const CLIENT: &str = "Qz7-client-32";

/// The audience the synthetic user's token names.
pub(crate) const AUDIENCE: &str = "urn:example:gateway-under-test";

/// The HL7 v3 `ActReason` code system of the synthetic purpose of use.
const ACT_REASON: &str = "http://terminology.hl7.org/CodeSystem/v3-ActReason";

/// A synthetic user who asked for an exchange through their OAuth token, for
/// the purpose of use `TREAT`.
pub(crate) fn user() -> OnBehalfOf {
    OnBehalfOf::User(
        User::new(ISSUER.to_owned(), SUBJECT.to_owned(), CLIENT.to_owned())
            .with_audience(Some(AUDIENCE.to_owned()))
            .with_purposes(vec![PurposeOfUse {
                system: Some(ACT_REASON.to_owned()),
                code: "TREAT".to_owned(),
            }]),
    )
}
