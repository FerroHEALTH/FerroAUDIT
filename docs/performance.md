<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Performance

Every product in the family sends its audit trail through `ihe-atna`, so the
library must not get slower without anyone noticing. The performance gate
measures each change against its base and each release against the last
published release. It fails on a regression (#331). No specification governs
the gate: it is our own design.

## What is measured

`tools/perf-arr` is an Audit Record Repository built from `ihe-atna` the way
a product mounts it. It runs as one process:

- the ATX: FHIR Feed `create` and `batch` (ITI TF-2 §3.20.4.2 and
  §3.20.4.4) and ITI-81 (§3.81.4) on an HTTP server;
- the ITI-20 syslog receiver (§3.20.4.1) on a TLS listener. Its spool
  delivers each message into the store as the `AuditEvent` of its DICOM
  audit message;
- an ITI-20 sender. A `POST /syslog` writes a DICOM audit message and hands
  it to the sender, whose spool and forwarder deliver it over RFC 5425 TLS
  to the receiver.

The store holds a fixed corpus of 10,000 synthetic records spread over 1,000
patients, which ITI-81 searches. It also keeps the records it receives in a
bounded ring, so memory stays flat however long the load runs. Both spools
are on disk under a tmpfs directory (`/dev/shm`), so the runner's disk adds
no noise to the comparison.

`perf/plan.toml` sends 40 requests per second, open loop, in four groups,
each naming a random patient of the corpus:

| Group | Request | What it loads |
|---|---|---|
| `feed-create` | `POST /fhir/AuditEvent`, one BALP patient read | the FHIR Feed reader and the store |
| `feed-batch` | `POST /fhir`, a `batch` of ten records | the batch reader and its `batch-response` |
| `iti-81` | `GET /fhir/AuditEvent?patient.identifier=…&date=ge2026-01-01&_count=10` | the query reader, the matcher over 10,000 records, the searchset and the "Audit Log Used" record |
| `iti-20-syslog` | `POST /syslog` | the DICOM writer, the RFC 5424 frame, the sender's spool and forwarder, the TLS hop, the receiver's frame reader, its spool and forwarder, and the DICOM-to-`AuditEvent` mapping |

An `iti-20-syslog` request is answered once the sender's spool stored the
message, which is what an audited request waits for. The delivery behind it
counts in the CPU and memory of the run.

## How it judges

`scripts/perf/gate.sh` runs `ferrohealth-perf` (pinned in
`docs/VERSIONS.md`) in its A/B mode. It builds the harness twice in release
mode: once over the head's `ihe-atna`, and once over the base's. On a pull
request the base is the pull request's base. In the release lane the base is
the newest `v*` tag before the tagged commit, the last published release. A
base that predates the harness gets the head's copy, built over the base's
crates.

The tool holds the head's idle CPU under 2 % of one core. It then runs four
pairs of 60-second load phases, the base and the head at once on the same
runner. A metric fails when the head's median is worse than the base's by
more than the family's tolerance (25 % for latency and CPU per request, 20 %
for throughput and peak memory) or the noise the two arms show, whichever is
larger, in most of the pairs. A run that cannot be judged (exit 2) is retried
on a fresh runner, at most twice. A confirmed breach fails at once.

CI runs the gate on a pull request that reaches `ihe-atna`, the harness, the
manifests or the gate, and on every push to `main`. The release lane runs it
before the GitHub release and the crates.io upload, so a regression is never
published.

## Latest numbers

Measured 2026-11-28 on GitHub's hosted `ubuntu-24.04` runner (Intel Xeon
Platinum 8573C, 4 vCPUs, 16 GiB) with `ferrohealth-perf` 0.19.0: the base is
`ihe-atna` 0.33.0, the head is the change that added the gate, and the gate
passed on its first attempt. Each figure is the median
over the pairs, in milliseconds unless the row names another unit.

| Metric | Base | Head | Head over base | Threshold |
|---|---:|---:|---:|---:|
| throughput, requests per second | 40.0 | 40.0 | 1.00 | 32.0 |
| peak memory, MiB | 274.2 | 273.1 | 1.00 | 329.1 |
| CPU per request | 1.471 | 1.490 | 1.01 | 1.839 |
| `feed-create` p50 / p95 / p99 | 0.244 / 0.354 / 0.440 | 0.260 / 0.383 / 0.453 | 1.06 / 1.08 / 1.03 | 0.306 / 0.443 / 0.550 |
| `feed-batch` p50 / p95 / p99 | 0.597 / 0.742 / 0.795 | 0.603 / 0.742 / 0.790 | 1.01 / 1.00 / 0.99 | 0.746 / 0.927 / 0.994 |
| `iti-81` p50 / p95 / p99 | 4.755 / 5.605 / 5.765 | 4.709 / 5.665 / 5.787 | 0.99 / 1.01 / 1.00 | 5.944 / 7.006 / 7.206 |
| `iti-20-syslog` p50 / p95 / p99 | 0.290 / 0.355 / 0.409 | 0.295 / 0.359 / 0.439 | 1.02 / 1.01 / 1.07 | 0.363 / 0.444 / 0.512 |

The head's idle CPU was 0.0 % of one core, against a limit of 2 %.
