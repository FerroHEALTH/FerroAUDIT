#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# Changelog structure guard (Keep a Changelog 1.1.0).
#
# Fails when any release section of CHANGELOG.md (including [Unreleased])
# violates the Keep a Changelog structure:
#   1. a duplicated `### <Type>` subsection inside one release section
#      (each type appears at most once per section: new entries merge into
#      the existing subsection, they never append a second header);
#   2. a subsection header outside the canonical type set
#      (Added / Changed / Deprecated / Removed / Fixed / Security), plus any
#      type a product adds deliberately in CHANGELOG_EXTRA_TYPES (comma-separated,
#      empty by default; e.g. "Upgrade notes");
#   3. a link reference definition that is missing, wrong or orphaned: each
#      release section links its comparison with the section below it,
#      `[X.Y.Z]: <repository>/compare/v<below>...vX.Y.Z`, the oldest its tag,
#      `<repository>/releases/tag/vX.Y.Z`, and [Unreleased] the comparison of
#      the newest release with HEAD (Keep a Changelog 1.1.0). The repository
#      is CHANGELOG_REPOSITORY, else the root Cargo.toml's `repository`.
#
#   scripts/checks/changelog-structure.sh --self-test
#      proves each rule on a fixture that passes and on one that fails.
#
# Wired into the CI changelog-guard job; runs unconditionally (the
# `no-changelog` escape label waives the entry REQUIREMENT, never the
# structural validity of the file).
set -euo pipefail

# shellcheck source=scripts/lib/guard-args.sh
. "$(dirname "$0")/../lib/guard-args.sh"
readonly GRAMMAR="[<file>] | --self-test"
guard_known_flags "$GRAMMAR" "--self-test" "$@"

# repository: the root Cargo.toml's [workspace.package] repository.
repository() {
  awk -F'"' '/^\[workspace\.package\]/{p=1; next} /^\[/{p=0} p && /^repository[[:space:]]*=/{print $2; exit}' \
    "$(dirname "$0")/../../Cargo.toml"
}

check() {
  local file="$1" repo="${CHANGELOG_REPOSITORY:-$(repository)}"
  [[ -n "$repo" ]] || { echo "changelog structure: no repository to write the compare links under" >&2; exit 2; }

# awk, not python: this repository ships no Python, and the check is a line scan
# over one file, exactly what awk is for. The quote character arrives as a
# variable because an awk string constant cannot portably escape it.
awk -v path="$file" -v q="'" -v extra="${CHANGELOG_EXTRA_TYPES:-}" -v repo="$repo" '
  BEGIN {
    allowed = "Added, Changed, Deprecated, Fixed, Removed, Security"
    n_extra = split(extra, extras, /[[:space:]]*,[[:space:]]*/)
    for (i = 1; i <= n_extra; i++) {
      sub(/^[[:space:]]+/, "", extras[i]); sub(/[[:space:]]+$/, "", extras[i])
      if (extras[i] != "") { extra_type[extras[i]] = 1; allowed = allowed ", " extras[i] }
    }
  }
  # A release heading opens a new section and resets what has been seen in it.
  /^## \[/ {
    section = $0
    sub(/^## \[/, "", section)
    sub(/\].*$/, "", section)
    delete seen
    names[++ns] = section
    next
  }
  # A link reference definition: its label and its URL, each read once.
  /^\[[^]]+\]: / {
    label = $0
    sub(/^\[/, "", label)
    sub(/\]: .*$/, "", label)
    url = $0
    sub(/^\[[^]]+\]: /, "", url)
    if (label in links) {
      errors[++n] = path ":" FNR ": a second link definition for [" label "]"
    } else {
      order[++nl] = label
      line[label] = FNR
    }
    links[label] = url
    next
  }
  # A subsection heading inside a section: check the type, then the duplicate.
  /^### / && section != "" {
    heading = $0
    sub(/^### /, "", heading)
    sub(/[[:space:]]+$/, "", heading)
    if (heading != "Added" && heading != "Changed" && heading != "Deprecated" \
        && heading != "Removed" && heading != "Fixed" && heading != "Security" \
        && !(heading in extra_type)) {
      errors[++n] = path ":" FNR ": " q "### " heading q " in [" section \
        "] is not a Keep-a-Changelog type (allowed: " allowed ")"
    }
    if (heading in seen) {
      errors[++n] = path ":" FNR ": duplicate " q "### " heading q " in [" \
        section "]: merge the entry into the existing subsection instead of " \
        "adding a second header"
    }
    seen[heading] = 1
  }
  END {
    for (i = 1; i <= ns; i++) {
      name = names[i]
      below = (i < ns) ? names[i + 1] : ""
      sectioned[name] = 1
      if (name == "Unreleased") {
        if (below == "") continue
        want = repo "/compare/v" below "...HEAD"
      } else if (below != "") {
        want = repo "/compare/v" below "...v" name
      } else {
        want = repo "/releases/tag/v" name
      }
      if (!(name in links)) {
        errors[++n] = path ": [" name "] has no link definition; add " q "[" name "]: " want q
      } else if (links[name] != want) {
        errors[++n] = path ":" line[name] ": [" name "] links " links[name] ", want " want
      }
    }
    for (i = 1; i <= nl; i++) {
      if (!(order[i] in sectioned)) {
        errors[++n] = path ":" line[order[i]] ": [" order[i] "] links a section the file does not have"
      }
    }
    if (n > 0) {
      print "changelog structure check FAILED:" > "/dev/stderr"
      for (i = 1; i <= n; i++) print "  " errors[i] > "/dev/stderr"
      exit 1
    }
    print "changelog structure OK (" path ")"
  }
' "$file"
}

# The self-test: one fixture per rule, each proven to pass or to fail.
self_test() {
  local work fails=0 rc
  work="$(mktemp -d)"
  # shellcheck disable=SC2064 # expand $work now
  trap "rm -r -- '$work'" EXIT
  local repo=https://example.test/r
  fixture() {
    printf '# Changelog\n\n## [Unreleased]\n\n## [0.2.0] - 2026-01-02\n\n### Added\n\n- b\n\n## [0.1.0] - 2026-01-01\n\n### Added\n\n- a\n\n'
  }
  links() {
    printf '[Unreleased]: %s/compare/v0.2.0...HEAD\n[0.2.0]: %s/compare/v0.1.0...v0.2.0\n[0.1.0]: %s/releases/tag/v0.1.0\n' "$repo" "$repo" "$repo"
  }
  expect() {
    local name="$1" want="$2" body="$3"
    printf '%s' "$body" > "$work/CHANGELOG.md"
    rc=0
    CHANGELOG_REPOSITORY="$repo" check "$work/CHANGELOG.md" > /dev/null 2>&1 || rc=$?
    if [[ "$rc" -ne "$want" ]]; then
      echo "self-test: $name: exit $rc (want $want)" >&2
      fails=$((fails + 1))
    fi
  }
  expect "a well-formed file" 0 "$(fixture; links)"
  expect "a duplicated type" 1 "$(fixture; printf '### Added\n\n'; links)"
  expect "a type outside the set" 1 "$(fixture | sed 's/### Added/### Notes/'; links)"
  expect "a missing link" 1 "$(fixture; links | grep -v '^\[0.2.0\]')"
  expect "a link to the wrong comparison" 1 "$(fixture; links | sed 's|v0.1.0...v0.2.0|v0.0.9...v0.2.0|')"
  expect "Unreleased behind the newest release" 1 "$(fixture; links | sed 's|v0.2.0...HEAD|v0.1.0...HEAD|')"
  expect "the oldest release compared with nothing" 1 "$(fixture; links | sed 's|releases/tag/v0.1.0|compare/v0.0.1...v0.1.0|')"
  expect "an orphaned link" 1 "$(fixture; links; printf '[0.1.5]: %s/compare/v0.1.0...v0.1.5\n' "$repo")"
  expect "a second definition" 1 "$(fixture; links; links | tail -n 1)"
  if [[ "$fails" -gt 0 ]]; then
    echo "changelog-structure self-test FAILED ($fails case(s))" >&2
    exit 1
  fi
  echo "changelog-structure self-test OK (9 cases)"
}

if [[ "${1:-}" == "--self-test" ]]; then
  self_test
else
  check "${1:-CHANGELOG.md}"
fi
