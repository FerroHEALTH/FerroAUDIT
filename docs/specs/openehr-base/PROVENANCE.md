<!-- This file describes vendored third-party material; the bytes beside it
     keep their upstream licence, not the licence of this repository. -->

# Provenance: the openEHR BASE Base Types specification source

Vendored verbatim by `scripts/vendor/openehr-base.sh`. Never edit a file
here: change the pin in docs/VERSIONS.md and re-run the script.

- Source: <https://github.com/openEHR/specifications-BASE>, published at
  <https://specifications.openehr.org/releases/BASE/development/>
- Pin: commit `707b8ac28cea1ccc4410f8e930665d473475350f`, the commit FerroHEALTH/FerroEHR vendors; the
  amendment record of the Base Types specification heads its entries
  `BASE Release 1.3.0 (unreleased)`, and the specification declares the status
  STABLE
- Fetched: 2026-10-21
- Upstream licence: Creative Commons Attribution-ShareAlike 3.0 Unported, the
  repository's `LICENSE` file, vendored beside this file
- Layout: the upstream paths, unchanged
- Files: 10
- Tree digest (sha256 over the sorted per-file `sha256  path` listing,
  `PROVENANCE.md` excluded): `dbdfd3de8ae1865b4f0065b34b163a8d59c73d6be9cc5ec821aeb040d372528d`
- Read by: the category map of `crates/ehds-logging`, which compares
  template and archetype ids by the case rule of Composite Identifiers
  and Case and holds archetype keys to the ARCHETYPE_ID grammar of the
  identification package's Syntaxes section

## What is here

`docs/base_types/` is the Base Types specification, the AsciiDoc chapters
with their figures; `master05-identification_package.adoc` is the
identification package. `LICENSE` is the repository's licence. The other
BASE specifications, the class tables, the rendered pages and the UML files
are left out.
