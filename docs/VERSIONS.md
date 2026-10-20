<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Pinned version matrix

This file is the single source of truth for every version pin in FerroAUDIT.
When it and a file that repeats a pin disagree, that is drift. Fix the
disagreement; never let either side silently win. `scripts/checks/versions.sh`
reads every vendored `PROVENANCE.md` and every vendored file back against the
rows below.

No specification governs this file; it is FerroAUDIT's own design.

## Specifications

The ground for each pin is the pin table in `docs/architecture.md` §2, which
records why the value is what it is.

| Item | Pin | Repeated in |
|---|---|---|
| IHE ITI-20 | Record Audit Event, ITI TF-2 §3.20, Revision 20.2 Final Text | `docs/architecture.md` §2, `docs/specs/ihe-atna/PROVENANCE.md` |
| RESTful ATNA | the IHE ITI supplement *Add RESTful ATNA (Query and Feed)*, Rev. 3.6 Trial Implementation | `docs/architecture.md` §2, `docs/specs/ihe-atna/PROVENANCE.md` |
| BALP | the IHE Basic Audit Log Patterns 1.1.4, on FHIR R4 4.0.1 | `docs/architecture.md` §2, `docs/specs/ihe-balp/PROVENANCE.md`, `docs/specs/ihe-balp-pages/PROVENANCE.md` |
| FHIR | R4 (4.0.1), through the `fhir-types` crate's `r4` module | `docs/architecture.md` §2, `Cargo.toml` |
| EHDS Regulation | Regulation (EU) 2025/327 as published (OJ L, 2025/327, 5.3.2025), with Implementing Regulation (EU) 2026/2099 | `docs/architecture.md` §2, `docs/specs/eu-ehds/PROVENANCE.md` |

## Corpora and machine-readable inputs

A corpus is pinned by version and checksum, never by a moving tag or a
`latest` URL, and vendored by a committed `scripts/vendor/*.sh` with a
`PROVENANCE.md`. Each script reads its pin from this table, and
`scripts/checks/versions.sh` reads each vendored `PROVENANCE.md` back and
hashes the vendored bytes again, failing when either disagrees with its row.
The pins are the ones FerroHEALTH/FerroFED vendored these corpora at, so the
two trees hold the same bytes; the `eu-ehds` corpus here holds five of the
documents FerroFED's holds, so its pin-set digest is its own.

| Item | Pin | Repeated in |
|---|---|---|
| IHE BALP FHIR package | `ihe.iti.balp` version `1.1.4` from `packages.fhir.org`, tarball sha256 `be46dda3088ee9d486d7163458dc5a4d7192e8a1c587fd47e2beb70e4bcefa90`, the whole package but its manifest | `scripts/vendor/ihe-balp.sh`, `docs/specs/ihe-balp/PROVENANCE.md` |
| IHE BALP narrative pages | the IG of package `ihe.iti.balp` 1.1.4 at `profiles.ihe.net/ITI/BALP/1.1.4/`, pin-set digest `f682913c6c5d5d2599c1dcb3c41a22373240b126a2051dab6b58ca8d56d1524d`, the Volume 1 (1:52) and Volume 3 (3:5.7) pages | `scripts/vendor/ihe-balp-pages.sh`, `docs/specs/ihe-balp-pages/PROVENANCE.md` |
| IHE ITI-20 Record Audit Event | `profiles.ihe.net/ITI/TF/Volume2/ITI-20.html`, Revision 20.2, page sha256 `881c7d6423fdf5ecaf4f9f50f8d25be61c3ed8ef97c87eff9591bd7fdf51570d` and its figure `media/Figure_3.20.4-1.png` sha256 `7aba1a2437e3492202460e150a6dda85b8a1886035bd2aa8daa4c892a579b734` | `scripts/vendor/ihe-atna.sh`, `docs/specs/ihe-atna/PROVENANCE.md` |
| IHE RESTful ATNA supplement | `IHE_ITI_Suppl_RESTful-ATNA.pdf` from `www.ihe.net`, Rev. 3.6, sha256 `d8451a4a0d951662b6a04b745084c33afff6196db5647f2cf79d9149dfa7265a` | `scripts/vendor/ihe-atna.sh`, `docs/specs/ihe-atna/PROVENANCE.md` |
| EU EHDS Regulation | Regulation (EU) 2025/327, Implementing Regulation (EU) 2026/2099, the eHealth Network Patient Summary and General Guidelines and Commission Decision 2011/833/EU on the reuse of Commission documents, each at the Cellar manifestation and sha256 FerroFED's `eu-ehds` corpus pins, pin-set digest `437682c4109f855d475ed052b80a4540b8d6f805da05f5b4130107445a76e1e8` | `scripts/vendor/eu-ehds.sh`, `docs/specs/eu-ehds/PROVENANCE.md` |
| HL7 Europe EU Health Data API | package `hl7.fhir.eu.health-data-api` 1.0.0-ballot, pin-set digest `253ebbce321fc294ae582c3c3abb67274ad53de6578c3eb3360e1566697691ac`, tarball sha256 `5204af60bf7f74f301a627e641a34683d392014f53d5df8ba3651c964e92977b`, whose `EEHRxFDocumentPriorityCategoryCS` codes the priority categories the access record writes | `scripts/vendor/ehds-categories.sh`, `docs/specs/eu-hl7-health-data-api/PROVENANCE.md`, `crates/ehds-logging/src/category.rs` |
| HL7 Europe Imaging Report | package `hl7.fhir.eu.imaging` 1.0.0-ballot, pin-set digest `aad98731a1f2597d6a7defa2c7f6ab7297781b677398383346c5592f913c9f4b`, tarball sha256 `bb6012bd7a019ac82f5e939dd6e892e6db6b15fd2931dcf7cc445c1ad92c3d70` | `scripts/vendor/ehds-categories.sh`, `docs/specs/eu-hl7-imaging/PROVENANCE.md` |
| HL7 Europe Hospital Discharge Report | package `hl7.fhir.eu.hdr` 0.1.0-ballot, pin-set digest `2bc6073aedf1a6e4630b3763c2058ed5bbec53c29bc2e74ef5a9c2a60f0af918`, tarball sha256 `458b8ba0edcf22d75d455d6149c9264d486bb8ec052251a90afd8654410482ee` | `scripts/vendor/ehds-categories.sh`, `docs/specs/eu-hl7-hdr/PROVENANCE.md` |
| MyHealth@EU Master Value Sets Catalogue | package `myhealth.eu.fhir.mvc-package` 9.1.0, pin-set digest `06f68a58f7382db396f029b1c0a420fcac6cbf53d8a948df47594937b0886749`, tarball sha256 `3f99350f36a2a1ea17b7fde37c39ff766bd5c3bc9a29c6183af1acfd07b78ce1` | `scripts/vendor/ehds-categories.sh`, `docs/specs/ehdsi-mvc/PROVENANCE.md` |
| IETF syslog and audit RFCs | RFC 3881, RFC 5424, RFC 5425, RFC 5426 and RFC 6750 from `www.rfc-editor.org`, each by its sha256, pin-set digest `9de9544040398351d75871f4074a2bde41a21e6b14aafe798678addb2d500bab` | `scripts/vendor/ietf-syslog.sh`, `docs/specs/ietf-syslog/PROVENANCE.md` |
| DICOM PS3.15 | release 2026c, `part15.html` from `dicom.nema.org`, cache only (not redistributed), pin-set digest `9a9b6eb4218785a873402cc045eff8aa16c05aa89a6df5909628301dc7a69ac7` | `scripts/vendor/dicom-ps3-15.sh`, `docs/specs/dicom-ps3-15/PROVENANCE.md` |
| openEHR BASE specification source | `openEHR/specifications-BASE` tag `Release-1.1.0`, the release FerroHEALTH/FerroFED vendors: the Base Types specification and the licence | `scripts/vendor/openehr-base.sh`, `docs/specs/openehr-base/PROVENANCE.md` |

## Licence

| Item | Pin | Repeated in |
|---|---|---|
| Project licence | BUSL-1.1 | `LICENSE`, `NOTICE`, the SPDX header of every first-party file, the `license` field of the workspace manifest |

`LICENSE` names Apache License 2.0 as a licence of its own, as the Change
License four years after each version. `ihe-atna` and `ehds-logging` are
hand-written code that implements the specifications above, so both are
under BUSL-1.1 like the rest of the family's own code; only generator-emitted code is published under
Apache-2.0 elsewhere in the family. `scripts/checks/versions.sh` fails on an
Apache-2.0 or MIT claim in a first-party file.

Vendored material keeps its upstream terms, recorded beside each vendored tree
in its `PROVENANCE.md`: CC BY 4.0 for the BALP package and pages, the IHE
Technical Frameworks General Introduction §9 licence for the ITI-20 page and
the RESTful ATNA supplement, CC0-1.0 for the HL7 Europe and MyHealth@EU
packages, and Commission Decision 2011/833/EU for the EU acts.

## The toolchain and the crate

| Item | Pin | Repeated in |
|---|---|---|
| Rust toolchain | 1.98.1, profile `minimal`, components `rustfmt` and `clippy` | `rust-toolchain.toml` |
| Rust edition | 2024 | `Cargo.toml` `[workspace.package] edition`, `rustfmt.toml` |
| MSRV | 1.97, the FerroHEALTH family floor, checked on every pull request by the `msrv` job with no feature and with `atna`; `balp` takes `fhir-types`, whose own `rust-version` is 1.98 | `Cargo.toml` `[workspace.package] rust-version`, `.github/workflows/ci.yml` (reads it from the manifest) |
| `ihe-atna`, `ehds-logging` | 0.1.1, the workspace version both crates inherit, the release tag `v0.1.1` and the `CHANGELOG.md` section | `Cargo.toml` `[workspace.package] version`, `Cargo.lock`, `CHANGELOG.md` (held together by `scripts/checks/crate-version-guard.sh`) |
| Third-party crates | the `[workspace.dependencies]` table, each version checked against crates.io on 2026-10-20 | `Cargo.toml`, `Cargo.lock` |

## The CI tools

| Item | Pin | Repeated in |
|---|---|---|
| zizmor | 1.31.1 | `.github/workflows/ci.yml` |
| actionlint | 1.7.12, image digest `sha256:b1934ee5f1c509618f2508e6eb47ee0d3520686341fec936f3b79331f9315667` | `.github/workflows/ci.yml` |
| shellcheck | 0.11.0 | `.github/workflows/ci.yml` |
| cargo-nextest, cargo-deny | the newest release `taiki-e/install-action` resolves at run time | `.github/workflows/ci.yml` |
| GitHub Actions | every `uses:` pinned to a full commit SHA with its version in a trailing comment, bumped by Dependabot | `.github/workflows/*.yml`, `.github/actions/setup-rust/action.yml` |
