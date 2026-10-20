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

## Corpora and machine-readable inputs

A corpus is pinned by version and checksum, never by a moving tag or a
`latest` URL, and vendored by a committed `scripts/vendor/*.sh` with a
`PROVENANCE.md`. Each script reads its pin from this table, and
`scripts/checks/versions.sh` reads each vendored `PROVENANCE.md` back and
hashes the vendored bytes again, failing when either disagrees with its row.
The pins are the ones FerroHEALTH/FerroFED vendored these corpora at, so the
two trees hold the same bytes.

| Item | Pin | Repeated in |
|---|---|---|
| IHE BALP FHIR package | `ihe.iti.balp` version `1.1.4` from `packages.fhir.org`, tarball sha256 `be46dda3088ee9d486d7163458dc5a4d7192e8a1c587fd47e2beb70e4bcefa90`, the whole package but its manifest | `scripts/vendor/ihe-balp.sh`, `docs/specs/ihe-balp/PROVENANCE.md` |
| IHE BALP narrative pages | the IG of package `ihe.iti.balp` 1.1.4 at `profiles.ihe.net/ITI/BALP/1.1.4/`, pin-set digest `f682913c6c5d5d2599c1dcb3c41a22373240b126a2051dab6b58ca8d56d1524d`, the Volume 1 (1:52) and Volume 3 (3:5.7) pages | `scripts/vendor/ihe-balp-pages.sh`, `docs/specs/ihe-balp-pages/PROVENANCE.md` |
| IHE ITI-20 Record Audit Event | `profiles.ihe.net/ITI/TF/Volume2/ITI-20.html`, Revision 20.2, page sha256 `881c7d6423fdf5ecaf4f9f50f8d25be61c3ed8ef97c87eff9591bd7fdf51570d` and its figure `media/Figure_3.20.4-1.png` sha256 `7aba1a2437e3492202460e150a6dda85b8a1886035bd2aa8daa4c892a579b734` | `scripts/vendor/ihe-atna.sh`, `docs/specs/ihe-atna/PROVENANCE.md` |
| IHE RESTful ATNA supplement | `IHE_ITI_Suppl_RESTful-ATNA.pdf` from `www.ihe.net`, Rev. 3.6, sha256 `d8451a4a0d951662b6a04b745084c33afff6196db5647f2cf79d9149dfa7265a` | `scripts/vendor/ihe-atna.sh`, `docs/specs/ihe-atna/PROVENANCE.md` |

## Licence

| Item | Pin | Repeated in |
|---|---|---|
| Project licence | BUSL-1.1 | `LICENSE`, `NOTICE`, the SPDX header of every first-party file, the `license` field of the workspace manifest |

`LICENSE` names Apache License 2.0 as a licence of its own, as the Change
License four years after each version. `ihe-atna` is hand-written code that
implements the specifications above, so it is under BUSL-1.1 like the rest of
the family's own code; only generator-emitted code is published under
Apache-2.0 elsewhere in the family. `scripts/checks/versions.sh` fails on an
Apache-2.0 or MIT claim in a first-party file.

Vendored material keeps its upstream terms, recorded beside each vendored tree
in its `PROVENANCE.md`: CC BY 4.0 for the BALP package and pages, and the IHE
Technical Frameworks General Introduction §9 licence for the ITI-20 page and
the RESTful ATNA supplement.

## The toolchain and the crate

| Item | Pin | Repeated in |
|---|---|---|
| Rust toolchain | 1.98.1, profile `minimal`, components `rustfmt` and `clippy` | `rust-toolchain.toml` |
| Rust edition | 2024 | `Cargo.toml` `[workspace.package] edition`, `rustfmt.toml` |
| MSRV | 1.98 | `Cargo.toml` `[workspace.package] rust-version` |
| `ihe-atna` | 0.1.0, the workspace version, the release tag `v0.1.0` and the `CHANGELOG.md` section | `Cargo.toml` `[workspace.package] version`, `Cargo.lock`, `CHANGELOG.md` (held together by `scripts/checks/crate-version-guard.sh`) |
| Third-party crates | the `[workspace.dependencies]` table, each version checked against crates.io on 2026-10-20 | `Cargo.toml`, `Cargo.lock` |

## The CI tools

| Item | Pin | Repeated in |
|---|---|---|
| zizmor | 1.31.1 | `.github/workflows/ci.yml` |
| actionlint | 1.7.12, image digest `sha256:b1934ee5f1c509618f2508e6eb47ee0d3520686341fec936f3b79331f9315667` | `.github/workflows/ci.yml` |
| shellcheck | 0.11.0 | `.github/workflows/ci.yml` |
| cargo-nextest, cargo-deny | the newest release `taiki-e/install-action` resolves at run time | `.github/workflows/ci.yml` |
| GitHub Actions | every `uses:` pinned to a full commit SHA with its version in a trailing comment, bumped by Dependabot | `.github/workflows/*.yml`, `.github/actions/setup-rust/action.yml` |
