<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.1] - 2026-10-21

A patch release with no API change: a product pins `=0.2.1` in place of
`=0.2.0` and changes nothing else.

### Added

- `ihe-atna`: the feature `recording` compiles `balp::recording` alone (tokio
  and thiserror only), and the feature `redact` compiles
  `redact::RedactedUrl` and `fhir_base` alone (url only), so a profile crate
  such as FerroPIX's `ihe-iti` takes the record bound and the URL helpers
  without the BALP or syslog stack and drops its own copies (#61).
- `ihe-atna`: `balp::recording::Late::new` (and `Default`), so an audited
  client outside the crate can return the late-record failure it reports;
  `Late` is `#[non_exhaustive]`, which kept 0.2.0's callers from making one
  (#61).

## [0.2.0] - 2026-10-21

The audit content the FerroFED and FerroEHR migrations found missing, the
correctness of the EHDS retention, and the packaging those migrations asked
for. Public APIs of both crates break, so a product changes what the
**Changed** entries name.

### Added

- `ihe-atna`: `balp::USER_AUTHENTICATION`, `LOGIN`, `LOGOUT`, `EXPORT`,
  `IMPORT` (DICOM PS3.15 A.5.3) and `OPERATION` (FHIR R4
  `restful-interaction`) (#43); `Outcome::MajorFailure`, written as `12`
  (#44); `Exchange::outcome_description` (`outcomeDesc`), `Observer::source_types`
  (`source.type`) and the `SOURCE_*` codes of `security-source-type` (#45);
  `User::with_token_id`, written at `agent[user].policy`, and the claim of
  `IHE.BasicAudit.OAUTHaccessTokenUse.Comprehensive` when the record holds to
  it (#28); `User::local`, `with_issuer` and `with_client_id`, for a user no
  OAuth token named (#47); `Exchange::audit_event_value` (#57).
- `ihe-atna`: the optional DICOM elements `EventOutcomeDescription`,
  `AuditSourceTypeCode` (`message::SourceType`) and
  `ParticipantObjectDataLifeCycle` (#49), and `atna::codes`, the DICOM and
  RFC 3881 code tables (#52).
- `ihe-atna`: `balp::recording` with `Late` and `within`, the bound on an
  audited exchange's record (#8); public `redact::{REDACTED, RedactedUrl}`
  and `fhir_base::under_base` (#9); `atna::targets`, the stable `tracing`
  targets `ihe_atna::atna::spool` and `ihe_atna::atna::forwarder`, which
  every spool and forwarder event names (#10).
- `ihe-atna`: `feed::Credentials` and `FeedRepository::with_credentials`:
  each Feed request is authorized from the source, and a refused token is
  renewed once (RFC 6750 §3.1) (#53); `forwarder::Outbox` with
  `Forwarder::syslog_from` and `Forwarder::fhir_feed_from`, delivery from a
  caller's own durable store (#55); `Connection` is `Sync` (#51).
- `ehds-logging`: `map::Matching::OpenEhr` and `CategoryMap::declare_with`,
  case-insensitive template and archetype keys as openEHR BASE asks, with the
  evidence kept in its own case (#16); every defect of a declaration at once
  (`MapErrors`) (#17); archetype keys held to the `ARCHETYPE_ID` grammar
  (#18); `CategoryMap::templates`, `archetypes` and `national` (#19);
  `Declared` serializes (#20).
- `ehds-logging`: `classify::ResourceKind`, `Evidence::resource`,
  `RootObject::kind` and `Basis::ResourceKind`, so a record of no category
  says which resource it reached and a no-category object sits beside content
  (#21, #22, #24); `CategoryMap::with_evidence_limit` and
  `Classification::evidence_counts` (#23).
- `ehds-logging`: `Accessor::roles`, written at `agent[user].role` beside the
  acting mode, which `balp::exchange` no longer overwrites (#27);
  `Accessor::token_id` (#28); `AccessRecord::origin_count` (#32),
  `AccessRecord::outcome_description` and `Outcome::MajorFailure`;
  `Request::operation_system`, the operation written as a subtype (#36).
- `ehds-logging`: `retention::Period` (years, days or `Indefinite`) and
  `retention::Bounds` with national floors, ceilings and floors outside the
  Union, each with its legal ground, through `RetentionPolicy::declare_within`;
  a record kept without end names no end date, a period in days must reach the
  floor from any date of access, and every record names the ground of its
  floor (#37, #38, #39, #40).
- The vendored IETF syslog and audit RFCs, the openEHR BASE Base Types
  specification and the pin of DICOM PS3.15 2026c, read locally and never
  committed; doctests for the syslog sender, the FHIR Feed and the BALP sink
  (#13); `ehds-logging`'s own feature-boundary test (#12).

### Changed

- **`ihe-atna`: the `balp` feature no longer implies `atna`** and compiles no
  syslog sender, DICOM message or RFC 5425 TLS stack; the spool, the
  forwarder and the Feed compile under it. `ihe-atna` no longer enables
  `reqwest`'s `rustls` feature: a product enables its TLS backend on its own
  `reqwest` (#11). A product that used the syslog sender with `balp` enables
  `atna` too.
- **`ihe-atna`: `balp::Coded` holds `Cow<'static, str>`** and is no longer
  `Copy`; build one with `Coded::fixed` or `Coded::new`. `EventKind::subtypes`
  is a `Vec<Coded>` (#42).
- **`ihe-atna`: `Direction::Received::endpoint` is a `NetworkAddress`** (use
  `NetworkAddress::uri(&url)` where a URL was given) (#46); `Entity::Patient`
  takes an optional `system` and a `namespace` (#33); `Exchange` has
  `outcome_description` and `Observer` has `source_types`.
- **`ihe-atna`: `User::issuer` and `User::client_id` return `Option<&str>`**;
  `atna_user_name` of a user with no issuer is the account (#47).
- **`ihe-atna`: `FeedRepository::cleartext_for_development` is now
  `FeedRepository::cleartext`** (#54), and a `401` is `FeedError::Unauthorized`,
  retried and never quarantined (#53).
- **`ihe-atna`: `Forwarder` is generic over its store** (`Forwarder<O: Outbox
  = Spool>`), so `Forwarder` alone still names the spool forwarder (#55).
- **`ehds-logging`: `Accessor::issuer`, `client_id` and `acting` are
  `Option`** (#25, #26); `PatientIdentifier::namespace`, `Origin::endpoint`,
  `Origin::status` and `Request::id` are `Option` (#33, #31, #35); `Origin::key`
  is the origin's endpoint, else its `system_id`.
- **`ehds-logging`: `BalpSink::new` and `balp::exchange` take a
  `NetworkAddress`** in place of a `Url` (#46).
- **`ehds-logging`: `Retention::years` is now `Retention::period`**,
  `Retention::ends` returns `Option<Date>` and `RetentionPolicy::longest`
  returns a `Period`; the record writes `ehds-retention-period` and
  `ehds-retention-floor`, and `ehds-retention-years` and `ehds-retention-ends`
  only when they apply.

## [0.1.1] - 2026-10-20

A patch release with no API change: a product pins `=0.1.1` in place of
`=0.1.0` and changes nothing else.

### Fixed

- Both crates declare `rust-version = "1.97"`, the FerroHEALTH family floor,
  in place of 1.98, which no feature needed. A product whose MSRV is 1.97,
  such as FerroEHR, can now take them, and its `cargo hack check
  --rust-version` lane passes. CI checks the declared MSRV on every pull
  request with no feature and with `atna`, so a change that needs a newer
  toolchain fails before it merges. The `balp` feature of both crates takes
  `fhir-types`, which declares `rust-version = "1.98"` itself, so a build
  with `balp` still needs 1.98, and Cargo says so by naming `fhir-types`
  (#14).

## [0.1.0] - 2026-10-20

The first release: the shared IHE audit crate of the FerroHEALTH products,
moved with its history and its tests from the `atna` and `balp` features of
FerroFED's `ihe-iti`, the EHDS logging component `ehds-logging`, moved from
FerroFED with its tests, and the repository setup. A product that used
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
- `ehds-logging`, moved from FerroFED (0.0.26 there) onto this repository's
  version line: the logging software component of an EHR system under
  Regulation (EU) 2025/327, with the Annex II 3.2 access record, the Article
  14(1) categories and the map that classifies an access, the Article 9(2)
  retention, the Annex II 3.4 security labels, and the record written as a
  BALP `AuditEvent`. Its `balp` feature now builds on `ihe-atna` in place of
  `ihe-iti`, and `BalpSink::new` and `balp::exchange` name their URL
  parameter `base`. A product that used `ehds-logging` 0.0.26 pins
  `ehds-logging = "=0.1.0"`; the API is otherwise unchanged (#4).
- The repository setup: the design of record (`docs/architecture.md`), the
  pin matrix, the vendored ITI-20 page and RESTful ATNA supplement, the BALP
  package and pages, the EHDS Regulation with the acts and guidelines
  `ehds-logging` cites, the HL7 Europe and MyHealth@EU packages, CI with a single `conclusion` check, the vendored
  specification guard, CodeQL, Scorecard, Dependabot, and the release lane
  with crates.io Trusted Publishing.

[Unreleased]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.1...HEAD
[0.2.1]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/FerroHEALTH/FerroAUDIT/releases/tag/v0.1.0
