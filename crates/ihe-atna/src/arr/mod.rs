// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The Audit Record Repository (feature `arr`).
//!
//! It is the receiving side of Record Audit Event [ITI-20] with the
//! Retrieve Audit Message Option, which answers Retrieve ATNA Audit Event
//! [ITI-81] over the records it received (`RESTful` ATNA supplement Rev.
//! 3.6, ITI TF-1 §9.2.3).
//!
//! The repository is a crate a product mounts over its own store, the
//! caller's [`AuditStore`]: it keeps what the product keeps, and answers
//! the searches the product lets a caller run.
//!
//! - [`syslog`] receives ITI-20 messages over RFC 5425 TLS (ITI TF-2
//!   §3.20.4.1). A frame is stored in the repository's [`Spool`] before the
//!   next is read, and a [`Forwarder`] delivers each from it, oldest first,
//!   into the store, as the spool rules have a sender do, since syslog has
//!   no acknowledgement to refuse a frame with. The DICOM audit message is
//!   kept as the `AuditEvent` the normative mapping of `RESTful` ATNA Table
//!   3.81.4.2.2.1-1 gives ([`dicom`]).
//! - [`feed`] receives the ATX: FHIR Feed's `create` of one `AuditEvent`
//!   (§3.20.4.2) and its `batch` Bundle (§3.20.4.4). It answers a record
//!   with a `2xx` only once the store has kept it, so the sender keeps every
//!   other record and sends it again.
//! - [`search`] answers ITI-81 over the store (§3.81.4): the query read by
//!   [`crate::retrieve::Query`], every parameter of §3.81.4.1.2 matched by
//!   [`search::matches()`], and the "Audit Log Used" record §3.81.5.1 asks the
//!   repository to keep of every retrieval, kept before the answer is
//!   given.
//!
//! [`Spool`]: crate::atna::spool::Spool
//! [`Forwarder`]: crate::atna::forwarder::Forwarder

pub mod dicom;
pub mod feed;
pub mod search;
pub mod syslog;

use std::fmt;

use fhir_types::r4::audit_event::AuditEvent;

use crate::retrieve::Query;
use crate::retrieve::bundle::Match;

/// Where the repository keeps its records and how it searches them: the
/// product's own store.
///
/// The repository hands every record it receives to [`AuditStore::keep`] and
/// counts it as kept once that returns `Ok`. ITI TF-2 §3.20.4.2.3 lets the
/// repository discard a record as irrelevant; a store that does so returns
/// `Ok` too.
#[async_trait::async_trait]
pub trait AuditStore: Send + Sync + fmt::Debug {
    /// Keeps `record` durably, returning the logical id the store gave it,
    /// which a FHIR Feed `create` answers in its `Location`, or `None` when
    /// it gave none or discarded the record.
    ///
    /// # Errors
    ///
    /// [`StoreError::Refused`] when the store will never keep this record,
    /// and [`StoreError::Unavailable`] when it cannot keep it now; the
    /// repository then holds or refuses the record as its transport allows.
    async fn keep(&self, record: AuditEvent) -> Result<Option<String>, StoreError>;

    /// The page of records that match `query`, which `requester` may read
    /// (ITI TF-1 §9.4), from the `query`'s offset and at most its count.
    ///
    /// A store may evaluate the query with [`search::matches()`], or with its
    /// own index under the same rules.
    ///
    /// # Errors
    ///
    /// A [`StoreError`] when the store cannot be searched.
    async fn search(&self, query: &Query, requester: &Requester) -> Result<Found, StoreError>;
}

/// One page of matches and the number of matches across every page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Found {
    /// Every match across all pages, FHIR R4 `Bundle.total`.
    pub total: u32,
    /// The page.
    pub matches: Vec<Match>,
}

/// Why a store did not do what the repository asked.
///
/// The cause is the store's, so it never carries a record.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The store will never keep the record.
    #[error("the audit store refused the record")]
    Refused(#[source] Box<dyn std::error::Error + Send + Sync>),
    /// The store cannot be reached or written now.
    #[error("the audit store is unavailable")]
    Unavailable(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Who asks the repository for records: the Audit Consumer, which the
/// "Audit Log Used" record names (§3.81.5.1).
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Requester {
    /// The authenticated human requestor, when known.
    pub user: Option<String>,
    /// The machine name or IP address the request came from.
    pub address: Option<String>,
}

impl fmt::Debug for Requester {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Requester")
            .field("user", &self.user.as_ref().map(|_| crate::redact::REDACTED))
            .field("address", &self.address)
            .finish()
    }
}
