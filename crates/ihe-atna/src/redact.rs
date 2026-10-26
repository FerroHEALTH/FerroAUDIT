// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The `Debug` rendering of a value that may name a patient or carry a
//! credential.
//!
//! A URL may carry a user name and password in its userinfo (RFC 3986
//! §3.2.1) and a credential in its query, so every type of this crate that
//! holds one shows it through `RedactedUrl` with both replaced by
//! [`REDACTED`]. Both render through `ferrohealth-secret`, the family's one
//! mask, so a product that logs a URL shows it the same way. No
//! specification governs this: our own design.

#[cfg(any(feature = "atna", feature = "balp", feature = "redact"))]
use std::fmt;

/// The fixed text a rendering shows in place of a credential or of a value
/// that may name a patient: the family mask of `ferrohealth-secret`.
pub const REDACTED: &str = ferrohealth_secret::REDACTED;

/// A URL whose `Debug` shows it with its userinfo and its query redacted.
///
/// The rendering is `ferrohealth_secret::secret_url::RedactedUrl`'s: the
/// userinfo becomes `***@` and the query `?***`, the scheme, the host, the
/// port, the path and the fragment show as written, and text with no `://`
/// shows as [`REDACTED`] whole. A text no URL parser accepts has its
/// userinfo taken up to its last `@`.
#[cfg(any(feature = "atna", feature = "balp", feature = "redact"))]
pub struct RedactedUrl<'a>(pub &'a str);

#[cfg(any(feature = "atna", feature = "balp", feature = "redact"))]
impl fmt::Debug for RedactedUrl<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&ferrohealth_secret::secret_url::RedactedUrl(self.0), f)
    }
}

/// Returns `url` with its userinfo and its query replaced by [`REDACTED`].
#[cfg(all(test, any(feature = "atna", feature = "balp", feature = "redact")))]
fn redact(url: &str) -> String {
    ferrohealth_secret::secret_url::RedactedUrl(url).to_string()
}

#[cfg(all(test, any(feature = "atna", feature = "balp", feature = "redact")))]
mod tests {
    use super::{REDACTED, RedactedUrl, redact};

    #[test]
    fn every_part_that_may_carry_a_credential_is_redacted() {
        for (raw, redacted) in [
            (
                "https://Qz7user@pix.example.org/fhir",
                "https://***@pix.example.org/fhir",
            ),
            (
                "https://Qz7user:Qz7password@pix.example.org:8443/fhir/Patient/$ihe-pix",
                "https://***@pix.example.org:8443/fhir/Patient/$ihe-pix",
            ),
            (
                "https://u:p@ss@pix.example.org/",
                "https://***@pix.example.org/",
            ),
            (
                "https://u:p@pix.example.org?x=1",
                "https://***@pix.example.org?***",
            ),
            (
                "https://pdq.example.org/fhir/Patient?_page=Qz7token#f",
                "https://pdq.example.org/fhir/Patient?***#f",
            ),
            ("urn:uuid:1e7f2a52-0d1c-4b0f-9a52-5d3c1c0e2f11", REDACTED),
            ("u:Qz7password@pix.example.org/x", REDACTED),
        ] {
            assert_eq!(redacted, redact(raw), "{raw}");
        }
    }

    #[test]
    fn a_password_holding_a_delimiter_is_redacted() {
        for (raw, redacted) in [
            ("https://u:p/x@host", "https://***@host"),
            (
                "https://u:Qz7/pass@pix.example.org/fhir",
                "https://***@pix.example.org/fhir",
            ),
            (
                "https://u:Qz7?pass@pix.example.org/fhir?x=1",
                "https://***@pix.example.org/fhir?***",
            ),
            (
                "https://u:Qz7#pass@pix.example.org/fhir",
                "https://***@pix.example.org/fhir",
            ),
        ] {
            assert_eq!(redacted, redact(raw), "{raw}");
        }
    }

    #[test]
    fn an_at_sign_after_the_authority_is_not_userinfo() {
        for (raw, redacted) in [
            (
                "https://directory.example.org/fhir/Endpoint/@handle",
                "https://directory.example.org/fhir/Endpoint/@handle",
            ),
            (
                "https://pix.example.org/fhir/a@b/Patient",
                "https://pix.example.org/fhir/a@b/Patient",
            ),
            (
                "https://pix.example.org/fhir/Patient?email=someone@example.org",
                "https://pix.example.org/fhir/Patient?***",
            ),
            (
                "https://pix.example.org/fhir#section@b",
                "https://pix.example.org/fhir#section@b",
            ),
        ] {
            assert_eq!(redacted, redact(raw), "{raw}");
        }
    }

    #[test]
    fn text_that_does_not_parse_is_redacted_to_its_last_at_sign() {
        assert_eq!(
            "https://***@handle",
            redact("https://pix example.org/fhir/@handle")
        );
    }

    #[test]
    fn a_url_without_a_credential_shows_as_written() {
        for raw in [
            "https://pix.example.org/fhir/Patient/$ihe-pix",
            "https://directory.example.org/fhir/Endpoint/@handle",
            "http://127.0.0.1:8080/fhir/",
            "",
        ] {
            assert_eq!(raw, redact(raw), "{raw}");
        }
    }

    #[test]
    fn debug_quotes_the_redacted_url() {
        let shown = format!(
            "{:?}",
            RedactedUrl("https://u:Qz7password@pix.example.org/")
        );
        assert_eq!("\"https://***@pix.example.org/\"", shown);
    }
}
