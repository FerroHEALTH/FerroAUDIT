#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
#
# FerroAUDIT's performance gate (#331): ferrohealth-perf in its A/B mode,
# the head of a change judged against its base measured beside it on the
# same runner, each arm the release build of tools/perf-arr, the Audit
# Record Repository harness, over that arm's `ihe-atna`.
#
#   scripts/perf/gate.sh build [BASE]
#       Builds the harness of the checkout, the head, and of its base: the
#       merge base of HEAD with BASE, a commit or ref, or with no BASE the
#       newest v* tag before HEAD, the last release, else HEAD's first
#       parent. The base is built from `git archive` of that commit under
#       target/, with its own toolchain pin, into the same target directory.
#       A base that predates the harness gets the head's copy of it, built
#       over the base's crates and workspace dependency lines.
#   scripts/perf/gate.sh run
#       Starts both harnesses, the head on 127.0.0.1:8080 and the base on
#       127.0.0.1:8081, each with its spools in a directory of its own, and
#       runs `ferrohealth-perf ab` with perf/plan.toml. Exit 0 when the gate
#       holds, 1 when it fails, 2 when the run cannot be judged or set up.
#   scripts/perf/gate.sh --self-test
#       Proves the base resolution and the bootstrap of a base without the
#       harness offline.
#
# Needs curl, git and ferrohealth-perf on PATH, on Linux, where the tool
# reads the harnesses' CPU and memory from /proc. No specification governs
# this: our own design.
set -euo pipefail

root="$(cd "$(dirname "$0")/../.." && pwd)"
readonly PLAN=perf/plan.toml
readonly HARNESS=tools/perf-arr
readonly WORK=target/perf-run
readonly BASE_TREE=target/perf-base
readonly HEAD_PORT=8080
readonly BASE_PORT=8081

die() {
  echo "perf-gate: $*" >&2
  exit 2
}

# base_rev [BASE] [AT]: the commit the base is built from, resolved from AT
# (HEAD by default).
base_rev() {
  local base="${1:-}" at="${2:-HEAD}" tag
  [[ "$base" =~ ^0+$ ]] && base=""
  if [[ -n "$base" ]]; then
    git -C "$root" merge-base "$at" "$base"
  elif tag="$(git -C "$root" describe --tags --abbrev=0 --match 'v[0-9]*' "$at^" 2> /dev/null)"; then
    git -C "$root" rev-parse "$tag^{commit}"
  else
    git -C "$root" rev-parse "$at^1"
  fi
}

# bootstrap HEAD_ROOT TREE: copies the head's harness into the base TREE,
# makes it a workspace member there, and adds each workspace dependency
# the harness names that the base's root manifest lacks, as the head's
# root manifest writes it.
bootstrap() {
  local head="$1" tree="$2" dep line
  mkdir -p "$tree/tools"
  cp -R "$head/$HARNESS" "$tree/$HARNESS"
  if ! grep -q '"tools/\*"' "$tree/Cargo.toml"; then
    sed -i.orig 's|^members = \["crates/\*"\]|members = ["crates/*", "tools/*"]|' "$tree/Cargo.toml"
    rm -- "$tree/Cargo.toml.orig"
    grep -q '"tools/\*"' "$tree/Cargo.toml" || die "the base's workspace members cannot take the harness"
  fi
  while IFS= read -r dep; do
    grep -q "^${dep} = " "$tree/Cargo.toml" && continue
    line="$(grep -m 1 "^${dep} = " "$head/Cargo.toml")" || die "the head's manifest names no workspace dependency $dep"
    awk -v line="$line" '{ print } /^\[workspace\.dependencies\]/ { print line }' "$tree/Cargo.toml" > "$tree/Cargo.toml.new"
    mv -- "$tree/Cargo.toml.new" "$tree/Cargo.toml"
  done < <(sed -n 's/^\([a-z0-9_-]*\) = { workspace = true.*/\1/p' "$tree/$HARNESS/Cargo.toml")
}

# build [BASE]: the base's release harness into target/perf-base/perf-arr,
# then the head's into target/release/perf-arr.
build() {
  local rev locked=--locked
  cd "$root"
  rev="$(base_rev "${1:-}")" || die "the base of ${1:-the previous release} cannot be resolved"
  echo "perf-gate: the base is $rev"
  rm -rf "$BASE_TREE"
  mkdir -p "$BASE_TREE/src"
  git archive "$rev" | tar -x -C "$BASE_TREE/src"
  printf '%s\n' "$rev" > "$BASE_TREE/rev"
  if [[ ! -d "$BASE_TREE/src/$HARNESS" ]]; then
    echo "perf-gate: the base predates the harness; it takes the head's"
    bootstrap "$root" "$BASE_TREE/src"
    locked=""
  fi
  (cd "$BASE_TREE/src" && cargo build --release ${locked:+"$locked"} -p perf-arr --target-dir "$root/target")
  cp target/release/perf-arr "$BASE_TREE/perf-arr"
  cargo build --release --locked -p perf-arr
}

head_pid=""
base_pid=""
cleanup() {
  local pid
  for pid in "$head_pid" "$base_pid"; do
    [[ -n "$pid" ]] && kill "$pid" 2> /dev/null || true
    [[ -n "$pid" ]] && wait "$pid" 2> /dev/null || true
  done
}

# spools: the directory both arms' spools live under. On Linux it is the
# tmpfs /dev/shm, so the runner's disk adds no noise to a comparison of
# the library's own work.
spools() {
  if [[ -d /dev/shm && -w /dev/shm ]]; then
    printf '/dev/shm/ferroaudit-perf-%s\n' "$$"
  else
    printf '%s/%s/spools\n' "$root" "$WORK"
  fi
}

# serve NAME BINARY PORT DIR: starts the arm in the background and prints
# its pid, which the tool reads its CPU and memory from.
serve() {
  local name="$1" binary="$2" port="$3" dir="$4"
  mkdir -p "$dir/$name" "$WORK/$name"
  chmod 0700 "$dir/$name"
  "$binary" "127.0.0.1:$port" "$dir/$name" > "$WORK/$name/server.log" 2>&1 &
  echo "$!"
}

# server_up NAME PORT: waits for the arm NAME to answer its probe, else
# exits 2 quoting the end of its log.
server_up() {
  for _ in $(seq 1 60); do
    curl --fail --silent "http://127.0.0.1:$2/health" > /dev/null 2>&1 && return 0
    sleep 1
  done
  die "the $1 harness did not come up: $(tail -n 5 "$WORK/$1/server.log" 2> /dev/null | tr '\n' ' ')"
}

run() {
  [[ $# -eq 0 ]] || die "usage: gate.sh run"
  cd "$root"
  command -v ferrohealth-perf > /dev/null || die "ferrohealth-perf is not on PATH"
  [[ -x target/release/perf-arr ]] || die "target/release/perf-arr is not built"
  [[ -x "$BASE_TREE/perf-arr" ]] || die "$BASE_TREE/perf-arr is not built: run gate.sh build first"
  local dir code=0 server
  dir="$(spools)"
  trap 'cleanup; rm -rf -- "$dir"' EXIT
  rm -rf "$WORK"
  mkdir -p "$WORK" "$dir"
  chmod 0700 "$WORK" "$dir"
  head_pid="$(serve head target/release/perf-arr "$HEAD_PORT" "$dir")"
  base_pid="$(serve base "$BASE_TREE/perf-arr" "$BASE_PORT" "$dir")"
  server_up head "$HEAD_PORT"
  server_up base "$BASE_PORT"
  if [[ -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
    printf "The base is \`%s\`.\n\n" "$(< "$BASE_TREE/rev")" >> "$GITHUB_STEP_SUMMARY"
  fi
  mkdir -p target/perf
  ferrohealth-perf ab --plan "$PLAN" \
    --base-url "http://127.0.0.1:$BASE_PORT" --base-pid "$base_pid" \
    --head-url "http://127.0.0.1:$HEAD_PORT" --head-pid "$head_pid" \
    --out-dir target/perf || code=$?
  if [[ "$code" -ne 0 ]]; then
    for server in head base; do
      echo "perf-gate: the $server harness's log follows" >&2
      tail -n 50 "$WORK/$server/server.log" >&2
    done
  fi
  exit "$code"
}

self_test() {
  local work n=0 failed=0
  work="$(mktemp -d)"
  # shellcheck disable=SC2064 # expand $work now
  trap "rm -r -- '$work'" EXIT
  check() {
    n=$((n + 1))
    if [[ "$2" != "$3" ]]; then
      echo "  FAIL: $1: expected [$2], got [$3]" >&2
      failed=1
    fi
  }
  local head="$work/head" tree="$work/base"
  mkdir -p "$head/$HARNESS" "$tree"
  printf '[workspace]\nmembers = ["crates/*", "tools/*"]\n\n[workspace.dependencies]\naxum = { version = "0.8.9" }\ntokio = "1.0.0"\n' > "$head/Cargo.toml"
  printf '[package]\nname = "perf-arr"\n\n[dependencies]\naxum = { workspace = true }\ntokio = { workspace = true, features = ["net"] }\n' > "$head/$HARNESS/Cargo.toml"
  printf '[workspace]\nmembers = ["crates/*"]\n\n[workspace.dependencies]\ntokio = "0.9.0"\n' > "$tree/Cargo.toml"
  bootstrap "$head" "$tree"
  check "the harness is copied into the base" 1 "$([[ -f "$tree/$HARNESS/Cargo.toml" ]] && echo 1 || echo 0)"
  check "the base's workspace takes the harness" 1 "$(grep -c '^members = \["crates/\*", "tools/\*"\]$' "$tree/Cargo.toml")"
  check "a dependency the base lacks comes from the head" 1 "$(grep -c '^axum = { version = "0.8.9" }$' "$tree/Cargo.toml")"
  check "a dependency the base has keeps the base's line" 'tokio = "0.9.0"' "$(grep '^tokio = ' "$tree/Cargo.toml")"
  bootstrap "$head" "$tree"
  check "a second bootstrap changes nothing" 1 "$(grep -c '^axum = ' "$tree/Cargo.toml")"
  check "the plan and the harness are where the gate reads them" 1 "$([[ -f "$root/$PLAN" && -f "$root/$HARNESS/Cargo.toml" ]] && echo 1 || echo 0)"
  local repo="$work/repo" saved_root="$root"
  git init --quiet "$repo"
  commit() { git -C "$repo" -c user.name=t -c user.email=t@t -c commit.gpgsign=false commit --quiet --allow-empty -m "$1"; }
  commit one
  commit two
  git -C "$repo" -c tag.gpgsign=false tag v0.0.1
  git -C "$repo" -c tag.gpgsign=false tag vendor-snapshot
  commit three
  git -C "$repo" branch side
  commit four
  git -C "$repo" checkout --quiet side
  commit side-only
  git -C "$repo" checkout --quiet -
  root="$repo"
  check "with no BASE the base is the newest release tag before HEAD" \
    "$(git -C "$repo" rev-parse v0.0.1)" "$(base_rev "")"
  check "a BASE names its merge base with HEAD" \
    "$(git -C "$repo" rev-parse HEAD~1)" "$(base_rev side)"
  check "an all-zero BASE takes the previous release" \
    "$(git -C "$repo" rev-parse v0.0.1)" "$(base_rev 0000000000000000000000000000000000000000)"
  check "with no tag before it the base is the first parent" \
    "$(git -C "$repo" rev-parse v0.0.1~1)" "$(base_rev "" v0.0.1)"
  root="$saved_root"
  [[ "$failed" -eq 0 ]] || exit 1
  echo "perf-gate: self-test OK ($n checks)."
}

case "${1:-}" in
  build)
    shift
    build "$@"
    ;;
  run)
    shift
    run "$@"
    ;;
  --self-test) self_test ;;
  *) die "usage: gate.sh build [BASE] | run | --self-test" ;;
esac
