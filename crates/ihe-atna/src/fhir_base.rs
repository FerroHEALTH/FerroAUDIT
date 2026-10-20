// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The URL of an interaction under a FHIR base, which the FHIR Feed posts to
//! and which a product that calls a FHIR server joins the same way.

use url::Url;

/// Returns the URL of `interaction` under the FHIR base `base`, or `None` when
/// `base` is not an `http` or `https` URL without a query or a fragment
/// (FHIR R4 HTTP, Service Base URL,
/// <http://hl7.org/fhir/R4/http.html#root>).
#[must_use]
pub fn under_base(mut base: Url, interaction: &str) -> Option<Url> {
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

#[cfg(test)]
mod tests {
    use url::Url;

    use super::under_base;

    #[test]
    fn an_interaction_joins_under_the_base_with_or_without_its_slash() {
        for base in ["https://arr.example.org/fhir", "https://arr.example.org/fhir/"] {
            let joined = under_base(Url::parse(base).expect("a URL"), "AuditEvent");
            assert_eq!(
                joined.map(String::from).as_deref(),
                Some("https://arr.example.org/fhir/AuditEvent"),
                "{base}"
            );
        }
    }

    #[test]
    fn a_base_of_another_scheme_or_with_a_query_or_fragment_is_refused() {
        for base in [
            "tls://arr.example.org:6514",
            "https://arr.example.org/fhir?x=1",
            "https://arr.example.org/fhir#f",
            "urn:uuid:1e7f2a52-0d1c-4b0f-9a52-5d3c1c0e2f11",
        ] {
            assert!(
                under_base(Url::parse(base).expect("a URL"), "AuditEvent").is_none(),
                "{base}"
            );
        }
    }
}
