#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# scripts/checks/versions.sh
#
# Version-drift guard: docs/VERSIONS.md is the single source of truth, and
# every file that repeats a pin must agree with it. The guard runs offline.
#
#   1. ITI-20 and RESTful ATNA  the page, its figure and the supplement
#                        hash to the sha256 values their rows pin, which the
#                        PROVENANCE.md repeats, and the tree still hashes to
#                        the digest it records.
#   2. the BALP package  the PROVENANCE.md names the pinned version and
#                        tarball sha256, and the tree still hashes to the
#                        digest it records.
#   3. the pinned corpora  the BALP pages, the EHDS acts, the four packages
#                        the categories are read against, the IETF RFCs and
#                        DICOM PS3.15: each PROVENANCE.md names its row's
#                        pin-set digest, and each tree still hashes to the
#                        digest it records.
#   3b. openEHR BASE     the PROVENANCE.md names the pinned tag, and the
#                        tree still hashes to the digest it records.
#   4. the design        the pin table of docs/architecture.md names the
#                        same BALP version as the matrix.
#   5. the licence       no first-party file claims a licence other than
#                        BUSL-1.1 in its SPDX header; the vendored trees are
#                        third-party material and are left out.
#
# Usage:
#   scripts/checks/versions.sh
#
# Requires: shasum, git.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

# shellcheck source=scripts/vendor/lib/corpus.sh
# shellcheck disable=SC1091 # shellcheck is not run with -x; the library is checked on its own
. "$root/scripts/vendor/lib/corpus.sh"

corpus_require shasum git

failures=0
fail() {
  printf 'versions: %s\n' "$*" >&2
  failures=$((failures + 1))
}
ok() { printf 'versions: ok: %s\n' "$*"; }

# The value after `- <Label>:` in a PROVENANCE.md, backticks removed. A
# label whose value wraps to the next line (the digests) is read as the
# first backticked token on the following line.
provenance_line() {
  local file="$1" label="$2" value
  value="$(awk -v label="- $label:" '
    index($0, label) == 1 {
      v = substr($0, length(label) + 1)
      gsub(/`/, "", v); gsub(/^[[:space:]]+|[[:space:]]+$/, "", v)
      print v; exit
    }' "$file")"
  printf '%s\n' "$value"
}

provenance_digest() {
  local file="$1" label="$2"
  awk -v label="- $label" '
    found { if (match($0, /`[0-9a-f]{64}`/)) { print substr($0, RSTART + 1, 64); exit } }
    index($0, label) == 1 { found = 1; if (match($0, /`[0-9a-f]{64}`/)) { print substr($0, RSTART + 1, 64); exit } }
  ' "$file"
}

# The sha256 tokens of a pin cell, in the order the cell names them.
hashes() {
  awk '{ for (i = 1; i <= NF; i++) { t = $i; gsub(/[`,.;:]/, "", t); if (t ~ /^[0-9a-f]{64}$/) print t } }' <<< "$1"
}

# tree_at DIR: the tree of DIR hashes to the digest its PROVENANCE.md records.
tree_at() {
  local dir="$1" want got
  want="$(provenance_digest "$dir/PROVENANCE.md" "Tree digest")"
  got="$(corpus_tree_digest "$dir")"
  [ "$got" = "$want" ] || fail "$dir hashes to $got, its PROVENANCE.md records ${want:-nothing}"
}

# ---- 1. ITI-20 and RESTful ATNA ---------------------------------------------
atna="docs/specs/ihe-atna"
if [ -f "$atna/PROVENANCE.md" ]; then
  page_pin="$(corpus_pin_cell "IHE ITI-20 Record Audit Event")"
  supplement_pin="$(corpus_pin_cell "IHE RESTful ATNA supplement")"
  for pair in \
    "Volume2/ITI-20.html:$(hashes "$page_pin" | sed -n 1p)" \
    "Volume2/media/Figure_3.20.4-1.png:$(hashes "$page_pin" | sed -n 2p)" \
    "IHE_ITI_Suppl_RESTful-ATNA.pdf:$(hashes "$supplement_pin" | sed -n 1p)"; do
    file="${pair%%:*}"
    want="${pair#*:}"
    if [ ! -f "$atna/$file" ]; then
      fail "$atna/$file is missing"
      continue
    fi
    got="$(corpus_sha256 "$atna/$file")"
    [ "$got" = "$want" ] || fail "$atna/$file hashes to $got, docs/VERSIONS.md pins $want"
    grep -F "| \`$file\` |" "$atna/PROVENANCE.md" | grep -qF "\`$want\`" ||
      fail "$atna/PROVENANCE.md does not record $file at $want"
  done
  tree_at "$atna"
  ok "ITI-20 and the RESTful ATNA supplement at their checksums"
else
  fail "$atna/PROVENANCE.md is missing"
fi

# ---- 2. the BALP package ----------------------------------------------------
balp="docs/specs/ihe-balp"
if [ -f "$balp/PROVENANCE.md" ]; then
  pin="$(corpus_pin_cell "IHE BALP FHIR package")"
  version="$(awk '{ for (i = 1; i < NF; i++) if ($i == "version") { v = $(i + 1); gsub(/[`,.;:]+$/, "", v); gsub(/`/, "", v); print v; exit } }' <<< "$pin")"
  sha="$(hashes "$pin" | sed -n 1p)"
  got_pin="$(provenance_line "$balp/PROVENANCE.md" Pin)"
  [ "$got_pin" = "package ihe.iti.balp version $version, tarball sha256 $sha" ] ||
    fail "$balp/PROVENANCE.md names '$got_pin', docs/VERSIONS.md pins version $version at $sha"
  tree_at "$balp"
  ok "the BALP package $version at its digest"
else
  fail "$balp/PROVENANCE.md is missing"
fi

# ---- 3. the corpora pinned file by file -------------------------------------
# pinned_at DIR ITEM: the PROVENANCE.md of DIR names the pin-set digest of the
# ITEM row, and the tree still hashes to the digest it records.
pinned_at() {
  local dir="$1" item="$2" want got
  if [ ! -f "$dir/PROVENANCE.md" ]; then
    fail "$dir/PROVENANCE.md is missing"
    return 0
  fi
  want="$(hashes "$(corpus_pin_cell "$item")" | sed -n 1p)"
  got="$(provenance_digest "$dir/PROVENANCE.md" "Pin-set digest")"
  [ "$got" = "$want" ] || fail "$dir/PROVENANCE.md records pin-set digest ${got:-nothing}, docs/VERSIONS.md $want"
  tree_at "$dir"
  ok "$item at pin-set digest $want"
}

pinned_at docs/specs/ihe-balp-pages "IHE BALP narrative pages"
pinned_at docs/specs/eu-ehds "EU EHDS Regulation"
pinned_at docs/specs/eu-hl7-health-data-api "HL7 Europe EU Health Data API"
pinned_at docs/specs/eu-hl7-imaging "HL7 Europe Imaging Report"
pinned_at docs/specs/eu-hl7-hdr "HL7 Europe Hospital Discharge Report"
pinned_at docs/specs/ehdsi-mvc "MyHealth@EU Master Value Sets Catalogue"
pinned_at docs/specs/ietf-syslog "IETF syslog and audit RFCs"
pinned_at docs/specs/dicom-ps3-15 "DICOM PS3.15"

# ---- 3b. openEHR BASE --------------------------------------------------------
base="docs/specs/openehr-base"
if [ -f "$base/PROVENANCE.md" ]; then
  tag="$(corpus_pin_tag "$(corpus_pin_cell "openEHR BASE specification source")")"
  grep -qF "Pin: tag \`$tag\`" "$base/PROVENANCE.md" ||
    fail "$base/PROVENANCE.md does not name tag $tag, which docs/VERSIONS.md pins"
  tree_at "$base"
  ok "openEHR BASE $tag at its digest"
else
  fail "$base/PROVENANCE.md is missing"
fi

# ---- 4. the design ----------------------------------------------------------
architecture="docs/architecture.md"
if [ -f "$architecture" ]; then
  matrix="$(grep -oE '[0-9]+\.[0-9]+\.[0-9]+' <<< "$(corpus_pin_cell "BALP")" | head -n1)"
  design="$(awk -F'|' '
    NF >= 3 {
      k = $2; v = $3
      gsub(/`/, "", k); gsub(/^[[:space:]]+|[[:space:]]+$/, "", k)
      if (k == "BALP") { print v; exit }
    }' "$architecture" | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n1)"
  [ "$design" = "$matrix" ] ||
    fail "$architecture pins BALP '$design', docs/VERSIONS.md $matrix"
  ok "$architecture agrees on BALP $matrix"
else
  fail "$architecture is missing"
fi

# ---- 5. the licence ---------------------------------------------------------
licence="$(awk '{ print $1; exit }' <<< "$(corpus_pin_cell "Project licence")")"
while IFS= read -r path; do
  claimed="$(grep -m1 -oE 'SPDX-License-Identifier: [A-Za-z0-9.+-]+' "$path" | cut -d' ' -f2 || true)"
  [ -z "$claimed" ] || [ "$claimed" = "$licence" ] ||
    fail "$path claims $claimed, the project licence is $licence"
done < <(git ls-files -z | tr '\0' '\n' | grep -v '^docs/specs/' | grep -v '^LICENSE$')
ok "every first-party SPDX header names $licence"

if [ "$failures" -gt 0 ]; then
  printf 'versions: %d failure(s)\n' "$failures" >&2
  exit 1
fi
printf 'versions: every pin agrees with docs/VERSIONS.md\n'
