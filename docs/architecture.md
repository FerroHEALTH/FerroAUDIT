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
| DICOM PS3.15 | 2026c | the release current when the optional message elements and the code tables were added; pinned by sha256 and read locally, since NEMA grants no licence to redistribute it |
| Syslog and audit RFCs | RFC 3881, 5424, 5425, 5426, 6750 | the RFCs ITI TF-2 §3.20.3 references, the RFC 3881 codes the IHE tables write, and the bearer token usage the Feed's credentials follow |
| openEHR BASE | Release 1.1.0 | the release FerroFED vendors, whose identification package fixes the case rule and the `ARCHETYPE_ID` grammar |

The corpora carry the pins FerroFED vendored them at, re-fetched here by the
same scripts, so the two trees hold the same bytes.

## 3. The `atna` feature

### 3.1 The audit message

`atna::message::AuditMessage` is the DICOM PS3.15 Annex A.5 `AuditMessage`
an ITI-20 syslog message carries (ITI TF-2 §3.20.4.1.2, §3.20.7): the event
identification, the active participants, the audit source and the
participant objects. A participant object's id, query and detail values may
name a patient, so they are held as secrets, and the written XML (UTF-8, no
byte order mark) is a secret too. The optional elements PS3.15 A.5.1 defines
and the IHE tables leave not specialized, `EventOutcomeDescription`,
`AuditSourceTypeCode` and `ParticipantObjectDataLifeCycle`, are written when
the caller gives them, in the schema's order. `atna::codes` holds the DICOM
codes of the A.5.3 message tables and the RFC 3881 §5.5.4 participant object
id types as `(code, codeSystemName, originalText)` triples.

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

The forwarder delivers from any `Outbox`: the spool is one, and a product
that keeps its records in its own durable store, such as a database table,
implements the trait and gets the same delivery, backoff and quarantine
(ITI TF-2 §3.20.4.1.1 leaves where a record is stored to the sender). The
spool's events log under the target `ihe_atna::atna::spool` and the
forwarder's under `ihe_atna::atna::forwarder`, both constants of
`atna::targets` that never follow the module layout. No specification
governs the targets: our own design.

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

A `Coded` holds fixed or run-time text, so a subtype known only when the
exchange is made, such as an operation id, follows the pattern's codes:
every RESTful pattern slices `subtype` openly. The record writes the
major-failure outcome `12` (FHIR R4 `audit-event-outcome`), `outcomeDesc`
and the observer's `source.type` (FHIR R4 `security-source-type`, which
RESTful ATNA Table 3.20.4.2.2.1-1 maps to `AuditSourceTypeCode`) when given,
and the token's `jti` at `agent[user].policy` (BALP §3:5.7.5.4). A record
that names the `jti` and an Application agent with the `client_id` claims
`IHE.BasicAudit.OAUTHaccessTokenUse.Comprehensive` beside its RESTful
pattern; the Minimal pattern types its user agent `UserOauthAgent`, which
the RESTful `agent:user` is not, so no record claims it. The recording
system's own agent is addressed by URL, host or IP (`network-type` `5`,
`1`, `2`). A patient identifier's namespace is `Identifier.system` only
when it is a URI; any other namespace is a `namespace` detail of the
patient entity. `Exchange::audit_event_value` gives the record as a JSON
value behind a redacting type, for a store that keeps JSON.

`balp::recording` bounds the record of an audited exchange by the
exchange's own timeout: a record not accepted in time fails a successful
exchange with `Late`, while the recorder goes on storing it. No
specification governs the bound: our own design.

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
other than `401`, `408` and `429` is `FeedError::Rejected` and the forwarder
quarantines the record; a `401` is `FeedError::Unauthorized`, the
credential's fault and never the record's, and is retried; every other
failure is `FeedError::Unavailable` and is retried. A repository can take a
`Credentials` source asked for the `Authorization` value of each request;
after a `401` it is told, and the request is sent once more with a fresh
value (RFC 6750 §3.1). `FeedRepository::new` takes `https` alone;
`FeedRepository::cleartext` reaches `http` for an operator who accepts that
records cross the network in clear text. No redirect is followed, and no
error carries the record, the credential or the answer body.

Under the `smart` feature, `atna::bearer::Bearer` is that `Credentials`
source over the client credentials token of `ferrohealth-smart`, the
family's SMART crate (its `TokenSource` fetches, caches and renews the
token): it gives the token's `Authorization` value and drops a refused token
so the repeated request carries one fetched anew. Every product that
authorizes its Feed with that grant takes it in place of an adapter of its
own. The dependency points from the audit crate to the SMART crate and is
pinned exactly, as the products pin it.

### 4.4 The retrieval (feature `retrieve`)

Retrieve ATNA Audit Event [ITI-81] is the read side of a repository: an
HTTP `GET` that is a FHIR R4 search on `AuditEvent` (RESTful ATNA
§3.81.4.1.2). `retrieve::Query::parse` reads its query string and nothing
else; the store, the query over it and the decision who may read which
records stay the repository's (§3.81.4.1.3, ITI TF-1 §9.4).

- **The parameters.** At least one `date` is required (§3.81.4.1.2.1), and a
  query without one is refused, which §3.81.4.2.2 lets a repository answer
  with a `400`. The other parameters are the four token parameters
  §3.81.4.1.2.2 adds (`agent.identifier`, `patient.identifier`,
  `entity.identifier`, `source.identifier`) and the FHIR R4 `AuditEvent`
  search parameters, each typed by the registry `fhir-types` generates from
  the R4 core package, which covers the rest of §3.81.4.1.2.2 (`address`,
  `entity-type`, `entity-role`, `type`, `subtype`, `outcome`). Repeated
  parameters combine with AND, values with OR (§3.81.4.1.2.2). Any other
  parameter, `_sort` and `_include` among them, is ignored and named
  (§3.81.4.1.3). `_count` is FHIR R4 Search §3.1.1.5.3; `_offset` is the
  page offset, which no specification governs: our own design, since FHIR
  leaves the paging link to the server. A modifier on a supported parameter
  is refused, as FHIR R4 Search §3.1.1.4.4 has a server do.
- **The values.** The query string is percent-decoded as RFC 3986 §2.1
  says (§3.81.4.1.2.2), so `+` stays a plus; `\` escapes follow FHIR R4
  Search §3.1.1.4.19, tokens the four forms of §3.1.1.4.10. A date is held
  as an R4 `dateTime` and checked against its lexical form, after `:00` is
  written into a time of hours and minutes, which §3.1.1.4.7 allows in a
  search.
- **The dates.** `recorded` is an instant, a point (§3.1.1.4.7), so each
  prefix of §3.1.1.4.5 becomes an interval around the range the value's
  precision implies: `eq` within it, `ne` outside it, `ge` from its start,
  `le` until its end, `gt` and `sa` from its end, `lt` and `eb` until its
  start. A value without a time is read in the time zone the repository
  names, as §3.1.1.4.7 has a server assume its local zone. `ap` is refused:
  FHIR leaves its approximation to the server, and the crate chooses none.
- **The answer.** `bundle::searchset` writes the `searchset` Bundle of
  §3.81.4.2.2.2 with `total`, the `self` link and an optional `next` link,
  each entry in `search.mode` `match`; `bundle::refusal` the
  `OperationOutcome` with the human-readable description §3.81.4.2.2 asks
  for. Both are `fhir_types::r4` resources, never hand-built JSON. The
  retrieval needs no transport of its own: the feature takes `fhir-types`,
  `jiff` and `percent-encoding`, and neither an HTTP client nor a runtime.

## 5. Whom an exchange is made for

`user::OnBehalfOf` is a `User`, as a verified OAuth 2.0 access token names
them (`iss`, `sub`, `client_id`, `aud`, `jti`, purposes of use, and the
optional organisation, alternative id, name, provider identifier, roles and
assurance level, BALP §3:5.7.5.4), a `User::local` authenticated otherwise,
with no issuer and no client, whose record writes neither, or `System`. The module compiles with no feature,
since the identity profiles that audit name the user in their own records
too (the ITI-55 Human Requestor, ITI TF-2 §3.55.5.1.1).

## 6. Redaction

A record names a patient toward its repository and nowhere else. No `Debug`
of a message, an exchange, a user or a written record shows an identifier, a
patient reference, the search that carries them or a user's values; a URL
that may carry a credential shows through `RedactedUrl` with its userinfo
and query replaced. No log line carries a message's content. `redact` and
`fhir_base` are public, so a product that logs a URL or joins a FHIR base
uses the same rule. No specification governs this: our own design.

## 7. Features and boundaries

| Feature | Compiles | Adds |
|---|---|---|
| none | `user` | nothing outside `std` |
| `atna` | `message`, `codes`, `syslog`, `repository`, `spool`, `forwarder` | XML, timestamps, base64, rustls, tokio, tracing |
| `recording` | `balp::recording` alone | tokio, thiserror |
| `redact` | `redact::RedactedUrl`, `fhir_base` | url |
| `balp` | `syslog`, `spool`, `forwarder`, `balp`, `feed` | `fhir-types` (R4 `resources`), reqwest (no TLS backend of its own), serde_json, timestamps, base64, tokio, tracing |
| `retrieve` | `retrieve` | `fhir-types` (R4 `resources`), timestamps, percent-encoding |
| `smart` | `balp` and `bearer` | `ferrohealth-smart` (`fetch`) |

`tests/it/features.rs` holds the boundary: `atna` compiles no FHIR model and
no HTTP client, and `balp` compiles no TLS stack and takes no XML writer of
its own (the owner's decision of 2026-10-20; `fhir-types` takes `quick-xml`
for its XML form). The Feed's TLS is the backend the caller builds its
`reqwest::Client` with. The crate depends on no product.

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
of the interoperability component (Article 2(2)(n), (o)); its own
`tests/it/features.rs` holds that with each feature on.

The map compares keys exactly, or, declared with `Matching::OpenEhr`, as
openEHR BASE 1.3.0 compares composite identifiers: case-insensitively and
case-preserving, so the evidence keeps each id as the access named it.
Archetype keys follow the `ARCHETYPE_ID` grammar of the same specification,
the legacy form with a major version from `0` and the namespaced ADL 2 form,
read as FerroEHR's `openehr-base` reads it (a test holds the two to one
answer),
and a declaration reports every defect at once. A root object of a resource
kind that holds no data of any category (`EHR`, `EHR_STATUS`, `FOLDER` and
the others) adds "no category" with its kind beside the categories of the
content around it, on the basis `resource-kind`. A map can bound each
evidence set of a classification, keeping the true count beside it.

The accessor's issuer, client and acting mode are optional, so a user
authenticated without a token is recorded without invented values; the
accessor carries the roles the person acted under, written at
`agent[user].role` beside the acting mode, and the token's `jti`. An origin
names the endpoint the access asked, the system stored data came from, or
both, and is keyed for retention and labels by its endpoint, else its
`system_id`. A request may have no id, and its operation can be written as a
second subtype in a named code system.

Retention is a period in years, in days or without end, declared within
bounds: inside the Union a floor of Art 9(2)'s three years, a national
floor at or above it and a ceiling, each with the legal ground the
deployment cites; outside the Union a floor the deployment's own law sets,
which may be under three years, and which every record names in place of
Art 9(2). A period in days stands over a floor in years only when it
reaches it from any date of access. The crate carries no table of national
law: the deployment states each bound and its citation. No specification
governs how bounds combine with categories and origins: our own design. It ships no
category map: the deployment authors it. Its tests hold the six codes to the
vendored HL7 Europe packages and the record to the BALP patterns.

## 9. How a product integrates

A product pins `ihe-atna` and, where it writes the EHDS access record,
`ehds-logging` exactly (`=0.2.7`) from crates.io and enables the feature it
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

## 12. Decisions not taken

Recorded on the tracker with their grounds; each is a decision, not a gap.

- **Syslog over UDP (RFC 5426).** ITI TF-2 §3.20.4.1.2.1.2 permits it, but
  RFC 5426 §4.1 and §5.4 say datagrams can be lost "without any notice" and
  the transport may truncate a message over 1024 bytes, which breaks this
  crate's rule that no record is dropped silently (FerroHEALTH/FerroAUDIT#50).
- **A narrower FHIR model for `balp`.** `fhir-types` has no feature narrower
  than `resources` that holds `AuditEvent`, and a hand-written `AuditEvent`
  would fork the generated model; the route is a root-set feature of
  `fhir-types` in its generator (FerroHEALTH/FerroAUDIT#56).
- **A per-access legal basis, a pseudonymisation domain and the raw
  assurance claim.** Annex II 3.2 asks for none of them, and they stay the
  columns of the product that keeps them (FerroHEALTH/FerroAUDIT#29,
  FerroHEALTH/FerroAUDIT#30).
- **A synthesized emergency code on `purposeOfEvent`.** The record keeps the
  purposes the accessor declared there and states the mark in its own
  entity (FerroHEALTH/FerroAUDIT#48).
