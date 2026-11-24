// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The receiving side of the ATX: FHIR Feed (ITI TF-2 §3.20.4.2 to
//! §3.20.4.5, `RESTful` ATNA Rev. 3.6): the `create` of one `AuditEvent` and
//! the `batch` Bundle of them, in JSON or XML (Appendix Z.6).
//!
//! [`receive`] reads the body the product's HTTP server took, hands each
//! `AuditEvent` to the [`AuditStore`] and returns the [`Answer`] the server
//! writes. A record is answered with a `2xx` only once the store kept it; a
//! store that cannot keep it now is answered with a `503`, which the
//! sender's forwarder retries from its spool, so no record is lost between
//! the two.

use fhir_types::r4::audit_event::AuditEvent;
use fhir_types::r4::bundle::{Bundle, BundleEntry, BundleEntryResponse};
use fhir_types::r4::operation_outcome::OperationOutcome;
use fhir_types::r4::primitives;
use fhir_types::r4::resource::Resource;
use http::StatusCode;

use super::{AuditStore, StoreError};
use crate::retrieve::bundle::{IssueType, refusal};

/// The format of a request body (ITI TF-2 Appendix Z.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// `application/fhir+json`.
    Json,
    /// `application/fhir+xml`.
    Xml,
}

impl Format {
    /// The format of the media type `content_type`, its parameters ignored,
    /// or `None` for a type ITI TF-2 Appendix Z.6 does not list.
    #[must_use]
    pub fn of(content_type: &str) -> Option<Self> {
        let essence = content_type.split(';').next().unwrap_or_default().trim();
        match essence.to_ascii_lowercase().as_str() {
            "application/fhir+json" | "application/json" | "application/json+fhir" => {
                Some(Self::Json)
            }
            "application/fhir+xml" | "application/xml" | "application/xml+fhir" => Some(Self::Xml),
            _ => None,
        }
    }
}

/// What the repository answers a FHIR Feed request with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The HTTP status.
    pub status: StatusCode,
    /// The `Location` of a created record, `AuditEvent/<id>`, when the store
    /// gave it an id.
    pub location: Option<String>,
    /// The body: the `batch-response` Bundle of a batch, or the
    /// `OperationOutcome` of a refusal.
    pub body: Option<Resource>,
}

impl Answer {
    fn refused(status: StatusCode, code: IssueType, diagnostics: &str) -> Self {
        Self {
            status,
            location: None,
            body: Some(Resource::OperationOutcome(Box::new(refusal(
                code,
                diagnostics,
            )))),
        }
    }
}

/// Answers the FHIR Feed request whose body is `body` in `format`: the
/// `create` of an `AuditEvent` (§3.20.4.2) or a `batch` Bundle of them
/// (§3.20.4.4).
///
/// A body that is no FHIR R4 resource, such as an `AuditEvent` without an
/// element R4 requires, is answered `400`; a resource that is no
/// `AuditEvent` and no `batch` Bundle, or an `AuditEvent` the store refuses,
/// `422`; a store that cannot keep the record now, `503`.
pub async fn receive(store: &dyn AuditStore, format: Format, body: &[u8]) -> Answer {
    let resource = match read(format, body) {
        Ok(resource) => resource,
        Err(answer) => return answer,
    };
    match resource {
        Resource::AuditEvent(record) => {
            let (status, location, outcome) = keep(store, *record).await;
            Answer {
                status,
                location,
                body: outcome.map(|outcome| Resource::OperationOutcome(Box::new(outcome))),
            }
        }
        Resource::Bundle(bundle) => batch(store, *bundle).await,
        _ => Answer::refused(
            StatusCode::UNPROCESSABLE_ENTITY,
            IssueType::Invalid,
            "the body is neither an AuditEvent nor a batch Bundle of AuditEvents",
        ),
    }
}

/// Reads the body as a resource.
fn read(format: Format, body: &[u8]) -> Result<Resource, Answer> {
    let unreadable = || {
        Answer::refused(
            StatusCode::BAD_REQUEST,
            IssueType::Invalid,
            "the body is no FHIR R4 resource in the format its media type names",
        )
    };
    match format {
        Format::Json => serde_json::from_slice(body).map_err(|_quotes_the_body| unreadable()),
        Format::Xml => {
            let text = std::str::from_utf8(body).map_err(|_quotes_the_body| unreadable())?;
            let object = fhir_types::xml::from_xml(&fhir_types::r4::schema::SCHEMAS, text)
                .map_err(|_quotes_the_body| unreadable())?;
            let value = fhir_types::codec::Value::Object(object)
                .into_serde_json(&mut fhir_types::codec::Path::root("Resource"))
                .map_err(|_quotes_the_body| unreadable())?;
            serde_json::from_value(value).map_err(|_quotes_the_body| unreadable())
        }
    }
}

/// Keeps one record: the status, the `Location` and the refusal of its
/// answer.
async fn keep(
    store: &dyn AuditStore,
    record: AuditEvent,
) -> (StatusCode, Option<String>, Option<OperationOutcome>) {
    match store.keep(record).await {
        Ok(id) => (
            StatusCode::CREATED,
            id.map(|id| format!("AuditEvent/{id}")),
            None,
        ),
        Err(StoreError::Refused(error)) => {
            tracing::warn!(target: crate::atna::targets::REPOSITORY, error = %crate::atna::chain(error.as_ref()), "the audit store refused an AuditEvent");
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                None,
                Some(refusal(
                    IssueType::Invalid,
                    "the repository does not keep this AuditEvent",
                )),
            )
        }
        Err(StoreError::Unavailable(error)) => {
            tracing::warn!(target: crate::atna::targets::REPOSITORY, error = %crate::atna::chain(error.as_ref()), "the audit store is unavailable; an AuditEvent is refused for its sender to retry");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                None,
                Some(refusal(
                    IssueType::Transient,
                    "the repository cannot keep the AuditEvent now; send it again",
                )),
            )
        }
    }
}

/// Answers a `batch` Bundle (§3.20.4.4.2, Table 3.20.4.4.2-1): one
/// `batch-response` entry per entry, each with its own status
/// (§3.20.4.5.2).
async fn batch(store: &dyn AuditStore, bundle: Bundle) -> Answer {
    let batch = bundle.r#type.value.as_deref() == Some("batch");
    if !batch || bundle.entry.is_empty() {
        return Answer::refused(
            StatusCode::UNPROCESSABLE_ENTITY,
            IssueType::Invalid,
            "a FHIR Feed Bundle is of type batch with at least one AuditEvent",
        );
    }
    let mut entries = Vec::with_capacity(bundle.entry.len());
    for entry in bundle.entry {
        let post = entry
            .request
            .as_ref()
            .and_then(|request| request.method.value.as_deref())
            == Some("POST");
        let (status, location, outcome) = match entry.resource {
            Some(Resource::AuditEvent(record)) if post => keep(store, *record).await,
            _ => (
                StatusCode::UNPROCESSABLE_ENTITY,
                None,
                Some(refusal(
                    IssueType::Invalid,
                    "each entry is a POST of an AuditEvent",
                )),
            ),
        };
        entries.push(BundleEntry {
            response: Some(BundleEntryResponse {
                status: primitives::String::from(status_line(status)),
                location: location.map(primitives::Uri::from),
                outcome: outcome.map(|outcome| Resource::OperationOutcome(Box::new(outcome))),
                ..BundleEntryResponse::default()
            }),
            ..BundleEntry::default()
        });
    }
    Answer {
        status: StatusCode::OK,
        location: None,
        body: Some(Resource::Bundle(Box::new(Bundle {
            r#type: primitives::Code::from("batch-response"),
            entry: entries,
            ..Bundle::default()
        }))),
    }
}

/// The `Bundle.entry.response.status` of `status`: its code and reason
/// phrase (FHIR R4 Bundle).
fn status_line(status: StatusCode) -> String {
    match status.canonical_reason() {
        Some(reason) => format!("{} {reason}", status.as_str()),
        None => status.as_str().to_owned(),
    }
}
