<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# ihe-atna

The IHE audit trail in Rust: the sending half of ATNA Record Audit Event
(ITI-20), and the Basic Audit Log Patterns (BALP) `AuditEvent` sent over
ITI-20's ATX: FHIR Feed, with a feature per layer.

| Feature | Layer | Transaction |
|---|---|---|
| `atna` | Audit Trail and Node Authentication: the DICOM audit message over syslog with TLS, the spool and the forwarder | ITI-20 |
| `balp` | Basic Audit Log Patterns: the FHIR `AuditEvent` of an IHE FHIR transaction and its spool, without the syslog sender, its TLS stack or an HTTP client | ITI-20 |
| `feed` | The ITI-20 FHIR Feed that sends the `AuditEvent`s through the same spool and forwarder, over the caller's `FeedClient` | ITI-20 |
| `retrieve` | The retrieval an Audit Record Repository serves: the query string read into typed criteria, and the `searchset` Bundle and refusal `OperationOutcome` of the answer | ITI-81 |
| `arr` | The Audit Record Repository with the Retrieve Audit Message Option, over the caller's store: ITI-20 received over RFC 5425 TLS and the FHIR Feed, and ITI-81 answered | ITI-20, ITI-81 |

`recording` compiles `balp::recording` alone and `redact` compiles
`redact::RedactedUrl` and `fhir_base` alone, for a profile crate that audits
through its own model or logs URLs it does not audit.

Under `feed` the FHIR Feed sends through your implementation of
`atna::feed::FeedClient`, written over your own HTTP client, which owns the
TLS, the timeouts, the credential and its renewal, and the tries of one
round. The crate links no HTTP client and no auth library.

The crate depends on no application. The profiles are published at
<https://profiles.ihe.net/ITI/>.

## ATNA (`atna`)

`ihe_atna::atna` is the sending half of ITI-20, Record Audit Event (ITI TF-2
§3.20), as syslog with the DICOM message:

- `message::AuditMessage`: the DICOM PS3.15 Annex A.5 audit message, with
  its participant objects held as secrets and written to UTF-8 XML with no
  byte order mark, every value through `escape::untrusted`, so no control
  or bidirectional character reaches the XML.
- `syslog::Sender`: the RFC 5424 message with the PRI `<85>` and the MSGID
  `IHE+RFC-3881` §3.20.4.1.2 fixes, in its RFC 5425 octet-counted frame.
- `repository::Repository`: the connection to an Audit Record Repository,
  `tls://host[:port]` over the rustls `ClientConfig` you hand it, which
  speaks TLS 1.2 or 1.3 alone and carries your trust roots and client
  certificate; `unencrypted_for_development` admits `tcp://` for
  development and tests.
- `repository::Timeouts`: the connect timeout, and the send timeout that
  bounds the TLS handshake and each write and flush.
- `spool::Spool`: the bounded store a sender that cannot reach its
  repository keeps the records in (§3.20.4.1.1), in a directory private to
  its owner, or a queue in memory. One writer thread appends the messages
  that queue behind one flush to the active segment as the next batch, each
  framed by its length and its CRC-32, with one flush per batch; a segment
  of 4,096 messages or 8 MiB is sealed with an index of its frames, so an
  open reads indexes instead of messages. A sidecar counts the messages
  delivered, and a crash sends again what it had not counted. A message that
  cannot be read or is no whole frame, and the bytes of a segment from a
  frame that fails its CRC on, move to its `quarantine` subdirectory, so the
  drain never stalls on them. Storing one
  message is bounded in time (`Bounds::write_timeout`): a write that misses
  the bound is refused as `SpoolError::Late` and runs on, and a message it
  stores is delivered like any other.
- `forwarder::Forwarder`: stores every message first, then delivers it from
  the spool in order, retries a transport failure with a jittered backoff
  capped where the caller says, and reports the spool's depth, its
  quarantine, its deliveries and its retries. `forwarder::Outbox` lets it
  deliver from your own durable store instead, such as a table of records
  not yet sent.
- `codes`: the DICOM EventID, EventTypeCode, RoleID, AuditSourceTypeCode and
  ParticipantObjectDataLifeCycle codes and the RFC 3881 participant object id
  types, as triples `CodedValue::of` reads.
- `targets`: the stable `tracing` targets of the spool and the forwarder.

A product builds the TLS configuration from the family's outbound TLS keys
(`client_identity` or `client_identity_file`, and `trust_roots_file`) with
`ferrohealth-server`'s `TlsClient::platform_config`, so the syslog
connection trusts what the product's HTTP clients trust, and hands it to the
repository:

```rust
use std::sync::Arc;
use std::time::Duration;

use ihe_atna::atna::repository::{Repository, Timeouts};
use rustls::ClientConfig;
use url::Url;

// `tls` is what `TlsClient::platform_config` built from `[audit.syslog]`.
fn repository(tls: ClientConfig) -> Result<Repository, Box<dyn std::error::Error>> {
    let address = Url::parse("tls://arr.example.org")?;
    let timeouts = Timeouts {
        connect: Duration::from_secs(5),
        send: Duration::from_secs(10),
    };
    Ok(Repository::tls(&address, Arc::new(tls), timeouts)?)
}
# let _ = repository;
```

## BALP (`balp`)

`ihe_atna::balp` writes the FHIR R4 `AuditEvent` of one audited exchange:

- `Exchange`: what the transaction's audit profile fixes (`EventKind`), the
  outcome, the other party, whom the exchange was made for and the entities
  it concerned; `Exchange::audit_event` writes the record with the
  `Observer` that records it. Every coding a BALP pattern fixes is a public
  constant.
- `AuditRecorder`: a deployment's route to its repository, awaited once per
  transaction before its answer is used, with `reserve` to take a record's
  place before the exchange is made.
- `atna::feed::FeedRepository` with `Forwarder::fhir_feed` (feature
  `feed`): the ATX: FHIR Feed's Send Audit Resource Request, a FHIR
  `create` of each stored record at `[base]/AuditEvent`, sent through your
  `FeedClient`, which carries the endpoint's credential (Basic, or a client
  credentials token renewed once on a `401`, RFC 6750 §3.1) and the tries of
  a round; the forwarder waits the repository's `between_rounds` after each
  failed round. A refused record is quarantined,
  every other failure retried in rounds that never give it up; `cleartext`
  reaches an `http` base for an operator who accepts it.
- `recording`: the bound on an audited exchange's record, `Late` and
  `within`.
- `Exchange::audit_event_value`: the record as a JSON value, for a store
  that keeps JSON.
- `Exchange::audit_event_resource`: the record as the typed FHIR R4
  `AuditEvent`, already escaped, for a repository that serves it typed,
  such as in an ITI-81 searchset, without reading the written record back.

## Retrieve (`retrieve`)

`ihe_atna::retrieve` serves Retrieve ATNA Audit Event (ITI-81), the FHIR R4
search on `AuditEvent`:

- `Query::parse`: the query string, percent-decoded, read into the `date`
  parameters and the other criteria. Each parameter takes its type (date,
  string, token, reference, uri) from the FHIR R4 search-parameter registry
  of `fhir-types`, with the four token parameters the RESTful ATNA
  supplement adds (`agent.identifier`, `patient.identifier`,
  `entity.identifier`, `source.identifier`). Repeated parameters combine
  with AND and the values of one with OR; a parameter the crate does not
  support is ignored and named in `Query::ignored`. A query with no `date`
  is refused.
- `date::DateParam::bounds`: the interval of `recorded` instants a `date`
  value matches, from its prefix and the range its precision implies.
  Every value, its escapes, tokens and dates, is read by the family's FHIR
  search grammar, `fhir-search`, so a query reads the same here as in the
  other FerroHEALTH FHIR servers.
- `bundle::searchset` and `bundle::refusal`: the answer as `fhir_types::r4`
  resources.

The repository keeps its store, the query over it and who may read what.

## Audit Record Repository (`arr`)

`ihe_atna::arr` is the receiving side, mounted by a product over its own
store (`arr::AuditStore`):

- `arr::syslog::serve` and `receive`: ITI-20 over RFC 5425 TLS. Each frame
  is stored in the repository's spool before the next is read, and the
  forwarder `Forwarder::into_store` delivers it into the store as the
  `AuditEvent` of RESTful ATNA Table 3.81.4.2.2.1-1 (`arr::dicom`). A full
  spool stops the reading, so the sender keeps its records; a message the
  mapping cannot carry is quarantined.
- `arr::feed::receive`: the FHIR Feed `create` and `batch`, JSON or XML,
  answered with a `2xx` only once the store kept the record.
- `arr::search::retrieve`: ITI-81 over the store, with `arr::search::matches`
  for every parameter of RESTful ATNA §3.81.4.1.2, and the "Audit Log Used"
  record of §3.81.5.1 kept before the answer is returned.

The product's HTTP server and TLS configuration stay its own.

## Whom an exchange is made for

`ihe_atna::user::OnBehalfOf` is the `User` an OAuth 2.0 access token names
(its `iss`, `sub`, `client_id`, `aud`, `jti` and purposes of use, and
optionally the organisation, name, provider identifier, roles and assurance
level), a `User::local` authenticated otherwise, with no issuer and no
client, or `System` for an exchange the system makes on its own behalf. The BALP record
names the user in the `agent:user` slice and an Application agent with the
`client_id` (BALP 1.1.4 §3:5.7.5.4). `User`'s `Debug` shows none of its
values. The module compiles with no feature, and so do `redact`, the
`Debug` rendering of a URL with its credential redacted, and (with either
feature) `fhir_base`, the URL of an interaction under a FHIR base.

## Licence

Business Source License 1.1 (`LICENSE` in the repository): free for every
non-production use and for non-commercial production use; a commercial
licence for other production use; Apache License 2.0 four years after each
version. Source: <https://github.com/FerroHEALTH/FerroAUDIT>.
