#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# scripts/checks/conformance.sh
#
# The self-tested conformance badges of the README (#280; no specification
# governs the badges: our own design). Each badge is a shields.io endpoint
# file under conformance/badges/ (https://shields.io/badges/endpoint-badge),
# N of M from the last CI run on main:
#
#   balp-1.1.4.json      the BALP AuditEvents the HL7 FHIR Validator found
#                        valid, from scripts/checks/fhir-validator.sh's counts
#   ehds-annex-ii.json   the logging points of Regulation (EU) 2025/327 Annex
#                        II point 3, 3.2(a) to (e), 3.3 and 3.4, whose test
#                        passed (annex_ii_<point>_*)
#   xt-ehr-d8-2.json     the six Xt-EHR D8.2 logging assertions, §5.1.8 to
#                        §5.1.12 and log-Retention of §5.1.13, whose test
#                        passed (d8_2_<assertion>_*)
#   iti-81.json          the eleven search parameters of RESTful ATNA Rev. 3.6
#                        §3.81.4.1.2 whose ITI-81 test passed
#
# The tests are read from the JUnit report of the CI conformance job's
# nextest run. A test that is missing counts as failed, so a renamed test
# lowers its badge rather than vanish from it.
#
# A badge label never claims more than a self-test shows: a label holding
# "certified", "compliant" or "conformant" fails, in a badge file or in the
# README's conformance group. The badges stay short, and the small caption
# under that group's heading says once what they count (owner decision,
# 2026-11-25); a README without it fails. Every badge, and every static
# badge in the README, carries a logo from scripts/checks/badge-logos/ (the
# EU emblem on EU blue for an EU act, else a pictogram), and one without
# fails. A badge's colour follows its count: green when all pass, yellow
# when some do, red when none do.
#
# Usage:
#   scripts/checks/conformance.sh --write JUNIT COUNTS   rewrite the badge files
#   scripts/checks/conformance.sh --check JUNIT COUNTS   fail when a badge file differs
#   scripts/checks/conformance.sh --labels               the label and logo guard alone
#   scripts/checks/conformance.sh --self-test
#
# JUNIT is the nextest JUnit report, COUNTS the validator's counts file
# ("<valid> <validated>"). Needs awk and jq.
set -euo pipefail
cd "$(dirname "$0")/../.."
# shellcheck source=scripts/checks/badge-logos/logo.sh
source scripts/checks/badge-logos/logo.sh

readonly BADGES="conformance/badges"
readonly README="README.md"
readonly FORBIDDEN='certified|compliant|conformant'

# The tests each badge counts: a name prefix per item, one item per line.
readonly ANNEX_ITEMS="balp::fields::annex_ii_3_2_a_
balp::fields::annex_ii_3_2_b_
balp::fields::annex_ii_3_2_c_
balp::fields::annex_ii_3_2_d_
balp::fields::annex_ii_3_2_e_
arr::search::annex_ii_3_3_
balp::fields::annex_ii_3_4_"
readonly D8_ITEMS="balp::fields::d8_2_log_user_access_
balp::fields::d8_2_log_api_access_
balp::fields::d8_2_log_user_id_
arr::search::d8_2_log_review_
balp::fields::d8_2_log_interoperability_
balp::fields::d8_2_log_retention_"
readonly ITI81_ITEMS="arr::search::date_
arr::search::address_
arr::search::agent_identifier_
arr::search::patient_identifier_
arr::search::entity_identifier_
arr::search::entity_type_
arr::search::entity_role_
arr::search::source_identifier_
arr::search::type_
arr::search::subtype_
arr::search::outcome_"

# outcomes JUNIT: one "<name> <pass|fail>" line per testcase of JUNIT.
outcomes() {
  awk '
    function flush() { if (name != "") print name, (failed ? "fail" : "pass"); name = ""; failed = 0 }
    /<testcase / {
      flush()
      match($0, / name="[^"]*"/)
      name = substr($0, RSTART + 7, RLENGTH - 8); failed = 0
      if ($0 ~ /\/>[[:space:]]*$/ && $0 !~ /<failure|<error/) { flush() }
      next
    }
    /<failure|<error/ { failed = 1 }
    /<\/testcase>/ { flush() }
    END { flush() }
  ' "$1"
}

# passed OUTCOMES ITEMS: how many items have at least one test, every one
# of which passed.
passed() {
  local outcomes="$1" items="$2" item n=0 found
  while IFS= read -r item; do
    [[ -n "$item" ]] || continue
    found="$(awk -v p="$item" 'index($1, p) == 1 { print $2 }' <<< "$outcomes")"
    if [[ -n "$found" ]] && ! grep -qx fail <<< "$found"; then
      n=$((n + 1))
    fi
  done <<< "$items"
  echo "$n"
}

# badge LABEL N M LOGO: the endpoint JSON of N of M, with logo LOGO.
badge() {
  local color logo
  color="$(badge_count_color "$2" "$3")"
  logo="$(badge_logo_fields "$4")"
  jq -cn --arg label "$1" --arg message "$2/$3" --arg color "$color" --argjson logo "$logo" \
    '{schemaVersion: 1, label: $label, message: $message, color: $color} + $logo'
}

# render JUNIT COUNTS DIR: writes every badge file to DIR.
render() {
  local junit="$1" counts="$2" dir="$3" outcomes valid validated
  outcomes="$(outcomes "$junit")"
  read -r valid validated < "$counts"
  mkdir -p "$dir"
  badge "IHE BALP 1.1.4 · AuditEvents valid" "$valid" "$validated" log > "$dir/balp-1.1.4.json"
  badge "EHDS Annex II 3.2 to 3.4 logging requirements" \
    "$(passed "$outcomes" "$ANNEX_ITEMS")" "$(grep -c . <<< "$ANNEX_ITEMS")" eu > "$dir/ehds-annex-ii.json"
  badge "Xt-EHR D8.2 2026-05-19 logging assertions" \
    "$(passed "$outcomes" "$D8_ITEMS")" "$(grep -c . <<< "$D8_ITEMS")" eu > "$dir/xt-ehr-d8-2.json"
  badge "IHE ITI-81 Retrieve (ARR) · RESTful ATNA Rev. 3.6 search parameters" \
    "$(passed "$outcomes" "$ITI81_ITEMS")" "$(grep -c . <<< "$ITI81_ITEMS")" magnifier > "$dir/iti-81.json"
}

# labels: fails when a badge label claims a certification or a badge carries
# no logo.
labels() {
  local bad=0 file
  for file in "$BADGES"/*.json; do
    [[ -f "$file" ]] || continue
    if jq -r .label "$file" | grep -qiE "$FORBIDDEN"; then
      echo "::error::$file: a badge label never says certified, compliant or conformant" >&2
      bad=1
    fi
  done
  if awk '/<!-- conformance:begin -->/,/<!-- conformance:end -->/' "$README" | grep -qiE "$FORBIDDEN"; then
    echo "::error::$README: the conformance group never says certified, compliant or conformant" >&2
    bad=1
  fi
  badge_caption_check "$README" || bad=1
  badge_color_check "$BADGES"/*.json || bad=1
  badge_logo_check "$BADGES"/*.json || bad=1
  badge_logo_readme "$README" || bad=1
  return "$bad"
}

# check JUNIT COUNTS: fails when a committed badge file differs from what
# --write would write.
check() {
  local scratch file status=0
  scratch="$(mktemp -d)"
  # shellcheck disable=SC2064 # the path is fixed now, on purpose
  trap "rm -rf '$scratch'" RETURN
  render "$1" "$2" "$scratch"
  for file in "$scratch"/*.json; do
    if ! cmp -s "$file" "$BADGES/${file##*/}"; then
      echo "::error::$BADGES/${file##*/} says $(jq -r .message "$BADGES/${file##*/}" 2> /dev/null || echo nothing), this run $(jq -r .message "$file"); run scripts/checks/conformance.sh --write" >&2
      status=1
    fi
  done
  labels || status=1
  return "$status"
}

# self_test: holds the counting and the label guard to their rules.
self_test() {
  local scratch wrong=0
  scratch="$(mktemp -d)"
  # shellcheck disable=SC2064 # the path is fixed now, on purpose
  trap "rm -rf '$scratch'" EXIT
  cat > "$scratch/junit.xml" << 'XML'
<testsuites>
<testsuite name="ehds-logging::it">
<testcase name="balp::fields::annex_ii_3_2_a_x" classname="ehds-logging::it" time="0.1"/>
<testcase name="balp::fields::annex_ii_3_2_b_x" classname="ehds-logging::it" time="0.1">
<failure message="boom"/>
</testcase>
<testcase name="balp::fields::annex_ii_3_2_c_x" classname="ehds-logging::it" time="0.1">
<system-out>ok</system-out>
</testcase>
</testsuite>
</testsuites>
XML
  echo "3 4" > "$scratch/counts"
  render "$scratch/junit.xml" "$scratch/counts" "$scratch/out"
  [[ "$(jq -r .message "$scratch/out/ehds-annex-ii.json")" == "2/7" ]] || {
    echo "::error::self-test: a failed or missing test counted" >&2
    wrong=1
  }
  [[ "$(jq -r .message "$scratch/out/balp-1.1.4.json")" == "3/4" ]] || {
    echo "::error::self-test: the validator counts are not the BALP badge" >&2
    wrong=1
  }
  [[ "$(jq -r .color "$scratch/out/balp-1.1.4.json")" == "yellow" ]] || {
    echo "::error::self-test: a badge short of its total is not yellow" >&2
    wrong=1
  }
  echo "0 4" > "$scratch/counts"
  render "$scratch/junit.xml" "$scratch/counts" "$scratch/zero"
  [[ "$(jq -r .color "$scratch/zero/balp-1.1.4.json")" == "red" ]] || {
    echo "::error::self-test: a badge with nothing passing is not red" >&2
    wrong=1
  }
  echo "4 4" > "$scratch/counts"
  render "$scratch/junit.xml" "$scratch/counts" "$scratch/all"
  [[ "$(jq -r .color "$scratch/all/balp-1.1.4.json")" == "brightgreen" ]] || {
    echo "::error::self-test: a badge with every case passing is not green" >&2
    wrong=1
  }
  badge_color_check "$scratch"/out/*.json "$scratch"/zero/*.json "$scratch"/all/*.json 2> /dev/null || {
    echo "::error::self-test: a rendered badge's colour does not follow its count" >&2
    wrong=1
  }
  jq -c '.color = "blue"' "$scratch/all/balp-1.1.4.json" > "$scratch/blue.json"
  if badge_color_check "$scratch/blue.json" 2> /dev/null; then
    echo "::error::self-test: a counted badge in blue passes" >&2
    wrong=1
  fi
  if jq -r .label "$scratch"/out/*.json | grep -qiE "$FORBIDDEN"; then
    echo "::error::self-test: a rendered label claims a certification" >&2
    wrong=1
  fi
  [[ "$(jq -r .logoSvg "$scratch/out/iti-81.json")" == "<svg "* ]] || {
    echo "::error::self-test: the ITI-81 badge carries no pictogram" >&2
    wrong=1
  }
  [[ "$(jq -r '.namedLogo + .labelColor' "$scratch/out/ehds-annex-ii.json")" == "europeanunion#003399" ]] || {
    echo "::error::self-test: the EHDS badge carries no EU emblem on EU blue" >&2
    wrong=1
  }
  badge_logo_check "$scratch"/out/*.json 2> /dev/null || {
    echo "::error::self-test: a rendered badge carries no logo" >&2
    wrong=1
  }
  jq -c 'del(.logoSvg)' "$scratch/out/balp-1.1.4.json" > "$scratch/bare.json"
  if badge_logo_check "$scratch/bare.json" 2> /dev/null; then
    echo "::error::self-test: a badge without a logo passes" >&2
    wrong=1
  fi
  printf '[![x](https://img.shields.io/badge/x-y-blue)](x)\n' > "$scratch/README.md"
  if badge_logo_readme "$scratch/README.md" 2> /dev/null; then
    echo "::error::self-test: a static badge without a logo passes" >&2
    wrong=1
  fi
  printf '**Conformance**<br>\n%s<br>\n<!-- conformance:begin -->\n' "$badge_caption" > "$scratch/heading.md"
  badge_caption_check "$scratch/heading.md" 2> /dev/null || {
    echo "::error::self-test: the conformance caption is not found" >&2
    wrong=1
  }
  printf '**Conformance**<br>\n<!-- conformance:begin -->\n' > "$scratch/heading.md"
  if badge_caption_check "$scratch/heading.md" 2> /dev/null; then
    echo "::error::self-test: a README without the conformance caption passes" >&2
    wrong=1
  fi
  if ! grep -qiE "$FORBIDDEN" <<< "IHE BALP Conformant"; then
    echo "::error::self-test: the guard misses a claim" >&2
    wrong=1
  fi
  [[ "$wrong" -eq 0 ]] || return 1
  echo "self-test: the badges count and the label guard holds"
}

case "${1:-}" in
--write)
  [[ "$#" -eq 3 ]] || { echo "usage: $0 --write JUNIT COUNTS" >&2; exit 2; }
  render "$2" "$3" "$BADGES"
  labels
  ;;
--check)
  [[ "$#" -eq 3 ]] || { echo "usage: $0 --check JUNIT COUNTS" >&2; exit 2; }
  check "$2" "$3"
  ;;
--labels) labels ;;
--self-test) self_test ;;
*)
  echo "usage: $0 --write JUNIT COUNTS | --check JUNIT COUNTS | --labels | --self-test" >&2
  exit 2
  ;;
esac
