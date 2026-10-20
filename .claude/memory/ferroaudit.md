---
name: ferroaudit
description: "FerroAUDIT is the shared audit library of the FerroHEALTH products: ihe-atna (the ATNA ITI-20 sender and BALP over the ATX: FHIR Feed, moved from FerroFED's ihe-iti) and ehds-logging (the EHDS logging component); every product sends audit through it, FerroSYS builds the Audit Record Repository on it; owner 2026-10-20"
metadata:
  type: project
---

<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

FerroAUDIT (`ihe-atna` on crates.io, FerroHEALTH/FerroAUDIT) is the shared
IHE audit crate of the FerroHEALTH products, the way FerroSMART is the shared
SMART crate (owner, 2026-10-20, recorded on FerroHEALTH/FerroFED#367).

- **What it is:** the sending half of ITI-20 Record Audit Event, the DICOM
  audit message over syslog with TLS, the store-and-forward spool and the
  forwarder (feature `atna`), and the BALP `AuditEvent` sent over the ATX:
  FHIR Feed (feature `balp`).
- **Who uses it:** FerroFED's mandatory access log (its A58, FerroHEALTH/FerroFED#697) and
  its `ehds-logging` sink, FerroEHR's `[audit.fhir_feed]`, FerroPIX's
  Manager-side BALP records and the profile audit writers of FerroPIX's
  `ihe-iti`. FerroSYS, the control plane, later builds the receiving Audit
  Record Repository on this crate, as a feature of it.
- **`ehds-logging`** (owner decision 2026-10-20): the EHDS Regulation
  (EU) 2025/327 logging component moved here from FerroFED (0.0.26 there) as
  the second crate on the same version line, its `balp` feature on
  `ihe-atna`. FerroEHR, which implemented the same law in its server, adopts
  it in its own issue.
- **No product reimplements audit again.** A behaviour a product needs is
  filed here and built here.
- **Where it came from:** FerroFED's `ihe-iti` features `atna` and `balp`,
  moved with their history (git filter-repo, FerroFED commit
  `8cefc8f78bd7814bd42dbadd1b1053794ae6e3e4`) and their tests.

**How to apply:** build every behaviour to `docs/architecture.md`; a change
that would name a product in the library or drop a record is refused and
goes back to the owner.
