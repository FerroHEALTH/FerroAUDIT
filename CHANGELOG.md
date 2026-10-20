<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - Unreleased

The first release: the shared IHE audit crate of the FerroHEALTH products,
moved with its history and its tests from the `atna` and `balp` features of
FerroFED's `ihe-iti`, and the repository setup. A product that used
`ihe_iti::atna`, `ihe_iti::balp` or `ihe_iti::user` depends on
`ihe-atna = "=0.1.0"` with the same feature and reads the same items from
`ihe_atna::atna`, `ihe_atna::balp` and `ihe_atna::user`.

### Added

- `atna`: the sending half of ITI-20 Record Audit Event. The DICOM PS3.15
  Annex A.5 audit message with its participant objects held as secrets; the
  RFC 5424 syslog message with the PRI `<85>` and the MSGID `IHE+RFC-3881`
  ITI TF-2 §3.20.4.1.2 fixes, in its RFC 5425 frame; the TLS connection to an
  Audit Record Repository with the platform's roots and an optional client
  certificate; the bounded store-and-forward spool of §3.20.4.1.1, one
  fsynced file per message in a private directory, with a time bound on each
  write and a quarantine for an unreadable message; and the forwarder that
  stores first, delivers oldest first and retries with a jittered backoff.
- `balp`: the FHIR R4 `AuditEvent` of an IHE FHIR transaction, written from
  one `Exchange` to the BALP 1.1.4 patterns, with the user's agents and
  purposes of use; the `AuditRecorder` seam an audited client records
  through; and the ATX: FHIR Feed of the RESTful ATNA supplement, a FHIR
  `create` of each stored record, a refused record quarantined.
- `user`: whom an audited exchange is made for, compiled with no feature.
- The repository setup: the design of record (`docs/architecture.md`), the
  pin matrix, the vendored ITI-20 page and RESTful ATNA supplement and the
  BALP package and pages, CI with a single `conclusion` check, the vendored
  specification guard, CodeQL, Scorecard, Dependabot, and the release lane
  with crates.io Trusted Publishing.

[Unreleased]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/FerroHEALTH/FerroAUDIT/releases/tag/v0.1.0
