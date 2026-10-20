---
name: crate-name
description: "The crate is ihe-atna (Rust path ihe_atna), named for the specification it implements and never a product, under BUSL-1.1 because it is hand-written code implementing a specification; owner 2026-10-20"
metadata:
  type: project
---

<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

The crate is `ihe-atna` (Rust path `ihe_atna`). The repository is
`FerroHEALTH/FerroAUDIT`, and FerroAUDIT is the name in prose.

**Why:** the family's published-crate naming rule: a crate takes the name of
the specification it implements, never a product's (FerroFED
`.claude/memory/published-crate-naming.md`). ATNA is the profile whose
transaction it carries; BALP is a feature, since its records travel over
ATNA's FHIR Feed. The orchestrator reserved `ihe-atna` at `0.0.0` on
crates.io on 2026-10-20, and the owner configured Trusted Publishing for
FerroHEALTH/FerroAUDIT (`release.yml` and `publish-crates.yml`, environment
`crates-io`) the same day, so Trusted Publishing takes over from `0.1.0`.

**The licence is BUSL-1.1**, not the Apache-2.0 of the family's generated
crates: hand-written code that implements a specification is BUSL, and only
generator-emitted code (`fhir-types`, `hl7v2-types` in FerroBRIDGE) is
Apache.

**How to apply:** never rename the crate or the package, and never run
`cargo publish` under another name or from outside the release lane.
Products depend on `ihe-atna = "=X.Y.Z"`.
