<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Vendored inputs: fetched by a committed script, verbatim, with provenance

FerroAUDIT vendors the specifications it implements, and the texts they
cite, under `docs/specs/`. Each tree carries a `PROVENANCE.md` that records
its licence:

- `ihe-atna`: the ITI-20 page with its figure and the RESTful ATNA
  supplement, under the licence of the IHE Technical Frameworks General
  Introduction §9.
- `ihe-balp`, `ihe-balp-pages`: the BALP 1.1.4 package and its Volume 1 and
  Volume 3 pages, CC BY 4.0.
- `eu-ehds`: Regulation (EU) 2025/327, Implementing Regulations (EU)
  2026/2083 and 2026/2099, the eHealth Network Patient Summary and General
  Guidelines, and Commission Decision 2011/833/EU, reusable under that
  Decision (the guidelines also under CC BY 4.0, by the Commission legal
  notice).
- `eu-gdpr`: Regulation (EU) 2016/679 and its 2018 English corrigendum,
  reusable under Commission Decision 2011/833/EU.
- `nl-law`: the Wabvpz and the UAVG, free of copyright under Auteurswet
  Art. 11.
- `eu-hl7-health-data-api`, `eu-hl7-imaging`, `eu-hl7-hdr`, `ehdsi-mvc`: the
  HL7 Europe and MyHealth@EU packages, CC0-1.0.
- `ietf-syslog` (RFC 3881, 5424, 5425, 5426 and 6750) and `ietf-oauth` (RFC
  8693): under the IETF Trust Legal Provisions (BCP 78) §3.c.
- `ehdsi-specifications`: the eHDSI Audit Trail Profile 10.1.0, SAML Profile
  10.0.0, Section I Security Policies 10.0.0, NCPeH Architecture and
  Components Specifications 10.0.0 and OpenNCP Implementation 10.0.0,
  supplied by the owner, whose cover pages read "Reuse is authorised,
  provided the source is acknowledged" at distribution level PU (Public).
- `openehr-base`: the openEHR BASE Base Types specification source, CC
  BY-SA 3.0 Unported.
- `dicom-ps3-15`: the `PROVENANCE.md` alone. NEMA grants no licence to
  redistribute DICOM PS3.15, so the page is read from the ignored
  `.vendor-cache/`.

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
  material keeps its upstream terms, the licences listed above.
- **A source behind a login** whose terms allow redistribution (the eHDSI
  specifications behind EU Login) is `supplied`: the owner places the file
  in a local directory, the script finds it by its pinned sha256 and
  commits it verbatim, downloading nothing, and its `--check` holds the
  committed bytes to the pins. A source whose terms do not allow
  redistribution, or are unclear, is `cache` (fetched into the ignored
  `.vendor-cache/`) or `withheld` (its sha256 recorded, copied nowhere).
- **A new corpus** is a row in `docs/VERSIONS.md`, a fetch by the
  script and the citation that needs it, in one change.
- **Bumping a pin** is a deliberate change: rerun the script, read what
  changed against the citations in `docs/architecture.md` and the code, and
  record the new version in `PROVENANCE.md` and `docs/VERSIONS.md`.
- **No patient data and no credential** ever enters the tree under any
  name.
