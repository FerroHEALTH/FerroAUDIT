<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Testing discipline

Test discipline is non-negotiable (a standing hard rule; see `CLAUDE.md`). It
applies to every crate.

## The hard rule

- **Never** silently weaken, skip, or delete an existing test to make a build
  pass.
- **Never** edit a test to route around a bug it exposes. If a test fails and
  the fix is unclear, leave it failing and record a `// TODO(#NNNN):` naming
  its issue.
- A conformance test asserts the specification: cite the section it encodes
  (ITI-20, the RESTful ATNA supplement, a BALP pattern), and never adjust an
  expectation to match a defect. The tests carried from FerroFED's `ihe-iti`
  are the contract the crate meets; a change to one is settled on the
  tracker, never by editing the test.

## Tooling

- **Runner:** `cargo-nextest` (`cargo nextest run --workspace`, with no
  feature, with `--features atna` and with `--all-features`).
- **Repositories:** a syslog Audit Record Repository is a local listener
  the test opens; a FHIR Feed repository is a `wiremock` stub. No test
  reaches the network.
- **The vendored corpora** are read from `docs/specs/` by path: a written
  record is held to the vendored BALP pattern, and the store-and-forward to
  the vendored ITI-20 text.
- **Timing:** the forwarder talks over real sockets, so the tokio clock is
  never paused under it; a deadline adds `timing::SLACK` to the waits the
  timeouts allow.
- **Logs** are read back through a test `tracing::Subscriber`, so a case
  asserts that no message content reaches a log line.

## Coverage is a mandate, not just pass rate

- the spool: the bounds, the late write, the quarantine, the private
  directory, the foreign file, the order of delivery;
- the forwarder: delivery, retry with backoff, a stalled repository given up
  within the send timeout, a record stored after its exchange gave up
  delivered once;
- the FHIR Feed: the `create`, a refusal quarantined, an unavailable
  repository leaving records spooled;
- the records: each fixed coding, the user's agents, the purposes of use at
  both places, the redaction of every `Debug`;
- the feature boundaries.
- **Coverage only ratchets up.** Cases are added, never removed to go green.
- **One behaviour per case.**

## Where tests live

Unit tests live beside the code they test (`#[cfg(test)] mod tests`); a test
that drives the public API lives in the crate's one integration binary,
`tests/it/main.rs` plus one `mod` per topic. Dedicated test files under `src/`
are banned.

## Test shapes

- `Result`-returning tests where plumbing can fail; assertions carry a
  message where the values alone do not say what failed.
- Doctests are copy-paste templates, never `ignore`; `no_run` for an
  example that would open a socket.

## No patient data, no credential, ever

Every patient identifier, user, client id and password the tests show is
invented for them (the `Qz7-` prefix marks a value no other text holds), and
no key, certificate or secret is committed.
