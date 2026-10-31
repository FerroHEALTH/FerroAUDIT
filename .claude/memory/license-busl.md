---
name: license-busl
description: "FerroAUDIT is under the Business Source License 1.1 on the FerroHEALTH terms, Cadasto B.V. the Licensor, Apache 2.0 the Change License four years after each version"
metadata:
  type: project
---

<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

FerroAUDIT's own code and text are under the Business Source License 1.1,
the terms FerroBRIDGE, FerroEHR, FerroTERM and FerroUI use. Cadasto B.V. is
the Licensor and handles the business side (info@cadasto.com); the
maintainer handles the technical side.

The Additional Use Grant names recording, sending or storing audit records
of access to health data through the Licensed Work as the hosted-service
use that needs a commercial licence; the wording is the owner's to keep or
change.

**How to apply:**

- Every first-party file carries `SPDX-FileCopyrightText: Cadasto B.V.` and
  `SPDX-License-Identifier: BUSL-1.1` in its header;
  `scripts/checks/versions.sh` fails on any other claim.
- `LICENSE` is the one file naming Apache 2.0 as a licence of its own, as the
  Change License.
- Vendored material keeps its upstream terms (CC BY 4.0 for the BALP
  package and pages, the IHE General Introduction §9 licence for the ITI-20
  page and the RESTful ATNA supplement), recorded in each `PROVENANCE.md`;
  `deny.toml` admits the dependency licences.
- A contribution is licensed under the same terms (CONTRIBUTING.md §
  Licensing of contributions); opening the pull request accepts them.
