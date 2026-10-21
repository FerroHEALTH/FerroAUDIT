// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! BALP with the feature `recording` alone.
//!
//! The bound on an audited exchange's record, for a client that audits through its own model, without the FHIR
//! `AuditEvent` writer and the Feed the feature `balp` adds. The path is the
//! one `balp` gives it, so a caller names it the same way under either
//! feature. No specification governs the feature layout: our own design.

#[path = "recording.rs"]
pub mod recording;
