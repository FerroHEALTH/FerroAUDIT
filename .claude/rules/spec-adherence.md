<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Spec adherence (the published specifications are the oracle)

FerroAUDIT answers to these sources, never to memory and never to how
another implementation behaves. Each is vendored under `docs/specs/` at the
version `docs/VERSIONS.md` pins, and the design of record,
`docs/architecture.md`, cites them section by section:

1. **ITI-20 Record Audit Event** (IHE ITI TF-2 §3.20, Revision 20.2,
   `docs/specs/ihe-atna/Volume2/ITI-20.html`): the syslog interaction, the
   store-and-forward of §3.20.4.1.1, the message semantics and transports of
   §3.20.4.1.2, and the IHE Audit Trail Message Format of §3.20.7.
2. **The RESTful ATNA supplement** (Rev. 3.6,
   `docs/specs/ihe-atna/IHE_ITI_Suppl_RESTful-ATNA.pdf`): the ATX: FHIR Feed
   Option, Send Audit Resource Request and Response (§3.20.4.2, §3.20.4.3)
   and the mapping from the DICOM message to the FHIR `AuditEvent`.
3. **BALP 1.1.4** (`docs/specs/ihe-balp/`, the package;
   `docs/specs/ihe-balp-pages/`, the Volume 1 and Volume 3 text): the audit
   patterns, their slices and fixed codings, and the token-to-agent mapping
   of §3:5.7.5.4.
4. **For `ehds-logging`, Regulation (EU) 2025/327** (`docs/specs/eu-ehds/`,
   with Implementing Regulation (EU) 2026/2099, the eHealth Network Patient
   Summary and General Guidelines and the reuse Decision), and the HL7 Europe
   and MyHealth@EU packages the Article 14(1) categories are held to
   (`docs/specs/eu-hl7-*/`, `docs/specs/ehdsi-mvc/`).
5. **The texts they cite**: RFC 3881, RFC 5424, RFC 5425, RFC 5426 and RFC
   6750 (`docs/specs/ietf-syslog/`), the openEHR BASE Base Types
   specification (`docs/specs/openehr-base/`), DICOM PS3.15 Annex A.5,
   pinned by release and sha256 (`docs/specs/dicom-ps3-15/PROVENANCE.md`)
   and read locally, since NEMA grants no licence to redistribute it, and,
   by link, RFC 6587 and FHIR R4 (4.0.1).
6. **The docs.rs pages of the pinned crates** (`quick-xml`, `rustls`,
   `tokio-rustls`, `reqwest`, `secrecy`, `fhir-types`), the Rust book and
   reference, for every question about the tools.

## Hard rules

- **Read the governing section before implementing or reviewing** a
  message field, a header, a transport rule, a retry or a pattern slice,
  and cite it (document and section) in the `// NOTE:` or the pull request.
- **NEVER LAX. Strictness is a hard rule.** The library writes exactly what
  the governing specification asks. A record that does not hold to its
  pattern is a failing test, never a tolerated drift. A choice outside the
  specification is recorded in `docs/architecture.md` and flagged "no
  specification governs this: our own design".
- **Cite only durable references**: the sources above or official external
  documentation. Never cite an internal markdown file as a design authority.
  Where no specification governs a decision (the spool bounds, the backoff,
  the redaction), flag it: "no specification governs this: our own design".
- **The products' audit code is prior art**, never the oracle. Where a
  product's behaviour and a specification disagree, the specification wins,
  and the difference is settled on the tracker before the product's
  migration, never by editing a test.
- **Never claim a property no test or citation shows.**
- **A defect in a published specification is recorded as an
  `upstream-report` issue**, with what the specification says, what this
  implementation does and the resolution an upstream would need. The issue
  is the record and stays here.
