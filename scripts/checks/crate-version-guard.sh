#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# The crate version rule (.claude/rules/crates-publishing.md): the workspace
# version is the library's version and the release tag, so it never moves
# backwards, Cargo.lock carries it for ihe-atna, and CHANGELOG.md has
# a section for it. A change that moves the version is a release change and
# must move all three together (no specification governs this: our own
# design). A public dependency (scripts/checks/lib/public-deps.sh) moved to
# another compatibility line breaks the crates that require it, so a change
# that moves one moves the version to a later minor at patch 0 (#204).
#
# Usage: scripts/checks/crate-version-guard.sh <base-ref> [head-ref]
set -euo pipefail
cd "$(dirname "$0")/../.."
# shellcheck disable=SC1091 # shellcheck is not run with -x; the library is checked on its own
. scripts/checks/lib/public-deps.sh

# NOTE: a reader that stops at its first match would end a `git show` pipe early, and
# under pipefail the writer's SIGPIPE fails the guard; each file is read whole first.
if [[ "${1:-}" == "--self-test" ]]; then
  guard="$(pwd)/scripts/checks/crate-version-guard.sh"
  scratch="$(mktemp -d)"
  trap 'rm -rf "$scratch"' EXIT
  mkdir -p "$scratch/scripts/checks/lib"
  cp "$guard" "$scratch/scripts/checks/"
  cp "$(pwd)/scripts/checks/lib/public-deps.sh" "$scratch/scripts/checks/lib/"
  cd "$scratch"
  git init -q -b main
  git -c user.email=t@example.org -c user.name=t commit -q --allow-empty -m base
  printf '[workspace.package]\nversion = "0.9.0"\n' > Cargo.toml
  {
    printf '[[package]]\nname = "ihe-atna"\nversion = "0.9.0"\n'
    for i in $(seq 1 4000); do printf '\n[[package]]\nname = "filler-%s"\nversion = "1.0.0"\n' "$i"; done
  } > Cargo.lock
  {
    printf '# Changelog\n\n## [0.9.0] - 2026-10-27\n'
    for i in $(seq 1 4000); do printf -- '- An entry long enough to fill the pipe buffer, number %s.\n' "$i"; done
  } > CHANGELOG.md
  git add -A
  git -c user.email=t@example.org -c user.name=t commit -q -m "release 0.9.0"
  for run in 1 2 3 4 5; do
    bash scripts/checks/crate-version-guard.sh main~1 HEAD > /dev/null ||
      { echo "crate-version-guard self-test: run $run refused files larger than a pipe buffer" >&2; exit 1; }
  done
  printf '# Changelog\n\n## [0.8.0] - 2026-10-01\n' > CHANGELOG.md
  git -c user.email=t@example.org -c user.name=t commit -q -am "drop the section"
  if bash scripts/checks/crate-version-guard.sh main~1 HEAD > /dev/null 2>&1; then
    echo "crate-version-guard self-test: a missing changelog section passed" >&2
    exit 1
  fi
  # The #204 cases: a public dependency moved to another compatibility line
  # breaks the crate, so the version moves to a later minor at patch 0.
  release() {
    printf '[workspace.package]\nversion = "%s"\n\n[workspace.dependencies]\nferrohealth-smart = "%s"\nserde = "%s"\n' "$1" "$2" "$3" > Cargo.toml
    mkdir -p crates/demo
    printf '[package]\nname = "demo"\nversion.workspace = true\n\n[dependencies]\nferrohealth-smart = { workspace = true, optional = true }\nserde.workspace = true\n' > crates/demo/Cargo.toml
    printf '[[package]]\nname = "ihe-atna"\nversion = "%s"\n' "$1" > Cargo.lock
    printf '# Changelog\n\n## [%s] - 2026-11-10\n' "$1" > CHANGELOG.md
    git add -A
    git -c user.email=t@example.org -c user.name=t commit -q -m "$4"
  }
  case_of() {
    local want="$1" label="$2" status=0
    bash scripts/checks/crate-version-guard.sh public-base HEAD > /dev/null 2>&1 || status=$?
    if [[ "$status" -ne "$want" ]]; then
      echo "crate-version-guard self-test: $label: exit $status, want $want" >&2
      exit 1
    fi
  }
  release 0.10.1 0.16.0 1.0.0 "a public dependency"
  git branch public-base
  release 0.10.1 0.17.0 1.0.0 "a public dependency on another line, same version"
  case_of 1 "a public dependency on another line without a bump"
  release 0.10.2 0.17.0 1.0.0 "a public dependency on another line, a patch"
  case_of 1 "a public dependency on another line with a patch step"
  release 0.11.0 0.17.0 1.0.0 "a public dependency on another line, a minor"
  case_of 0 "a public dependency on another line with a minor step"
  release 0.11.1 0.17.0 1.0.0 "a public dependency on another line, a minor keeping a patch"
  case_of 1 "a public dependency on another line with a minor step that keeps a patch"
  release 0.10.1 0.16.4 2.0.0 "a compatible public and an incompatible private dependency"
  case_of 0 "a private dependency on another line without a bump"
  echo "crate-version-guard self-test: OK"
  exit 0
fi

base="${1:?usage: crate-version-guard.sh <base-ref> [head-ref] | --self-test}"
head="${2:-HEAD}"

# The `version` key of `[workspace.package]` in a Cargo.toml read on stdin.
workspace_version() {
  awk '/^\[workspace\.package\]/{p=1; next} /^\[/{p=0}
       p && /^version[[:space:]]*=/{gsub(/.*=[[:space:]]*"|".*/, ""); print; exit}'
}

# The version of the ihe-atna package in a Cargo.lock read on stdin.
locked_version() {
  awk '/^name = "ihe-atna"$/{p=1; next}
       p && /^version = /{gsub(/version = "|"/, ""); print; exit}'
}

manifest="$(git show "$head:Cargo.toml")"
# The base has no manifest before the workspace's first change.
base_manifest="$(git show "$base:Cargo.toml" 2> /dev/null || true)"
lock="$(git show "$head:Cargo.lock")"
changelog="$(git show "$head:CHANGELOG.md")"

want="$(workspace_version <<< "$manifest")"
was="$(workspace_version <<< "$base_manifest")"
locked="$(locked_version <<< "$lock")"

failed=0
if [[ -z "$want" ]]; then
  echo "crate-version-guard: Cargo.toml declares no [workspace.package] version" >&2
  exit 1
fi

if [[ -n "$was" && "$was" != "$want" ]]; then
  newest="$(printf '%s\n%s\n' "$was" "$want" | sort -V | tail -1)"
  if [[ "$newest" != "$want" ]]; then
    echo "crate-version-guard: the version moves backwards, $was to $want" >&2
    failed=1
  fi
fi

if [[ "$locked" != "$want" ]]; then
  echo "crate-version-guard: Cargo.lock holds ihe-atna ${locked:-<none>}, the workspace $want; run cargo update -w" >&2
  failed=1
fi

if ! grep -qF "## [$want]" <<< "$changelog"; then
  echo "crate-version-guard: CHANGELOG.md has no section for $want" >&2
  failed=1
fi

# The line a version sits on, as a caret requirement names it: `0.Y` while
# the major is 0, the major otherwise.
version_line() {
  if [[ "${1%%.*}" == "0" ]]; then cut -d. -f1-2 <<<"$1"; else echo "${1%%.*}"; fi
}

# A public dependency required on another compatibility line breaks every
# crate that requires it, so the version moves to a later minor at patch 0.
if [[ -n "$was" ]]; then
  for member in $(git ls-tree -r --name-only "$head" -- crates | grep '^crates/[^/]*/Cargo.toml$' || true); do
    git cat-file -e "$base:$member" 2> /dev/null || continue
    moved="$(moved_lines \
      "$(manifest_requirements <(printf '%s\n' "$base_manifest") <(git show "$base:$member") | public_lines)" \
      "$(manifest_requirements <(printf '%s\n' "$manifest") <(git show "$head:$member") | public_lines)")"
    [[ -n "$moved" ]] || continue
    if [[ "$(version_line "$was")" == "$(version_line "$want")" ]] || [[ "${want##*.}" != "0" ]]; then
      echo "crate-version-guard: $member requires a public dependency on another compatibility line ($moved), which breaks its consumers; the version moves from $was to a later minor at patch 0, not $want (.claude/rules/crates-publishing.md)" >&2
      failed=1
    fi
  done
fi

[[ "$failed" -eq 0 ]] || exit 1
echo "crate-version-guard: version $want, the lock and the changelog agree"
