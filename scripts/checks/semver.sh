#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
#
# The SemVer judge of the published crates (.claude/rules/crates-publishing.md,
# #204): cargo-semver-checks compares each crate's public API with its newest
# release on crates.io and refuses a break that the manifest version does not
# answer with a minor bump while the crate is 0.x (0.10.1 to 0.11.0). The rule
# is The Cargo Book, "SemVer Compatibility"
# (https://doc.rust-lang.org/cargo/reference/semver.html), alone: a 0.y.z
# release "can treat changes in "y" as a major release, and "z" as a minor
# release". An unchanged version is judged as a patch. A public dependency
# (scripts/checks/lib/public-deps.sh) moved to another compatibility line
# breaks the crate where the judge cannot see it, so it needs the minor too.
# No specification governs where the judge runs: our own design. CI runs it on
# every pull request and in the merge queue, and the release plan at every
# tag, all over the workspace, whose crates share one version line.
#
# Each crate is judged on more than one feature set, because the
# cargo-semver-checks README (v0.51.0) says it does not catch "breaking
# changes that exist when only a subset of all crate features are activated":
# the judge's own default (every feature but the unstable-named ones), and,
# for a crate with features, --only-explicit-features, what a dependant built
# with default-features = false compiles against.
#
#   semver.sh [<crate>...]   judge the named crates, or every crate
#   semver.sh --pin          print the cargo-semver-checks version pinned in
#                            docs/VERSIONS.md, which CI installs
#
# The installed cargo-semver-checks must be the pinned version. Exit 0 when
# every crate's version answers its change, 1 when one does not or could not
# be judged, 2 on a usage or setup error.
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
# shellcheck disable=SC1091 # shellcheck is not run with -x; the library is checked on its own
. scripts/checks/lib/public-deps.sh

usage() {
  echo "usage: semver.sh [<crate>...] | --pin" >&2
  exit 2
}

# The Pin cell of the cargo-semver-checks row, its backticks stripped.
pin="$(awk -F'|' '$2 ~ /^ *cargo-semver-checks *$/ { v = $3; gsub(/[^0-9.]/, "", v); print v; exit }' \
  docs/VERSIONS.md)"
if [[ -z "$pin" ]]; then
  echo "semver: docs/VERSIONS.md pins no cargo-semver-checks version" >&2
  exit 2
fi
if [[ "${1:-}" == "--pin" ]]; then
  [[ "$#" -eq 1 ]] || usage
  echo "$pin"
  exit 0
fi

if ! installed="$(cargo semver-checks --version 2>/dev/null)"; then
  echo "semver: cargo-semver-checks is not installed; install $pin (cargo binstall cargo-semver-checks@$pin)" >&2
  exit 2
fi
if [[ "$installed" != "cargo-semver-checks $pin" ]]; then
  echo "semver: found '$installed', docs/VERSIONS.md pins $pin" >&2
  exit 2
fi

# The `version` key of `[workspace.package]` in the root manifest.
workspace_version() {
  awk -F'"' '/^\[workspace\.package\]/{p=1; next} /^\[/{p=0} p && /^version[[:space:]]*=/{print $2; exit}' Cargo.toml
}

# The crates.io API document at path $1 under /api/v1/crates/.
crates_io() {
  curl --proto '=https' --tlsv1.2 -sSL --fail \
    -H 'User-Agent: ferroaudit-semver (scripts/checks/semver.sh)' \
    "https://crates.io/api/v1/crates/$1" 2>/dev/null
}

# The newest stable release of crate $1 on crates.io; empty when it has none.
newest_release() {
  crates_io "$1" | jq -r '.crate.max_stable_version // empty' || true
}

# Whether $1 and $2 share their compatibility line: the major, and the minor
# too while the major is 0 (the Cargo caret rule).
same_line() {
  local a="$1" b="$2"
  if [[ "${a%%.*}" == "0" ]]; then
    [[ "$(cut -d. -f1-2 <<<"$a")" == "$(cut -d. -f1-2 <<<"$b")" ]]
  else
    [[ "${a%%.*}" == "${b%%.*}" ]]
  fi
}

# The public dependency lines of crate $1 in the tree, and of release $2 of
# it on crates.io.
tree_lines() {
  manifest_requirements Cargo.toml "crates/$1/Cargo.toml" | public_lines
}
release_lines() {
  crates_io "$1/$2/dependencies" |
    jq -r '.dependencies[] | select(.kind == "normal") | "\(.crate_id) \(.req)"' | public_lines
}

# The feature selections crate $1 is judged on, one line of
# cargo-semver-checks flags each; an empty line is the judge's own default.
feature_sets() {
  echo ""
  # A crate with no features has one feature set.
  if grep -q '^\[features\]' "crates/$1/Cargo.toml"; then
    echo "--only-explicit-features"
  fi
}

crates=("$@")
if [[ "${#crates[@]}" -eq 0 ]]; then
  for manifest in crates/*/Cargo.toml; do
    crates+=("$(basename "$(dirname "$manifest")")")
  done
fi

current="$(workspace_version)"
log="$(mktemp)"
trap 'rm -f "$log"' EXIT
failed=""
for crate in "${crates[@]}"; do
  [[ -f "crates/$crate/Cargo.toml" ]] || { echo "semver: no crate crates/$crate" >&2; exit 2; }
  previous="$(newest_release "$crate")"
  if [[ -z "$previous" ]] || [[ "$previous" == "0.0.0" ]]; then
    echo "::error::semver: crates.io names no release of $crate to judge it against; see its answer above." >&2
    failed="$failed $crate"
    continue
  fi
  echo "semver: $crate $previous -> $current against crates.io"
  judged=true
  while IFS= read -r selection; do
    run=(check-release --package "$crate")
    # The selection is a list of flags that splits on whitespace by design.
    # shellcheck disable=SC2206
    [[ -z "$selection" ]] || run+=($selection)
    label="${selection:-the default feature set}"
    echo "semver: $crate on $label"
    status=0
    cargo semver-checks "${run[@]}" < /dev/null > "$log" 2>&1 || status=$?
    cat "$log"
    # Only a summary the judge printed counts as a judgement; a build that
    # fails is named apart from a break.
    if [[ "$status" -ne 0 ]] && grep -q 'semver requires new' "$log"; then
      echo "::error::semver: $crate $current breaks its public API against $previous on $label; a breaking change bumps the minor while the crate is 0.x (.claude/rules/crates-publishing.md)." >&2
      judged=false
      break
    fi
    if [[ "$status" -ne 0 ]] || ! grep -q 'Summary' "$log"; then
      echo "::error::semver: cargo-semver-checks could not judge $crate $current against $previous on $label (exit $status); see its output above." >&2
      judged=false
      break
    fi
  done < <(feature_sets "$crate")
  if [[ "$judged" != true ]]; then
    failed="$failed $crate"
    continue
  fi
  # A public dependency on another line breaks the crate where the judge
  # cannot see it, so it needs the minor.
  if ! released="$(release_lines "$crate" "$previous")"; then
    echo "::error::semver: crates.io did not answer with the dependencies of $crate $previous, so its public dependency lines are not judged." >&2
    failed="$failed $crate"
    continue
  fi
  moved="$(moved_lines "$released" "$(tree_lines "$crate")")"
  if [[ -n "$moved" ]]; then
    if same_line "$previous" "$current"; then
      echo "::error::semver: $crate moves from $previous to $current while a public dependency moves to another compatibility line ($moved); that breaks its consumers, so it bumps the minor (.claude/rules/crates-publishing.md)." >&2
      failed="$failed $crate"
    else
      echo "semver: $crate leaves its line for its public dependencies ($moved)"
    fi
  fi
done

if [[ -n "$failed" ]]; then
  echo "semver: refused:$failed" >&2
  exit 1
fi
echo "semver: every judged version answers its change"
