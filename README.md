<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->
# FerroAUDIT

[![License: BUSL-1.1](https://img.shields.io/badge/License-BUSL--1.1-blue.svg)](LICENSE)

The shared audit library of the FerroHEALTH products, in pure Rust: the IHE
audit trail as `ihe-atna` and the EHDS logging component as `ehds-logging`
on crates.io. Every product sends its audit trail through them, so no
product implements audit on its own.

- **ATNA, ITI-20 Record Audit Event** (feature `atna`): the DICOM PS3.15
  audit message, the RFC 5424 syslog message it travels in with the header
  fields ITI-20 fixes, the RFC 5425 TLS connection to an Audit Record
  Repository, the bounded store-and-forward spool ITI-20 requires of a
  sender, and the forwarder that stores every record first and delivers it
  oldest first, retrying with a jittered backoff.
- **BALP, the Basic Audit Log Patterns** (feature `balp`): the FHIR R4
  `AuditEvent` an IHE FHIR transaction is audited as, written from one
  `Exchange` per transaction, and the ATX: FHIR Feed that posts it to an
  Audit Record Repository through the same spool and forwarder.
- **EHDS logging** (crate `ehds-logging`): the record of every access to
  personal electronic health data that Regulation (EU) 2025/327 Annex II 3.2
  asks of an EHR system, its Article 14(1) categories and the map that
  classifies an access, its Article 9(2) retention and security labels, and
  the record written as a BALP `AuditEvent` sent through `ihe-atna`.

A record names a patient toward its repository and nowhere else: no `Debug`
rendering, log line or error carries an identifier, a search, a user's values
or a credential. A record the spool cannot take is refused at once, so the
caller fails the exchange it audits; nothing is dropped silently.

FerroAUDIT is one of the [FerroHEALTH](https://ferrohealth.eu/) family.
FerroFED, FerroEHR and FerroPIX send their audit records through it, and
FerroSYS builds the receiving Audit Record Repository on it. This
repository is where the design and the build happen; the tracker is the
record of both.

## Use it

```toml
[dependencies]
ihe-atna = { version = "=0.2.8", features = ["balp"] }
ehds-logging = { version = "=0.2.8", features = ["balp"] }
```

The crate documentation on docs.rs is the integration guide; the design is
in [`docs/architecture.md`](docs/architecture.md). The specifications the
crates implement, ITI-20 Record Audit Event, the RESTful ATNA supplement,
BALP 1.1.4 and Regulation (EU) 2025/327, are vendored under [`docs/specs/`](docs/specs/) at the versions
[`docs/VERSIONS.md`](docs/VERSIONS.md) pins.

## Build and test

```
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo nextest run --workspace --all-features --locked
```

Every Audit Record Repository in the tests is a local listener or a
`wiremock` stub, and every patient identifier is synthetic.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md), [SECURITY.md](SECURITY.md) and
[SUPPORT.md](SUPPORT.md).

## Licence

FerroAUDIT is source-available under the Business Source License 1.1. The
parameters that apply, the Licensor, the Licensed Work, the Additional Use
Grant and the Change Date, are in [LICENSE](LICENSE): free for non-commercial
production use, a commercial licence for any other production use, and Apache
2.0 four years after each version is published. The crates are hand-written
code that implements the specifications above, which is why it is under the
family's own licence and not the Apache-2.0 of the family's generated crates.

A commercial licence is arranged with [Cadasto B.V.](https://www.cadasto.com),
the Licensor, which handles the business side of FerroAUDIT: write to
[info@cadasto.com](mailto:info@cadasto.com) or use
[cadasto.com/contact](https://www.cadasto.com/contact/). Technical questions go
to the maintainer named in [MAINTAINERS.md](MAINTAINERS.md).

The vendored specifications under `docs/specs/` keep their upstream terms,
recorded in the `PROVENANCE.md` beside each.

Contributions carry the terms in
[CONTRIBUTING.md](CONTRIBUTING.md#licensing-of-contributions): you keep your
copyright, and you grant the Licensor the relicensing right that keeps the work
one work under one licensor. There is no separate agreement to sign.
