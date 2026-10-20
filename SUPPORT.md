<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Getting help

## I have a question

Start at the [README](README.md), the crate documentation on docs.rs
(<https://docs.rs/ihe-atna>) and the design of record,
[`docs/architecture.md`](docs/architecture.md), which records every layer
with its specification citation. If those do
not answer it, open an issue through the
[issue chooser](https://github.com/FerroHEALTH/FerroAUDIT/issues/new/choose).

There is no commercial support offering and no service-level agreement. A
commercial licence or any other business question goes to Cadasto B.V., the
Licensor: [info@cadasto.com](mailto:info@cadasto.com) or
<https://www.cadasto.com/contact/>.

## I found a defect

[Open an issue](https://github.com/FerroHEALTH/FerroAUDIT/issues/new/choose)
when a record does not hold to its ITI-20 message format or BALP pattern, a
record is lost or delivered twice, or a patient identifier or a credential
reaches a log line or a `Debug` rendering. The forwarder's status and the
error text help; never paste a real audit record, patient identifier, key or
secret.

## I found a vulnerability

Do not open an issue. Follow [SECURITY.md](SECURITY.md).
