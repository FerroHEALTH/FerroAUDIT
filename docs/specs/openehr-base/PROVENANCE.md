<!-- This file describes vendored third-party material; the bytes beside it
     keep their upstream licence, not the licence of this repository. -->

# Provenance: the openEHR BASE Base Types specification source

Vendored verbatim by `scripts/vendor/openehr-base.sh`. Never edit a file
here: change the pin in docs/VERSIONS.md and re-run the script.

- Source: <https://github.com/openEHR/specifications-BASE>, published at
  <https://specifications.openehr.org/releases/BASE/Release-1.1.0/>
- Pin: tag `Release-1.1.0`, which resolves to commit `a7154147fc17721fd9d575b61081a1e54113b974`; the amendment
  record of the Base Types specification heads its entries `BASE Release
  1.1.0`, and the specification declares the status STABLE
- Fetched: 2026-10-20
- Upstream licence: Creative Commons Attribution-ShareAlike 3.0 Unported, the
  repository's `LICENSE` file, vendored beside this file
- Layout: the upstream paths, unchanged
- Files: 7
- Tree digest (sha256 over the sorted per-file `sha256  path` listing,
  `PROVENANCE.md` excluded): `a28b001632572f1a0960cef14d08201d78d53979adc00495fdc64efc666f081e`
- Read by: the category map of `crates/ehds-logging`, which compares
  template and archetype ids by the case rule of §4.4 (Composite Identifiers
  and Case) and holds archetype keys to the ARCHETYPE_ID grammar of the
  identification package's Syntaxes section

## What is here

`docs/base_types/` is the Base Types specification, the AsciiDoc chapters
with their figures; `master04-identification_package.adoc` is the
identification package. `LICENSE` is the repository's licence. The other
BASE specifications, the class tables, the rendered pages and the UML files
are left out.
