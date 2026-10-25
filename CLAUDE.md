<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# CLAUDE.md

**FerroAUDIT** is the shared IHE audit library of the FerroHEALTH products:
the ATNA Record Audit Event sender (ITI-20) and the Basic Audit Log Patterns
(BALP) `AuditEvent` sent over ITI-20's ATX: FHIR Feed. It is one Cargo
workspace with two library crates on one version line, both published to
crates.io: `ihe-atna` (Rust path `ihe_atna`) and `ehds-logging`, the EHDS
logging component (Regulation (EU) 2025/327), whose access record travels as
a BALP `AuditEvent` through `ihe-atna`. FerroSMART is the family's shared
SMART crate in the same way.

FerroAUDIT in prose, `ihe-atna` in identifiers. The repository, the crate
name and its licence are the owner's decision of 2026-10-20
(FerroHEALTH/FerroFED#367): the crate takes its specification's name, never
a product's, and it is BUSL-1.1 because it is hand-written code that
implements a specification (`.claude/memory/crate-name.md`,
`.claude/memory/license-busl.md`).

## What the library is

Every product sends its audit trail through it, and no product implements
audit again: FerroFED's mandatory access log, FerroEHR's
`[audit.fhir_feed]`, FerroPIX's Manager-side BALP records and the profile
audit records of FerroPIX's `ihe-iti`. `ihe-atna` carries the whole audit
trail, sending, forwarding and retrieving, and `ehds-logging` the EHDS
access record (`.claude/memory/family-architecture.md`). The code came from
`ihe-iti` (features `atna` and `balp`) while that crate lived in FerroFED,
with its history and its tests.

- `atna` (feature): the DICOM PS3.15 audit message (`atna::message`), the
  RFC 5424 syslog message in its RFC 5425 frame (`atna::syslog`), the TLS
  connection to an Audit Record Repository (`atna::repository`), the
  bounded store-and-forward spool ITI-20 requires of a sender
  (`atna::spool`), and the forwarder that stores first and delivers oldest
  first (`atna::forwarder`).
- `balp` (feature): the FHIR R4 `AuditEvent` an IHE FHIR transaction is
  audited as (`balp`), the `AuditRecorder` seam an audited client records
  through, and the ATX: FHIR Feed client (`atna::feed`), over the same spool
  and forwarder, without the syslog sender or its TLS stack (owner decision
  2026-10-20, FerroHEALTH/FerroAUDIT#11).
- `user` (always): whom an audited exchange is made for, a user an OAuth
  2.0 access token names or the system.
- `ehds-logging`: the Annex II 3.2 access record, the Article 14(1)
  categories and the map that classifies an access, the Article 9(2)
  retention, the security labels, and (feature `balp`) the record as a BALP
  `AuditEvent` with a sink over an `ihe-atna` recorder. It moved from FerroFED
  (0.0.26 there); FerroEHR adopts it in place of its own implementation.

## The design is recorded in `docs/architecture.md`

Read it first: the role, the pinned versions, each layer with its
citations, the features and their boundaries, how a product integrates, the
naming and the licence. The vendored specifications under `docs/specs/` are
the oracle, pinned in `docs/VERSIONS.md` and held to their provenance by
`scripts/checks/versions.sh`.

## Repo map

- `crates/ihe-atna`: the library. `src/` (`atna` with `message`, `syslog`,
  `repository`, `spool`, `forwarder` and `feed`; `balp` with its writer;
  `user`; the private `redact` and `search`), `tests/it/` (one binary: the
  spool and the forwarder against local listeners, the FHIR Feed against a
  `wiremock` repository, the records against the vendored BALP corpus, the
  feature boundaries; every patient identifier synthetic).
- `crates/ehds-logging`: the EHDS logging component. `src/` (`record`,
  `category`, `map`, `classify`, `retention`, `label`, `emergency`,
  `transformation`, `sink`, and `balp` behind its feature), `tests/it/`.
- `docs/`: `architecture.md`, `VERSIONS.md`, `specs/` (the vendored ITI-20
  page and RESTful ATNA supplement, the BALP package and pages, the EHDS
  Regulation with the acts and guidelines `ehds-logging` cites, the HL7
  Europe and MyHealth@EU packages the categories are held to, the IETF
  syslog and audit RFCs, the openEHR BASE Base Types specification, and the
  pin of DICOM PS3.15, which is read from the git-ignored `.vendor-cache/`
  and never committed, each with a `PROVENANCE.md`).
- `scripts/`: `vendor/` (the corpus fetchers), `checks/` (the committed
  guards), `release/` (the crate publish and the milestone check), `gh/`
  (the tracker helpers).
- `.github/`: CI with a single required `conclusion` job, the vendored
  specification guard, CodeQL, Scorecard, Dependabot, the release lane with
  crates.io Trusted Publishing (`release.yml`, `release-crate.yml`,
  `publish-crates.yml`).
- `.claude/`: the working discipline (rules, agents, skills, hooks, memory).

## IMPORTANT hard rules

- **The specifications are the authority**: ITI-20 Record Audit Event (ITI
  TF-2 §3.20, Revision 20.2), the RESTful ATNA supplement (Rev. 3.6) and BALP
  1.1.4, and for `ehds-logging` Regulation (EU) 2025/327, as vendored under
  `docs/specs/`, with the DICOM PS3.15, RFC 5424, RFC 5425 and FHIR R4 texts
  they cite. Cite the section in every
  `// NOTE:`; never resolve a question from memory or from another
  implementation's behaviour. Where none governs a decision, say so: "no
  specification governs this: our own design"
  (`.claude/rules/spec-adherence.md`).
- **A record names a patient toward its repository and nowhere else.** No
  `Debug`, log line or error carries an identifier, a patient reference, a
  search, a user's values or a credential. Every such value is a secret
  type.
- **No record is dropped silently.** A record the spool cannot take is
  refused at once, so its caller fails the exchange it audits.
- **Never send the user's email address to any service**, a `User-Agent`
  header included.
- **Comments follow RFC 505 and RFC 1574 with hard budgets**
  (`.claude/rules/comments.md`); prose follows
  `.claude/rules/writing-style.md`.
- **Branches use conventional types** (`feat/`, `fix/`, `chore/`, `docs/`,
  `refactor/`, `perf/`, `test/`, `ci/`, `build/`, `release/`); every commit is
  signed; `main` takes changes only through pull requests and the merge
  queue.
- **NEVER add AI or Claude attribution** to a commit, PR, issue, comment, or
  code comment. A `PreToolUse` hook blocks a commit or PR command carrying
  one.
- **Never weaken, skip, or delete a test** (`.claude/rules/testing.md`).
- **Keep the changelog** (`CHANGELOG.md`, Keep a Changelog 1.1.0).

## Licence

Business Source License 1.1 (`LICENSE`, `NOTICE`), Licensor Cadasto B.V.,
Change License Apache 2.0 four years after each version; every first-party
file carries `SPDX-FileCopyrightText: Cadasto B.V.` and
`SPDX-License-Identifier: BUSL-1.1`. The vendored specifications keep their
upstream terms, recorded beside them.

## Working discipline (`.claude/`)

- Rules: `writing-style`, `rust-style`, `reliability`, `comments`, `testing`,
  `spec-adherence`, `vendored-inputs`, `ci-cd`, `crates-publishing`,
  `issue-workflow`, `issue-relationships`, `project-board`.
- Agents: `implementer`, `spec-researcher`. Skills: `next-task`,
  `phase-done`, `phase-status`.
- Memory: `.claude/memory/MEMORY.md` is the index; every learning is a
  tracked file there.
