<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# FerroAUDIT

FerroAUDIT is the shared IHE audit library of the FerroHEALTH products,
published as the crate `ihe-atna`: the ATNA Record Audit Event transaction
(ITI-20) with its DICOM audit message, syslog over TLS, store-and-forward
spool and forwarder, and the IHE Basic Audit Log Patterns (BALP) `AuditEvent`
sent over the ATX: FHIR Feed. Every FerroHEALTH product sends its audit trail
through it.

The repository is being set up; the crate and its design land with the setup
pull request.

## Licence

FerroAUDIT is licensed under the Business Source License 1.1; see
[`LICENSE`](LICENSE) and [`NOTICE`](NOTICE). Cadasto B.V. is the Licensor.
