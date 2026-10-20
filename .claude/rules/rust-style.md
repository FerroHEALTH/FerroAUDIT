<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Rust style: idiomatic library code

Applies to every hand-written `.rs` file in the repository. **FerroAUDIT is
modern idiomatic Rust of our own design**, held to the specifications it
implements: ITI-20 Record Audit Event, the RESTful ATNA supplement and BALP
1.1.4 (`spec-adherence.md`). The products' own audit code is prior art: read
it for how a problem was solved, never port it blindly.

## Build idiomatic, compiling, tested code

- **Use proper crates; do not hand-roll** what the pinned set provides: the
  XML writer (`quick-xml`), the TLS stack (`rustls`, `tokio-rustls`), the
  HTTP engine (`reqwest`), the FHIR model (`fhir-types`), the secrets
  (`secrecy`). Add a dependency through
  the root `Cargo.toml [workspace.dependencies]` table (`dep.workspace =
  true`), and verify its version against crates.io or docs.rs the day it is
  added, never from memory. A new library dependency reaches every product,
  so it is justified against what it brings.
- **Every crate you touch compiles, is clippy-clean with no feature, with
  `atna` and with every feature, and is tested before you move on.**
- **Read the governing specification first** for any message field,
  header, transport or pattern behaviour (`spec-adherence.md`).

## File and module size

A hand-written Rust file is at most 1000 lines, and a file over 750 lines is
split into a module folder before it grows further. The shape of a split is
a folder `<name>/mod.rs` holding the module doc, the `mod` declarations and
the items every child shares, with one child file per concern. A split never
creates a test file under `src/`, adds no `pub use`, and moves code without
changing behaviour.

## Comments and documentation

The comment and doc-comment discipline lives in **`comments.md`** (RFC 505
plus RFC 1574): line comments only, `// TODO(#NNNN):` / `// NOTE:` as the only
markers, a NOTE as a citation plus one sentence (at most 3 lines), plain `//`
runs at most 8 lines, the summary-line convention for doc comments.

## Default values live in the struct's `Default` impl (RFC 3681 shape)

One `impl Default` per struct, every default value inline, and
container-level `#[serde(default, deny_unknown_fields)]` on a configuration
section. The per-field `#[serde(default = "path")]` form, a zero-argument
`fn default_x()`, and a single-reader `const DEFAULT_X` are all banned: each
puts a field's default somewhere other than its own struct. A `const` with
more than one consumer stays a constant.

## RFC-grounded conventions

| RFC | The rule | Enforcement |
|---|---|---|
| 0505 + 1574 | comment and doc conventions | `comments.md` + `too_long_first_doc_paragraph` |
| 3681 | a field's default lives inline in its `Default` impl | §Default values above |
| 3107 | `#[derive(Default)]` + `#[default]` where the default is a variant | `clippy::derivable_impls` (deny) |
| 0199 / 0344 / 0430 | `as_`/`to_`/`into_` cost conventions, naming | `clippy::wrong_self_convention` |
| 1940 | `#[must_use]` on functions whose result is the point | `clippy::must_use_candidate` (pedantic) |
| 2383 | every suppression carries `reason = "…"` | `clippy::allow_attributes_without_reason` (deny) |
| 1946 | intra-doc links over bare paths | `rustdoc::broken_intra_doc_links` (deny) |
| 0201 | an error carries its cause (`Error::source`) | `#[source]`/`#[from]` on every wrapping variant; review-enforced |

## Type and error conventions

- `thiserror` error enums, `#[non_exhaustive]` where the API will grow; no
  `unwrap`/`expect` outside `#[cfg(test)]`; `todo!()` and `unimplemented!()`
  are banned. An error a caller branches on is a variant, never a substring.
- A value that may name a patient or a user is a `SecretString` or a
  `SecretSlice<u8>`, a written message or record is a secret too, and every
  type that holds one hand-writes a `Debug` that redacts it.
- Model closed sets as Rust `enum`s (`Outcome`, `Direction`, `Entity`,
  `NetworkAddress`, `Content`, `FeedError`).
- **No `use X as Y` import renaming.** Fix the name at its definition, or
  qualify one at the use site. (A trait import as `use Trait as _;` is not a
  rename.)
- `std::sync::LazyLock` for statics. Edition 2024, resolver 3, `cargo fmt`
  clean.
- Suppressions are `#[expect(lint, reason = "…")]` scoped to the smallest
  item (`reliability.md`).

## HTTP statuses are compared as types

An HTTP status is a `StatusCode`, and it is compared as one, never as a
number. Rendering the number (the audit line's `status` field) stays legal.

## Documentation (`missing_docs` is enforced workspace-wide)

Every public item carries a doc comment; intra-doc links resolve in the scope
of the module where the item is defined; literal square brackets are escaped.

## Edition-2024 standing guidance

- `if let` scrutinee temporaries drop before `else` (`reliability.md`).
- RPIT captures every in-scope lifetime and type parameter by default; say
  `use<…>` when a returned `impl Trait` must not.
- `Future` and `IntoFuture` are in the prelude.

## Naming

A name says what the thing is in its domain. Specification-defined things
keep the specification's names verbatim (`AuditMessage`,
`ParticipantObjectIdentification`, `purposeOfEvent`). Modules are named for
their contents.

## No Python, anywhere

The tooling languages are bash and Rust. Python is banned in scripts and
embedded in shell; reach for `jq`, `awk`, `sed`, `grep`, or a Rust tool.

## What not to do

- Do not drop a record: a record the spool cannot take is refused, so its
  caller fails the exchange it audits.
- Do not send an audit message over plain TCP outside
  `Repository::unencrypted_for_development`, or follow a redirect from a
  FHIR Feed repository.
- Do not name a product in the library: a product's event codes, its
  configuration and its routes are the product's, supplied through the
  seams.
