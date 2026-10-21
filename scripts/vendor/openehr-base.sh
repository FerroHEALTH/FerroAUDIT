#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# scripts/vendor/openehr-base.sh
#
# Vendors the Base Types specification of the openEHR BASE component into
# docs/specs/openehr-base/: its AsciiDoc chapters, with the identification
# package that fixes the case rule of composite identifiers and the
# ARCHETYPE_ID grammar the category map of ehds-logging holds its keys to,
# and the repository's licence file, from the commit the "openEHR BASE
# specification source" row of docs/VERSIONS.md pins. The commit is the one
# FerroHEALTH/FerroEHR vendors, whose archetype-id parser the map's check is
# held to, so the two trees hold the same bytes for these paths. The other
# BASE specifications, the class tables and the rendered pages are left out:
# nothing here cites them.
#
# Usage:
#   scripts/vendor/openehr-base.sh
#
# Requires: curl, tar, shasum, jq.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

# shellcheck source=scripts/vendor/lib/corpus.sh
# shellcheck disable=SC1091 # shellcheck is not run with -x; the library is checked on its own
. "$root/scripts/vendor/lib/corpus.sh"

corpus_require curl tar shasum jq

dest="docs/specs/openehr-base"
paths=("docs/base_types" "LICENSE")

pin="$(corpus_pin_cell "openEHR BASE specification source")"
repo="$(corpus_pin_repo "$pin")"
commit="$(corpus_pin_commit "$pin")"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

say "$repo at commit $commit"
tree_root="$(corpus_fetch "$repo" "$commit" "$tmp")"

rm -rf "$dest"
mkdir -p "$dest"
corpus_take "$tree_root" "$dest" "${paths[@]}"
dropped="$(corpus_drop_manifests "$dest")"
[ -z "$dropped" ] || die "the release carries a dependency manifest: $dropped"

identification="$dest/docs/base_types/master05-identification_package.adoc"
grep -q '==== Composite Identifiers and Case' "$identification" ||
  die "the release states no case rule for composite identifiers"
grep -q '^archetype_id ' "$identification" ||
  die "the release gives no ARCHETYPE_ID grammar"
grep -q 'Attribution-ShareAlike 3.0' "$dest/LICENSE" ||
  die "the LICENSE is not Creative Commons Attribution-ShareAlike 3.0"
release="$(sed -nE 's/^.*\*BASE Release ([0-9.]+)( \(unreleased\))?\*.*$/\1\2/p' \
  "$dest/docs/base_types/master00-amendment_record.adoc" | head -n1)"
[ -n "$release" ] || die "the Base Types amendment record heads no BASE Release"
grep -qF "$release" <<< "$pin" ||
  die "the pin names no BASE Release $release, the head of the amendment record"
status="$(sed -nE 's/^:spec_status:[[:space:]]*([A-Z_]+).*/\1/p' "$dest/docs/base_types/master.adoc" "$dest/docs/base_types/manifest_vars.adoc" | head -n1)"
[ -n "$status" ] || die "docs/base_types declares no spec_status"

files="$(corpus_file_count "$dest")"
digest="$(corpus_tree_digest "$dest")"
fetched="$(corpus_fetched)"

cat > "$dest/PROVENANCE.md" << PROV
<!-- This file describes vendored third-party material; the bytes beside it
     keep their upstream licence, not the licence of this repository. -->

# Provenance: the openEHR BASE Base Types specification source

Vendored verbatim by \`scripts/vendor/openehr-base.sh\`. Never edit a file
here: change the pin in docs/VERSIONS.md and re-run the script.

- Source: <https://github.com/$repo>, published at
  <https://specifications.openehr.org/releases/BASE/development/>
- Pin: commit \`$commit\`, the commit FerroHEALTH/FerroEHR vendors; the
  amendment record of the Base Types specification heads its entries
  \`BASE Release $release\`, and the specification declares the status
  $status
- Fetched: $fetched
- Upstream licence: Creative Commons Attribution-ShareAlike 3.0 Unported, the
  repository's \`LICENSE\` file, vendored beside this file
- Layout: the upstream paths, unchanged
- Files: $files
- Tree digest (sha256 over the sorted per-file \`sha256  path\` listing,
  \`PROVENANCE.md\` excluded): \`$digest\`
- Read by: the category map of \`crates/ehds-logging\`, which compares
  template and archetype ids by the case rule of Composite Identifiers
  and Case and holds archetype keys to the ARCHETYPE_ID grammar of the
  identification package's Syntaxes section

## What is here

\`docs/base_types/\` is the Base Types specification, the AsciiDoc chapters
with their figures; \`master05-identification_package.adoc\` is the
identification package. \`LICENSE\` is the repository's licence. The other
BASE specifications, the class tables, the rendered pages and the UML files
are left out.
PROV

say "$files files, tree digest $digest"
say "done"
