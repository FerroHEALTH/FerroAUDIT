<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->
<div align="center">

<img src="assets/brand/ferroaudit-lockup-auto.svg" alt="FerroAUDIT" width="290">

**The audit trail of the FerroHEALTH products: who looked at which patient's data, recorded the standard way.**

[Documentation](https://docs.rs/ihe-atna) ·
[The crates](#the-crates) ·
[Install](#install) ·
[Licensing](https://ferrohealth.eu/docs/licensing.html)

**Project**<br>
[![Release](https://img.shields.io/github/v/release/FerroHEALTH/FerroAUDIT?sort=semver&logo=github&label=release)](https://github.com/FerroHEALTH/FerroAUDIT/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/FerroHEALTH/FerroAUDIT/ci.yml?branch=main&logo=githubactions&logoColor=white&label=CI)](https://github.com/FerroHEALTH/FerroAUDIT/actions/workflows/ci.yml)
[![License: BUSL-1.1](https://img.shields.io/badge/License-BUSL--1.1-blue)](LICENSE)
[![Docs](https://img.shields.io/docsrs/ihe-atna?logo=docsdotrs&label=docs.rs)](https://docs.rs/ihe-atna)

**Quality and security**<br>
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/FerroHEALTH/FerroAUDIT/badge)](https://scorecard.dev/viewer/?uri=github.com/FerroHEALTH/FerroAUDIT)
[![CodeQL](https://img.shields.io/github/actions/workflow/status/FerroHEALTH/FerroAUDIT/codeql.yml?branch=main&logo=github&label=CodeQL)](https://github.com/FerroHEALTH/FerroAUDIT/actions/workflows/codeql.yml)
[![Signed releases](https://img.shields.io/badge/releases-signed-brightgreen?logo=github)](docs/release.md)
[![SLSA Build L3](https://slsa.dev/images/gh-badge-level3.svg)](docs/release.md)

**Conformance and compliance**<br>
[![EHDS](https://img.shields.io/badge/EHDS-logging%20component-blue?logo=europeanunion)](crates/ehds-logging/README.md)

</div>

## What FerroAUDIT does

Patients and regulators have a right to know who looked at a health record.
Every access to patient data has to be written down, kept safely and handed
to the people who check it. FerroAUDIT does that for every FerroHEALTH
product, in the formats hospitals already use, so their audit systems can
read the records without a custom connection.

It writes each access as a standard audit record (IHE ATNA, the profile
hospitals use for audit trails), sends it to the hospital's audit
repository, and keeps it in a local queue until the repository confirms it,
so a record is never lost when the network is down. It also records what
the European Health Data Space regulation asks of an electronic health
record system: which categories of data were read, by whom, and how long
the record is kept.

### Who it is for

- **Privacy officers and hospital IT**, who receive the audit trail of every
  FerroHEALTH product in their existing audit repository.
- **Developers of health software in Rust**, who need ATNA and EHDS logging
  without writing it again.

### Why you can trust what it claims

Every release is signed and carries SLSA build provenance, so you can check
that the crate you use is the one built here. The source is public, and
every function is in this repository: there is no paid edition with extra
features. A record names a patient toward its repository and nowhere else,
and a record the queue cannot take is refused at once, so nothing is
dropped silently. The IHE and EU texts the crates implement are vendored at
the versions [`docs/VERSIONS.md`](docs/VERSIONS.md) pins, so you can check
each record against its standard.

## Part of FerroHEALTH

FerroHEALTH is five servers, each its own product, that reach each other
only over published standards: FerroEHR stores the record, FerroBRIDGE
translates it to FHIR and the OMOP research model, FerroFED connects records
across organisations and borders, FerroPIX matches patient identities, and
FerroTERM serves the code systems (the shared lists of medical terms). Work
that every server needs lives in three shared libraries: FerroAUTH for
sign-in and access control, FerroAUDIT for the audit trail, and FerroUI for
the web consoles. FerroEHR, FerroFED and FerroPIX send their audit records
through FerroAUDIT, and each keeps its own audit store.

## The crates

Both crates are published on crates.io and released together on one
version line. Each has its own README, which is its crates.io page.

| Crate | Version | What it provides | Features |
|---|---|---|---|
| [`ihe-atna`](crates/ihe-atna/README.md) | [![crates.io](https://img.shields.io/crates/v/ihe-atna?logo=rust&label=crates.io)](https://crates.io/crates/ihe-atna) | The IHE audit trail: ATNA Record Audit Event (ITI-20) with the DICOM audit message over syslog and TLS, BALP `AuditEvent`s over the ATX: FHIR Feed, the store-and-forward spool and forwarder, and Retrieve ATNA Audit Event (ITI-81) | `atna`, `balp`, `smart`, `retrieve`, `recording`, `redact` |
| [`ehds-logging`](crates/ehds-logging/README.md) | [![crates.io](https://img.shields.io/crates/v/ehds-logging?logo=rust&label=crates.io)](https://crates.io/crates/ehds-logging) | The EHDS logging component of Regulation (EU) 2025/327: the Annex II 3.2 access record, the Art 14(1) categories and the map that classifies an access, retention and security labels, written as a BALP `AuditEvent` or a DICOM audit message | `balp`, `atna`, `schema` |

## What it provides

- **ATNA audit messages (ITI-20):** the DICOM PS3.15 audit message in an
  RFC 5424 syslog message, sent over RFC 5425 TLS to any Audit Record
  Repository (feature `atna`).
- **BALP audit events:** the FHIR R4 `AuditEvent` of every IHE FHIR
  transaction, posted over the ATX: FHIR Feed through
  `ferrohealth-smart`'s outbound client (features `balp` and `smart`).
- **A store-and-forward spool** that keeps every record, one fsynced file
  per message, until the repository takes it, oldest first, with a
  quarantine for a record that cannot be read.
- **Retrieving audit events (ITI-81):** the FHIR search a product serves
  over its own audit store, read into typed criteria with the family's FHIR
  search grammar, and the `searchset` answer (feature `retrieve`).
- **The EHDS access record:** who accessed which categories of whose data,
  when and from where, as Annex II 3.2 of Regulation (EU) 2025/327 asks,
  with the deployment's category map, retention within the Art 9(2) floor,
  the Annex II 3.4 security labels and the person's own access trail.

## Install

The crates need Rust 1.98 or newer (edition 2024). Pin the versions
exactly, as every FerroHEALTH product does:

```sh
cargo add ihe-atna@=0.11.1 --features balp
cargo add ehds-logging@=0.11.1 --features balp
```

The release plan refuses a tag while this page or a crate README pins
another version, so these lines name the release being cut.

## Usage

A deployment declares which templates hold which EHDS category, and each
access is classified by what it returned:

```rust
use std::collections::BTreeMap;

use ehds_logging::category::Category;
use ehds_logging::classify::{Basis, Evidence, RootObject};
use ehds_logging::map::{CategoryMap, Declared};

let templates = BTreeMap::from([(
    "Example Lab Report.v1".to_owned(),
    Declared::Codes(vec!["Laboratory-Reports".to_owned()]),
)]);
let map = CategoryMap::declare(&[], &templates, &BTreeMap::new())?;
let read = RootObject {
    template_id: Some("Example Lab Report.v1".to_owned()),
    ..RootObject::default()
};
let classified = map.classify(&Evidence::reached(Basis::Returned, vec![read]));
assert!(classified.categories().contains_key(&Category::MedicalTestResult));
assert!(classified.unclassified().is_none());
# Ok::<(), ehds_logging::map::MapError>(())
```

CI runs this example as a doctest of `ehds-logging`. The crate READMEs show
the rest: the syslog connection, the spool, the FHIR Feed and ITI-81 in
[`ihe-atna`](crates/ihe-atna/README.md), and the record, retention and the
category codes on the wire in [`ehds-logging`](crates/ehds-logging/README.md).

## Security

- A record names a patient toward its repository and nowhere else: the
  participant objects are held as secrets, and no `Debug` rendering shows a
  value.
- Every value is escaped before it reaches the XML or JSON of a record, so
  no control or bidirectional character gets through.
- A record the spool cannot take is refused at once, never dropped
  silently.
- The syslog connection speaks TLS 1.2 or 1.3 alone, over the rustls
  configuration and trust roots you hand it.
- The crates contain no `unsafe` code (`unsafe_code = "forbid"`), and
  `cargo-deny` checks every dependency on every pull request.
- Every release is signed and carries SLSA build provenance
  ([`docs/release.md`](docs/release.md)).
- Report a vulnerability privately as the [security policy](SECURITY.md)
  describes.

## Versioning

Both crates follow [Cargo's SemVer rules](https://doc.rust-lang.org/cargo/reference/semver.html)
on one version line, released by the tag `vX.Y.Z`, with one
[changelog](CHANGELOG.md). On a `0.x` line a public API break moves the
minor version. Every pull request runs
[`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks)
against the newest release on crates.io and refuses a break the version
does not allow; `ehds-logging` lists the public dependencies its API
exposes under "API stability" in its README.

## Documentation

- The API reference on docs.rs: [`ihe-atna`](https://docs.rs/ihe-atna) and
  [`ehds-logging`](https://docs.rs/ehds-logging).
- The [architecture](docs/architecture.md) and the
  [pinned specifications](docs/VERSIONS.md).
- The [release process](docs/release.md).
- The [FerroHEALTH book](https://ferrohealth.eu/docs/).

## Support

[SUPPORT.md](SUPPORT.md) says where to ask a question and how to report a
defect. Read [CONTRIBUTING.md](CONTRIBUTING.md) and the
[code of conduct](CODE_OF_CONDUCT.md) before you open a pull request;
[GOVERNANCE.md](GOVERNANCE.md) and [MAINTAINERS.md](MAINTAINERS.md) say who
decides and who reviews.

## Licence

Business Source License 1.1 ([LICENSE](LICENSE), [NOTICE](NOTICE)), with
Apache License 2.0 as the Change License four years after each version.
Non-production use and non-commercial production use are free; the
[licensing page](https://ferrohealth.eu/docs/licensing.html) says when you
need a commercial licence from Cadasto B.V.
