// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The IHE audit trail, one feature per layer:
//!
//! - `atna`: Audit Trail and Node Authentication, the sending half of ITI-20
//!   Record Audit Event (ITI TF-2 §3.20): the DICOM PS3.15 audit message over
//!   syslog with TLS, the store-and-forward spool and the forwarder.
//! - `balp`: the Basic Audit Log Patterns, the FHIR R4 `AuditEvent` an IHE
//!   FHIR transaction is audited as, sent over ITI-20's ATX: FHIR Feed
//!   through the same spool and forwarder.
//!
//! Module `user` names whom an audited exchange is made for, a user an OAuth
//! 2.0 access token names or the system itself, and compiles with or without
//! a feature.
//!
//! The profiles are published at <https://profiles.ihe.net/ITI/>. The crate
//! depends on no application: it is the audit transactions as Rust, for any
//! caller.
#![doc(test(attr(deny(warnings))))]

#[cfg(feature = "atna")]
pub mod atna;
#[cfg(feature = "balp")]
pub mod balp;
mod redact;
#[cfg(feature = "balp")]
mod search;
pub mod user;
