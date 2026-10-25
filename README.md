<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->
<div align="center">

<img src="assets/brand/ferroaudit-lockup-auto.svg" alt="FerroAUDIT" width="290">

**The audit trail of the FerroHEALTH products: who looked at which patient's data, recorded the standard way.**

[Documentation](https://docs.rs/ihe-atna) ·
[Quick start](#quick-start) ·
[What ships today](#what-ships-today) ·
[Licensing](https://ferrohealth.eu/docs/licensing.html)

**Project**<br>
[![Release](https://img.shields.io/github/v/release/FerroHEALTH/FerroAUDIT?sort=semver&logo=github&label=release)](https://github.com/FerroHEALTH/FerroAUDIT/releases/latest)
[![CI](https://img.shields.io/github/actions/workflow/status/FerroHEALTH/FerroAUDIT/ci.yml?branch=main&logo=githubactions&logoColor=white&label=CI)](https://github.com/FerroHEALTH/FerroAUDIT/actions/workflows/ci.yml)
[![License: BUSL-1.1](https://img.shields.io/badge/License-BUSL--1.1-blue)](LICENSE)
[![Docs](https://img.shields.io/docsrs/ihe-atna?logo=docsdotrs&label=docs.rs)](https://docs.rs/ihe-atna)

**Quality and security**<br>
[![OpenSSF Scorecard](https://api.securityscorecards.dev/projects/github.com/FerroHEALTH/FerroAUDIT/badge)](https://scorecard.dev/viewer/?uri=github.com/FerroHEALTH/FerroAUDIT)
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

## Quick start

Add the crates to a Rust project:

```sh
cargo add ihe-atna@=0.2.9 --features balp
cargo add ehds-logging@=0.2.9 --features balp
```

Pin the versions exactly. The crate documentation on
[docs.rs](https://docs.rs/ihe-atna) is the integration guide, and
[`docs/architecture.md`](docs/architecture.md) has the design.

## What ships today

- [ATNA audit messages (ITI-20)](crates/ihe-atna/README.md#atna-atna): the DICOM audit message over syslog with TLS, to any Audit Record Repository
- [BALP audit events](crates/ihe-atna/README.md#balp-balp): the FHIR `AuditEvent` for every IHE FHIR transaction, posted over the FHIR feed
- [A store-and-forward queue](crates/ihe-atna/README.md#atna-atna) that keeps every record until the repository takes it, oldest first
- [Retrieving audit events (ITI-81)](crates/ihe-atna/README.md#retrieve-retrieve): the FHIR search a product serves over its own audit store
- [The EHDS access record](crates/ehds-logging/README.md), as Annex II 3.2 of Regulation (EU) 2025/327 asks, with its data categories and retention

<details>
<summary><b>Published crates</b> (2)</summary>

| Crate | What it does | Version | Docs | Downloads |
|---|---|---|---|---|
| [`ihe-atna`](https://crates.io/crates/ihe-atna) | The IHE audit trail: ATNA (ITI-20) over syslog and TLS, BALP AuditEvents over the FHIR feed, the spool and forwarder, and ITI-81 retrieval | [![crates.io](https://img.shields.io/crates/v/ihe-atna?logo=rust&label=crates.io)](https://crates.io/crates/ihe-atna) | [![docs.rs](https://img.shields.io/docsrs/ihe-atna?logo=docsdotrs&label=docs.rs)](https://docs.rs/ihe-atna) | [![downloads](https://img.shields.io/crates/d/ihe-atna?logo=rust&label=downloads)](https://crates.io/crates/ihe-atna) |
| [`ehds-logging`](https://crates.io/crates/ehds-logging) | The EHDS logging component: the Annex II 3.2 access record, its categories and retention, written as a BALP AuditEvent | [![crates.io](https://img.shields.io/crates/v/ehds-logging?logo=rust&label=crates.io)](https://crates.io/crates/ehds-logging) | [![docs.rs](https://img.shields.io/docsrs/ehds-logging?logo=docsdotrs&label=docs.rs)](https://docs.rs/ehds-logging) | [![downloads](https://img.shields.io/crates/d/ehds-logging?logo=rust&label=downloads)](https://crates.io/crates/ehds-logging) |

</details>

## Learn more

- [`ihe-atna`](https://docs.rs/ihe-atna) and [`ehds-logging`](https://docs.rs/ehds-logging) on docs.rs, and the [FerroHEALTH book](https://ferrohealth.eu/docs/)
- [Architecture](docs/architecture.md) and the [pinned specifications](docs/VERSIONS.md)
- [Changelog](CHANGELOG.md) and [milestones](https://github.com/FerroHEALTH/FerroAUDIT/milestones)
- [Security policy](SECURITY.md)
- [Contributing](CONTRIBUTING.md), [code of conduct](CODE_OF_CONDUCT.md), [support](SUPPORT.md), [governance](GOVERNANCE.md) and [maintainers](MAINTAINERS.md)

## Licence

Business Source License 1.1 ([LICENSE](LICENSE)), with Apache License 2.0 as
the Change License four years after each version; the
[licensing page](https://ferrohealth.eu/docs/licensing.html) says when you
need a commercial licence from Cadasto B.V.
