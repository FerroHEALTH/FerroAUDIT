<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Architecture

FerroAUDIT is the shared audit library of the FerroHEALTH products. It
publishes two crates on one version line: `ihe-atna`, through which every
product sends its audit trail (FerroFED's mandatory access log, FerroEHR's
FHIR audit feed and FerroPIX's Manager-side BALP records), and
`ehds-logging`, the logging software component of an EHR system under the
European Health Data Space Regulation, whose access record travels as a BALP
`AuditEvent` through `ihe-atna`. FerroSYS later builds the receiving Audit
Record Repository on the same crates. It is the owner's decision of
2026-10-20 (FerroHEALTH/FerroFED#367), and this document is the design of
record. No product implements audit, or the EHDS access record, again.

The code was written in FerroFED: `ihe-atna` in the `ihe-iti` crate behind
its `atna` and `balp` features, `ehds-logging` as its own crate at 0.0.26.
Both moved here with their tests. Every
decision below names its ground in a vendored specification (`docs/specs/`,
pinned in `docs/VERSIONS.md`), and where no specification governs a decision
the text says so: "no specification governs this: our own design".

Citation style: `ITI TF-2 §3.20.4.1.1` is the section of
`docs/specs/ihe-atna/Volume2/ITI-20.html`; `RESTful ATNA §3.20.4.2` is the
section of `docs/specs/ihe-atna/IHE_ITI_Suppl_RESTful-ATNA.pdf`; `BALP
§3:5.7.5.4` is the section of the BALP 1.1.4 pages under
`docs/specs/ihe-balp-pages/`, and a pattern such as `IHE.BasicAudit.Read` is
its StructureDefinition under `docs/specs/ihe-balp/package/`.

## 0. The picture

```text
  product (an IHE actor, a FHIR server, a gateway)
     │
     ├── atna::message::AuditMessage ──▶ atna::syslog::Sender (RFC 5424 + RFC 5425 frame)
     │                                         │
     └── balp::Exchange ──▶ AuditEvent (FHIR R4 JSON)
                                               │
                              atna::forwarder::Forwarder::submit
                                               │
                                     atna::spool::Spool (store first)
                                               │
                                  Forwarder::run (oldest first)
                               ┌───────────────┴───────────────┐
                 atna::repository::Repository        atna::feed::FeedRepository
                 syslog over TLS (RFC 5425)          FHIR create of AuditEvent
                               └──────▶ Audit Record Repository ◀──────┘
```

## 1. The role: the sending half of ITI-20

The crate is the Secure Node and Secure Application side of the ATNA profile:
it builds an audit record and delivers it to an Audit Record Repository with
Record Audit Event, ITI-20 (ITI TF-2 §3.20). It speaks both forms ITI-20
defines: the DICOM audit message over syslog (ITI TF-2 §3.20.4.1) and, with
the ATX: FHIR Feed Option the RESTful ATNA supplement adds, a FHIR
`AuditEvent` posted as a `create` (RESTful ATNA §3.20.4.2).

The receiving side, the Audit Record Repository with its query transactions
(ITI-81), is not here yet. FerroSYS builds it on this crate, which then
gains its server half as a feature of the same crate.

## 2. Pinned versions

| Item | Pin | Ground |
|---|---|---|
| ITI-20 | ITI TF-2 §3.20, Revision 20.2 Final Text | the published Technical Framework at the time of the move |
| RESTful ATNA | Rev. 3.6 Trial Implementation | the supplement that defines the ATX: FHIR Feed Option |
| BALP | 1.1.4 | the version the PIXm, PDQm, mCSD and PMIR audit profiles build on |
| FHIR | R4 (4.0.1) | BALP 1.1.4 declares FHIR 4.0.1 |
| EHDS Regulation | Regulation (EU) 2025/327 as published, with Implementing Regulation (EU) 2026/2099 | the law `ehds-logging` implements; no consolidated text exists |
| Article 14(1) category codes | `hl7.fhir.eu.health-data-api` 1.0.0-ballot | the code system HL7 Europe publishes for the six priority categories |

The corpora carry the pins FerroFED vendored them at, re-fetched here by the
same scripts, so the two trees hold the same bytes.

## 3. The `atna` feature

### 3.1 The audit message

`atna::message::AuditMessage` is the DICOM PS3.15 Annex A.5 `AuditMessage`
an ITI-20 syslog message carries (ITI TF-2 §3.20.4.1.2, §3.20.7): the event
identification, the active participants, the audit source and the
participant objects. A participant object's id, query and detail values may
name a patient, so they are held as secrets, and the written XML (UTF-8, no
byte order mark) is a secret too.

### 3.2 The syslog message

`atna::syslog::Sender` writes the RFC 5424 message with the header fields
ITI TF-2 §3.20.4.1.2 fixes: PRI `<85>` (facility 10, severity 5), MSGID
`IHE+RFC-3881`, no STRUCTURED-DATA, and the audit message as MSG. It frames
the message for the wire as RFC 5425 §4.3 octet counting.

### 3.3 The repository connection

`atna::repository::Repository` reaches a repository at `tls://host[:port]`
(port 6514 by default) over rustls at TLS 1.2 or later, with the platform's
roots plus the caller's, and an optional client certificate for the mutual
TLS of a Secure Node (ITI TF-2 §3.20.4.1.2.1.1 recommends TLS). Plain TCP
exists only through `Repository::unencrypted_for_development`: an audit
message names a patient. `Timeouts` bounds the connect, the TLS handshake and
each write and flush.

### 3.4 The spool

ITI TF-2 §3.20.4.1.1 has a sender that cannot reach its repository store the
record and send it when it can. `atna::spool::Spool` is that store, bounded
by messages and bytes (`Bounds`), the queued writes counted with the stored
ones. A message past a bound is refused at once, so the caller can fail the
exchange it audits; nothing is dropped silently. Storing one message is
bounded in time (`Bounds::write_timeout`): a late write is refused as
`SpoolError::Late`, runs on, and the message it stores is delivered like any
other.

On disk each message is one file, written to a temporary name, flushed,
renamed, and the directory flushed, so a stored message survives a crash. On
Unix the directory must be private to its owner and every file is created
readable by its owner alone. A message that cannot be read, or that is not of
the spool's content, moves to `quarantine/` when the drain reaches it, logged
by sequence number and never by content. A file the spool did not write
refuses `Spool::open`, naming the file. The bounds and the layout are our own
design: no specification governs them.

### 3.5 The forwarder

`atna::forwarder::Forwarder::submit` writes to the spool and never waits on
the network, so a slow repository never holds an exchange. `Forwarder::run`
delivers oldest first: to a syslog repository over one kept connection, to a
FHIR Feed repository one `create` per record. A transport failure drops the
connection and retries with a jittered exponential backoff capped where the
caller says. `Forwarder::status` reports the depth, the quarantine, the
deliveries and the retries. No specification governs the backoff: our own
design.

## 4. The `balp` feature

### 4.1 The exchange and the record

PIXm, PDQm, mCSD and PMIR define the audit record of each transaction as an
`AuditEvent` profile built on a BALP pattern. An audited client hands its
recorder one `balp::Exchange` per transaction: what the audit profile fixes
(`EventKind`), the outcome, the other party (`Peer`, `Direction`), whom the
exchange was made for (`user::OnBehalfOf`), and the entities it concerned.
`Exchange::audit_event` writes the `AuditEvent` with the `Observer` that
records it. Each coding a pattern fixes is a public constant (`REST`,
`SOURCE_ROLE`, `READ`, `QUERY_ROLE` and the others).

A user's purposes of use are written at the `agent:user` `purposeOfUse`
(BALP 1.1.4) and at `purposeOfEvent`, which RESTful ATNA Table
3.20.4.2.2.1-1 maps to the DICOM `PurposeOfUse` ITI TF-2 §3.20.7.1.2 asks
for.

### 4.2 The recorder

`balp::AuditRecorder` is a deployment's route to its repository. It is
awaited once per transaction, before the answer is returned, and it may
`reserve` a place before the exchange is made, so an exchange whose record
could not be taken is refused before it reaches the other party. The audit
record is part of the transaction: an exchange the recorder cannot accept
fails, and its answer is never used. No specification governs this: our own
design, the one ITI TF-2 §3.20.4.1.1 implies by asking a sender to keep every
record.

### 4.3 The FHIR Feed

`atna::feed::FeedRepository::send` posts one stored record to
`[base]/AuditEvent` as FHIR JSON (RESTful ATNA §3.20.4.2). A `2xx` is
success (§3.20.4.3.2). §3.20.4.3.3 leaves a failure to the client: a `4xx`
other than `408` and `429` is `FeedError::Rejected` and the forwarder
quarantines the record; every other failure is `FeedError::Unavailable` and
is retried. No redirect is followed, and no error carries the record or the
answer body.

## 5. Whom an exchange is made for

`user::OnBehalfOf` is a `User`, as a verified OAuth 2.0 access token names
them (`iss`, `sub`, `client_id`, `aud`, purposes of use, and the optional
organisation, alternative id, name, provider identifier, roles and assurance
level, BALP §3:5.7.5.4), or `System`. The module compiles with no feature,
since the identity profiles that audit name the user in their own records
too (the ITI-55 Human Requestor, ITI TF-2 §3.55.5.1.1).

## 6. Redaction

A record names a patient toward its repository and nowhere else. No `Debug`
of a message, an exchange, a user or a written record shows an identifier, a
patient reference, the search that carries them or a user's values; a URL
that may carry a credential shows through `RedactedUrl` with its userinfo
and query replaced. No log line carries a message's content. No
specification governs this: our own design.

## 7. Features and boundaries

| Feature | Compiles | Adds |
|---|---|---|
| none | `user` | nothing outside `std` |
| `atna` | `message`, `syslog`, `repository`, `spool`, `forwarder` | XML, timestamps, base64, rustls, tokio, tracing |
| `balp` | `atna` plus `balp` and `feed` | `fhir-types` (R4 `resources`), reqwest, serde_json |

`tests/it/features.rs` holds the boundary: `atna` compiles no FHIR model and
no HTTP client. The crate depends on no product.

## 8. The `ehds-logging` crate

Regulation (EU) 2025/327 asks an EHR system that gives access to personal
electronic health data to record every access: who accessed the data, which
natural person, the categories of the data, when, and where it came from
(Annex II 3.2). `ehds-logging` is that record and what classifies it:

- `record`: the `AccessRecord`, with the data subject and the purpose of use
  beside the five points of Annex II 3.2.
- `category`: the six priority categories of Article 14(1), coded with HL7
  Europe's `EEHRxFDocumentPriorityCategoryCS`, and the national categories a
  deployment declares.
- `map` and `classify`: a deployment's map from openEHR template and
  archetype ids to categories, and the classification of one access by the
  model ids of what it reached. An access the map cannot classify is marked
  unclassified with its evidence; it is never refused for it.
- `retention`: how long a record is kept, never under the three years of
  Article 9(2). `label`: the security labels per category and origin
  (Annex II 3.4). `emergency`: the purposes of use that assert an access in
  the vital interests of the data subject (Article 11(5)).
  `transformation`: the record of a semantic transformation an access
  applied (eHealth Network General Guidelines Release 3.1 Article 6(3)).
- `sink`: where records go; `balp` (feature `balp`): the record written as a
  BALP `AuditEvent` through `ihe-atna`'s `balp` feature, and the sink over an
  `ihe-atna` `AuditRecorder`.

The crate depends on no interoperability component and on nothing of the
system that builds the records, so the logging component stays independent
of the interoperability component (Article 2(2)(n), (o)). It ships no
category map: the deployment authors it. Its tests hold the six codes to the
vendored HL7 Europe packages and the record to the BALP patterns.

## 9. How a product integrates

A product pins `ihe-atna` and, where it writes the EHDS access record,
`ehds-logging` exactly (`=0.1.x`) from crates.io and enables the feature it
sends with. The migrations, each one pull request in the
product's repository:

- **FerroFED:** the mandatory access log (its A58) builds `ehds-logging`
  records, which take the place of its own copy of the crate, and sends them
  through the `balp` sink; the paths change from `ihe_iti::atna` and
  `ihe_iti::balp` to `ihe_atna::atna` and `ihe_atna::balp`.
- **FerroPIX:** `ihe-iti`, which moves there, takes its profile audit records
  (the PIXm, PDQm, mCSD, PMIR and ITI-55 writers) from this crate, and the
  Manager-side BALP records go through it.
- **FerroEHR:** its `[audit.fhir_feed]` sends through `balp` and the feed,
  and its own implementation of the EHDS access record moves onto
  `ehds-logging` (its own issue).
- **FerroSYS:** builds the Audit Record Repository on this crate.

## 10. Naming and licence

The crate takes the specification's name, never a product's, as every
published crate of the family does. It is `ihe-atna` because ATNA is the
profile whose transaction it carries; BALP is a feature of it, since BALP's
records travel over ATNA's FHIR Feed (BALP §1:52.1.1.1). It is under
BUSL-1.1: hand-written code implementing a specification is BUSL in this
family, and only generator-emitted code is Apache-2.0. `ehds-logging` takes
the name of the component the Regulation defines, the European logging
software component (Article 2(2)(o)).

Both crates inherit the workspace version and release together under one
tag; `ehds-logging` pins `ihe-atna` exactly at that version. The line starts
at 0.1.0, above FerroFED's last `ehds-logging` 0.0.26. No specification
governs this: our own design.

## 11. Verification

The tests run on every pull request (`.github/workflows/ci.yml`): the spool
and forwarder against local listeners, the FHIR Feed against a `wiremock`
repository, the written records against the vendored BALP Read pattern and
capability statements, the store-and-forward against the vendored ITI-20
text, and the Article 14(1) category codes against the vendored HL7 Europe
packages. Every patient identifier in them is synthetic.
