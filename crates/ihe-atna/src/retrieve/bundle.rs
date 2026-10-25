// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The Retrieve ATNA Audit Event Response, as `fhir_types::r4` resources.
//!
//! `RESTful` ATNA supplement §3.81.4.2: the `searchset` Bundle of matching
//! `AuditEvent` resources (§3.81.4.2.2.2) and the `OperationOutcome` of a
//! refusal.

use fhir_types::r4::audit_event::AuditEvent;
use fhir_types::r4::bundle::{Bundle, BundleEntry, BundleEntrySearch, BundleLink};
use fhir_types::r4::operation_outcome::{OperationOutcome, OperationOutcomeIssue};
use fhir_types::r4::primitives;
use fhir_types::r4::resource::Resource;

/// One matching record of a page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// The absolute URL of the record, `entry.fullUrl`; `None` when the
    /// repository serves no read of a single record.
    pub full_url: Option<String>,
    /// The record.
    pub resource: AuditEvent,
}

/// The `issue.code` of a refusal, from the FHIR R4 `issue-type` code
/// system (<http://hl7.org/fhir/R4/codesystem-issue-type.html>): the codes a
/// retrieval answers with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueType {
    /// `invalid`: the query cannot be read.
    Invalid,
    /// `login`: the caller is not authenticated.
    Login,
    /// `forbidden`: the caller may not read what it asked for.
    Forbidden,
    /// `not-found`: the repository serves no retrieval here.
    NotFound,
    /// `not-supported`: the request asks for something not supported.
    NotSupported,
    /// `too-costly`: the result would be too large to return.
    TooCostly,
    /// `transient`: the store cannot be reached now.
    Transient,
    /// `exception`: the repository failed.
    Exception,
}

impl IssueType {
    /// The code.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Invalid => "invalid",
            Self::Login => "login",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not-found",
            Self::NotSupported => "not-supported",
            Self::TooCostly => "too-costly",
            Self::Transient => "transient",
            Self::Exception => "exception",
        }
    }
}

/// Returns the `searchset` Bundle of one page of matches (§3.81.4.2.2.2).
///
/// `total` is every match across all pages (FHIR R4 `Bundle.total`),
/// `self_url` the request as the repository read it, and `next_url` the
/// next page when there is one (<http://hl7.org/fhir/R4/http.html#paging>).
/// No match gives an empty Bundle, which §3.81.4.2.2 answers with a `200`.
#[must_use]
pub fn searchset(
    total: u32,
    self_url: &str,
    next_url: Option<&str>,
    matches: Vec<Match>,
) -> Bundle {
    let mut link = vec![BundleLink {
        relation: primitives::String::from("self"),
        url: primitives::Uri::from(self_url),
        ..BundleLink::default()
    }];
    if let Some(next) = next_url {
        link.push(BundleLink {
            relation: primitives::String::from("next"),
            url: primitives::Uri::from(next),
            ..BundleLink::default()
        });
    }
    Bundle {
        r#type: primitives::Code::from("searchset"),
        total: Some(primitives::UnsignedInt::from(total)),
        link,
        entry: matches.into_iter().map(entry).collect(),
        ..Bundle::default()
    }
}

/// The entry of one match, `search.mode` `match`
/// (<http://hl7.org/fhir/R4/codesystem-search-entry-mode.html>).
fn entry(found: Match) -> BundleEntry {
    BundleEntry {
        full_url: found.full_url.map(primitives::Uri::from),
        resource: Some(Resource::AuditEvent(Box::new(found.resource))),
        search: Some(BundleEntrySearch {
            mode: Some(primitives::Code::from("match")),
            ..BundleEntrySearch::default()
        }),
        ..BundleEntry::default()
    }
}

/// Returns the `OperationOutcome` of a refused retrieval: one `error` issue
/// with `code` and the human-readable `diagnostics` §3.81.4.2.2 asks a
/// repository to complement its error code with.
///
/// `diagnostics` is shown to the caller, so it never names a patient.
#[must_use]
pub fn refusal(code: IssueType, diagnostics: &str) -> OperationOutcome {
    OperationOutcome {
        issue: vec![OperationOutcomeIssue {
            severity: primitives::Code::from("error"),
            code: primitives::Code::from(code.code()),
            diagnostics: Some(primitives::String::from(diagnostics)),
            ..OperationOutcomeIssue::default()
        }],
        ..OperationOutcome::default()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn an_empty_page_is_a_searchset_with_its_total_and_self_link() {
        let bundle = searchset(
            0,
            "https://arr.example.org/AuditEvent?date=ge2013-01-01",
            None,
            Vec::new(),
        );
        assert_eq!(
            serde_json::to_value(&bundle).expect("serializes"),
            json!({
                "resourceType": "Bundle",
                "type": "searchset",
                "total": 0,
                "link": [{
                    "relation": "self",
                    "url": "https://arr.example.org/AuditEvent?date=ge2013-01-01"
                }]
            })
        );
    }

    #[test]
    fn a_match_is_an_entry_in_match_mode_with_a_next_link() {
        let bundle = searchset(
            3,
            "https://arr.example.org/AuditEvent?date=2013&_count=1",
            Some("https://arr.example.org/AuditEvent?date=2013&_count=1&_offset=1"),
            vec![Match {
                full_url: Some("https://arr.example.org/AuditEvent/23".into()),
                resource: AuditEvent {
                    id: Some("23".into()),
                    ..AuditEvent::default()
                },
            }],
        );
        let value = serde_json::to_value(&bundle).expect("serializes");
        assert_eq!(value["total"], 3);
        assert_eq!(value["link"][1]["relation"], "next");
        assert_eq!(value["entry"][0]["search"]["mode"], "match");
        assert_eq!(
            value["entry"][0]["fullUrl"],
            "https://arr.example.org/AuditEvent/23"
        );
        assert_eq!(value["entry"][0]["resource"]["resourceType"], "AuditEvent");
    }

    #[test]
    fn a_refusal_is_one_error_issue() {
        let outcome = refusal(IssueType::Invalid, "the query names no date parameter");
        assert_eq!(
            serde_json::to_value(&outcome).expect("serializes"),
            json!({
                "resourceType": "OperationOutcome",
                "issue": [{
                    "severity": "error",
                    "code": "invalid",
                    "diagnostics": "the query names no date parameter"
                }]
            })
        );
    }
}
