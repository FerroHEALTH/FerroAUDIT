#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# The crate version rule (.claude/rules/crates-publishing.md): the workspace
# version is the library's version and the release tag, so it never moves
# backwards, Cargo.lock carries it for ihe-atna, and CHANGELOG.md has
# a section for it. A change that moves the version is a release change and
# must move all three together (no specification governs this: our own
# design).
#
# Usage: scripts/checks/crate-version-guard.sh <base-ref> [head-ref]
set -euo pipefail
cd "$(dirname "$0")/../.."

# NOTE: a reader that stops at its first match would end a `git show` pipe early, and
# under pipefail the writer's SIGPIPE fails the guard; each file is read whole first.
if [[ "${1:-}" == "--self-test" ]]; then
  guard="$(pwd)/scripts/checks/crate-version-guard.sh"
  scratch="$(mktemp -d)"
  trap 'rm -rf "$scratch"' EXIT
  mkdir -p "$scratch/scripts/checks"
  cp "$guard" "$scratch/scripts/checks/"
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

[[ "$failed" -eq 0 ]] || exit 1
echo "crate-version-guard: version $want, the lock and the changelog agree"
