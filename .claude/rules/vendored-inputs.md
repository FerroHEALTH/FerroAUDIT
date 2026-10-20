<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Vendored inputs: fetched by a committed script, verbatim, with provenance

FerroAUDIT vendors the specifications it implements under `docs/specs/`: the
ITI-20 page with its figure and the RESTful ATNA supplement, the BALP 1.1.4
package and its narrative pages, each tree with a `PROVENANCE.md`.

## Rules

- **Every vendored file is fetched by a committed script** under
  `scripts/vendor/` from the pin `docs/VERSIONS.md` records (a commit, an
  immutable publication or a checksum, never a moving tag or a `latest`
  URL), never copied by hand.
- **Vendored files are verbatim**: never edited. `scripts/checks/versions.sh`
  hashes every vendored byte back against its row and its `PROVENANCE.md`,
  so a hand-edited file, a moved pin or a stale provenance fails the pull
  request.
- **Every vendored tree carries a `PROVENANCE.md`**: the source, the pinned
  version, the date fetched, the licence, and how to refresh it. Vendored
  material keeps its upstream terms (CC BY 4.0 for the BALP package and
  pages, the IHE General Introduction §9 licence for the ITI-20 page and the
  supplement).
- **A new corpus** is a row in `docs/VERSIONS.md`, a fetch by the
  script and the citation that needs it, in one change.
- **Bumping a pin** is a deliberate change: rerun the script, read what
  changed against the citations in `docs/architecture.md` and the code, and
  record the new version in `PROVENANCE.md` and `docs/VERSIONS.md`.
- **No patient data and no credential** ever enters the tree under any
  name.
