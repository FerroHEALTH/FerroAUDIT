// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The URL of an interaction under a FHIR base.

use url::Url;

/// Returns the URL of `interaction` under the FHIR base `base`, or `None` when
/// `base` is not an `http` or `https` URL without a query or a fragment
/// (FHIR R4 HTTP, Service Base URL,
/// <http://hl7.org/fhir/R4/http.html#root>).
pub(crate) fn under_base(mut base: Url, interaction: &str) -> Option<Url> {
    if !matches!(base.scheme(), "http" | "https")
        || base.cannot_be_a_base()
        || base.query().is_some()
        || base.fragment().is_some()
    {
        return None;
    }
    if !base.path().ends_with('/') {
        let path = format!("{}/", base.path());
        base.set_path(&path);
    }
    base.join(interaction).ok()
}
