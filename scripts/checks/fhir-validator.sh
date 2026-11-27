#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# scripts/checks/fhir-validator.sh
#
# The BALP validation lane (#279; no specification governs the lane: our own
# design). The ehds-logging suites that write an access record as a BALP
# AuditEvent, and the ihe-atna suite that writes each other BALP pattern,
# also write it to the directory BALP_EMIT_DIR names; this script
# runs HL7's FHIR Validator over every one of them with the BALP 1.1.4
# package loaded, so each record is held to the BALP pattern it claims in
# meta.profile and every record to the FHIR R4 AuditEvent. The validator is
# a test tool, never shipped, and never the oracle: the BALP and FHIR texts
# are.
#
# The validator runs offline: no terminology server (-tx n/a) and no HTTP
# access (-no-http-access). Its jar and the packages it loads are pinned by
# version and sha256 in the "HL7 FHIR Validator" table of docs/VERSIONS.md,
# fetched into the ignored target/hl7-validator/ and checked before a run.
#
# A run fails on an issue of severity error or fatal, on a record the output
# names no outcome for, and on a claimed profile the validator could not
# find, whatever severity it reports that at, so a claim is never left
# unchecked. Warnings and information never fail a run: among them are the
# terminology messages an offline run gives for a code it cannot look up.
# Every issue is listed in summary.tsv and counted in summary.md. Example
# URLs are admitted (-allow-example-urls true): the test records name their
# hosts under example.org, which RFC 2606 reserves.
#
# Usage:
#   scripts/checks/fhir-validator.sh RECORDS OUT
#   scripts/checks/fhir-validator.sh --judge OUTPUT RECORDS OUT
#   scripts/checks/fhir-validator.sh --fetch
#   scripts/checks/fhir-validator.sh --cache-key
#   scripts/checks/fhir-validator.sh --self-test
#
# RECORDS is the directory the tests wrote (BALP_EMIT_DIR), OUT the directory
# the results go to: validation.json (the validator's output), validator.log,
# summary.tsv, summary.md, counts (two numbers: the records that passed
# and the records validated) and profiles (each meta.profile a record that
# passed claims, once, sorted). --judge reads an existing validator output.
# Needs Java 17 or later (as $JAVA or java on PATH) for a run, and curl, jq,
# tar and shasum. Exit 1 naming each failure; 0 otherwise.
set -euo pipefail
cd "$(dirname "$0")/../.."

readonly PINS="docs/VERSIONS.md"
readonly WORK="target/hl7-validator"
readonly CACHE="$WORK/home/.fhir/packages"
readonly REGISTRY="https://packages.fhir.org"
readonly AGENT="ferroaudit-checks (scripts/checks/fhir-validator.sh)"
readonly BALP="ihe.iti.balp#1.1.4"
readonly OUTCOME_FILE="http://hl7.org/fhir/StructureDefinition/operationoutcome-file"

# rows: one `<item> <version> <sha256>` line per row of the HL7 FHIR
# Validator table.
rows() {
  awk '
    /^## HL7 FHIR Validator/ { inside = 1; next }
    /^## / { inside = 0 }
    inside && /^\| `/ {
      item = $0; sub(/^\| `/, "", item); sub(/`.*/, "", item)
      pin = $0; sub(/^\| `[^`]*` \| /, "", pin); sub(/ \|.*/, "", pin)
      split(pin, part, " ")
      digest = part[3]; gsub(/`/, "", digest)
      print item, part[1], digest
    }
  ' "$PINS"
}

# sha FILE: the sha256 of FILE.
sha() {
  shasum -a 256 "$1" | cut -d' ' -f1
}

# fetch URL FILE SHA256: downloads URL to FILE unless FILE is there at
# SHA256, so a warm cache makes no request, and fails on another digest.
fetch() {
  local url="$1" file="$2" digest="$3" found
  if [[ -f "$file" ]] && [[ "$(sha "$file")" == "$digest" ]]; then
    return
  fi
  if ! curl -fsSL --retry 5 --retry-all-errors --retry-connrefused --retry-delay 2 \
    -A "$AGENT" -o "$file" "$url"; then
    rm -f "$file"
    echo "::error::cannot fetch $url" >&2
    return 1
  fi
  found="$(sha "$file")"
  if [[ "$found" != "$digest" ]]; then
    rm -f "$file"
    echo "::error::$url has sha256 $found, the pin is $digest" >&2
    return 1
  fi
}

# prepare: fetches the jar and the packages and fills the private cache.
prepare() {
  mkdir -p "$WORK" "$CACHE"
  local item version digest file n=0
  while read -r item version digest; do
    n=$((n + 1))
    if [[ "$item" == "validator_cli.jar" ]]; then
      fetch "https://github.com/hapifhir/org.hl7.fhir.core/releases/download/$version/validator_cli.jar" \
        "$WORK/validator_cli.jar" "$digest"
      continue
    fi
    file="$WORK/$item#$version.tgz"
    fetch "$REGISTRY/$item/$version" "$file" "$digest"
    if [[ ! -d "$CACHE/$item#$version/package" ]]; then
      mkdir -p "$CACHE/$item#$version"
      tar -xzf "$file" -C "$CACHE/$item#$version"
    fi
  done < <(rows)
  if [[ "$n" -eq 0 ]]; then
    echo "::error::$PINS holds no HL7 FHIR Validator table" >&2
    return 1
  fi
}

# issues OUTPUT: one "file<TAB>severity<TAB>message-id<TAB>where<TAB>text"
# line per issue of OUTPUT, an OperationOutcome or a Bundle of them, each
# naming its file in the operationoutcome-file extension.
issues() {
  jq -r '
    def ext($name): (.extension // []) | map(select((.url // "") | endswith("/" + $name))) | .[0];
    (if .resourceType == "Bundle" then [.entry[]?.resource] else [.] end)[]
    | (ext("operationoutcome-file") | .valueString // "" | split("/") | last) as $file
    | .issue[]?
    | [ $file,
        (.severity // ""),
        (ext("operationoutcome-message-id") | (.valueCode // .valueString // "")),
        (((.expression // .location // []) | .[0]) // ""),
        ((.details.text // .diagnostics // "") | gsub("[\t\r\n]"; " "))
      ]
    | @tsv' "$1"
}

# judge OUTPUT RECORDS OUT: holds OUTPUT for the records in RECORDS to the
# rules in the header, writes summary.tsv, summary.md, counts and profiles
# to OUT, and returns 1 when the run fails.
judge() {
  local output="$1" records="$2" out="$3" status=0 record base n=0 outcomes failing bad
  local file severity id where text
  mkdir -p "$out"
  if ! jq -e '.resourceType == "Bundle" or .resourceType == "OperationOutcome"' "$output" > /dev/null 2>&1; then
    echo "::error::$output is no OperationOutcome and no Bundle of them" >&2
    return 1
  fi
  {
    printf 'file\tseverity\tmessage-id\twhere\ttext\n'
    issues "$output"
  } > "$out/summary.tsv"
  outcomes="$(cut -f1 "$out/summary.tsv" | tail -n +2 | sort -u)"
  for record in "$records"/*.json; do
    [[ -f "$record" ]] || continue
    n=$((n + 1))
    base="${record##*/}"
    if ! grep -qxF "$base" <<< "$outcomes"; then
      echo "::error::the validator output names no outcome for $base" >&2
      status=1
    fi
  done
  if [[ "$n" -eq 0 ]]; then
    echo "::error::$records holds no record: did the tests run with BALP_EMIT_DIR set?" >&2
    status=1
  fi
  failing="$(awk -F '\t' '
    NR > 1 {
      unknown = ($3 ~ /^(VALIDATION_VAL_PROFILE_UNKNOWN|Validation_VAL_Profile_Unknown|Validation_VAL_Unknown_Profile|VALIDATION_VAL_GLOBAL_PROFILE_UNKNOWN)/) \
        || ($5 ~ /has not been checked because it could not be found/)
      if ($2 == "error" || $2 == "fatal" || unknown) print
    }' "$out/summary.tsv")"
  bad=0
  if [[ -n "$failing" ]]; then
    bad="$(cut -f1 <<< "$failing" | sort -u | wc -l | tr -d ' ')"
    while IFS=$'\t' read -r file severity id where text; do
      echo "::error::$file: $severity ${id:+[$id] }at ${where:-?}: $text" >&2
    done <<< "$failing"
    status=1
  fi
  printf '%d %d\n' "$((n - bad))" "$n" > "$out/counts"
  for record in "$records"/*.json; do
    [[ -f "$record" ]] || continue
    if ! cut -f1 <<< "$failing" | grep -qxF "${record##*/}"; then
      jq -r '(.meta.profile // [])[]' "$record" 2> /dev/null || true
    fi
  done | LC_ALL=C sort -u > "$out/profiles"
  {
    echo "## HL7 FHIR Validator over the BALP records"
    echo
    echo "$((n - bad)) of $n record(s) valid against $BALP and FHIR R4; a record fails on an error, a fatal issue or an unchecked profile claim."
    echo
    echo "| Severity | Issues |"
    echo "|---|---|"
    for severity in fatal error warning information; do
      printf '| %s | %d |\n' "$severity" \
        "$(awk -F '\t' -v s="$severity" 'NR > 1 && $2 == s { c++ } END { print c + 0 }' "$out/summary.tsv")"
    done
    echo
    echo "### Warnings by message id"
    echo
    awk -F '\t' 'NR > 1 && $2 == "warning" { c[($3 == "" ? "(none)" : $3)]++ } END { for (id in c) printf "- %s: %d\n", id, c[id] }' "$out/summary.tsv" | sort
    if [[ -n "$failing" ]]; then
      echo
      echo "### Failing issues"
      echo
      while IFS=$'\t' read -r file severity id where text; do
        echo "- \`$file\` $severity ${id:+\`$id\` }at \`${where:-?}\`: $text"
      done <<< "$failing"
    fi
  } > "$out/summary.md"
  cat "$out/summary.md"
  return "$status"
}

# run RECORDS OUT: prepares the validator, runs it offline over every record
# in RECORDS and judges its output.
run() {
  local records="$1" out="$2" java="${JAVA:-java}" code=0 first
  command -v "$java" > /dev/null 2>&1 || {
    echo "::error::java is not on PATH; the validator needs Java 17 or later" >&2
    return 1
  }
  prepare
  first="$(find "$records" -maxdepth 1 -name '*.json' -print -quit 2> /dev/null || true)"
  if [[ -z "$first" ]]; then
    echo "::error::$records holds no record: did the tests run with BALP_EMIT_DIR set?" >&2
    return 1
  fi
  mkdir -p "$out"
  "$java" -Xmx4g -Duser.home="$PWD/$WORK/home" -jar "$WORK/validator_cli.jar" \
    "$records" \
    -version 4.0.1 \
    -ig "$BALP" \
    -tx n/a \
    -no-http-access \
    -show-message-ids \
    -allow-example-urls true \
    -output "$out/raw.json" \
    > "$out/validator.log" 2>&1 || code=$?
  tail -n 20 "$out/validator.log" >&2
  if [[ ! -f "$out/raw.json" ]]; then
    echo "::error::the validator wrote no output (exit $code); see validator.log" >&2
    return 1
  fi
  # NOTE: the validator names no file on the outcome of a run over one record, so it takes
  # the one record's name.
  jq --arg file "$first" --arg url "$OUTCOME_FILE" '
    if .resourceType == "Bundle" then .
    else {resourceType: "Bundle", type: "collection", entry: [{resource:
      (if ((.extension // []) | map(select(.url == $url)) | length) > 0 then .
       else .extension = ((.extension // []) + [{url: $url, valueString: $file}]) end)}]}
    end' "$out/raw.json" > "$out/validation.json"
  judge "$out/validation.json" "$records" "$out"
}

# self_test: holds judge to its rules over synthetic validator outputs.
self_test() {
  local scratch
  scratch="$(mktemp -d)"
  # shellcheck disable=SC2064 # the path is fixed now, on purpose
  trap "rm -rf '$scratch'" EXIT
  mkdir -p "$scratch/records"
  echo '{"meta":{"profile":["p:a","p:both"]}}' > "$scratch/records/a.json"
  echo '{"meta":{"profile":["p:both"]}}' > "$scratch/records/b.json"
  local mid='http://hl7.org/fhir/StructureDefinition/operationoutcome-message-id'
  outcome() {
    local name="$1" severity="$2" id="$3"
    printf '{"resourceType":"OperationOutcome","extension":[{"url":"%s","valueString":"/x/%s"}],"issue":[{"extension":[{"url":"%s","valueString":"%s"}],"severity":"%s","code":"processing","details":{"text":"t"},"expression":["AuditEvent"]}]}' \
      "$OUTCOME_FILE" "$name" "$mid" "$id" "$severity"
  }
  bundle() {
    printf '{"resourceType":"Bundle","type":"collection","entry":[{"resource":%s},{"resource":%s}]}' "$1" "$2"
  }
  local cases=0 wrong=0
  check() {
    local label="$1" want="$2" counts="$3" json="$4" profiles="${5:-}" got=0
    printf '%s' "$json" > "$scratch/out.json"
    judge "$scratch/out.json" "$scratch/records" "$scratch/result" > /dev/null 2>&1 || got=1
    cases=$((cases + 1))
    if [[ "$got" != "$want" ]]; then
      echo "::error::self-test: $label: judged $got, expected $want" >&2
      wrong=1
    fi
    if [[ -n "$counts" ]] && [[ "$(cat "$scratch/result/counts")" != "$counts" ]]; then
      echo "::error::self-test: $label: counted $(cat "$scratch/result/counts"), expected $counts" >&2
      wrong=1
    fi
    if [[ -n "$profiles" ]] && [[ "$(paste -sd' ' "$scratch/result/profiles")" != "$profiles" ]]; then
      echo "::error::self-test: $label: profiles $(paste -sd' ' "$scratch/result/profiles"), expected $profiles" >&2
      wrong=1
    fi
  }
  check "information alone passes" 0 "2 2" \
    "$(bundle "$(outcome a.json information All_OK)" "$(outcome b.json information All_OK)")" "p:a p:both"
  check "a warning passes" 0 "2 2" \
    "$(bundle "$(outcome a.json warning UNKNOWN_CODESYSTEM)" "$(outcome b.json information All_OK)")"
  check "an error fails its record, and its profiles" 1 "1 2" \
    "$(bundle "$(outcome a.json error Validation_VAL_Profile_Minimum)" "$(outcome b.json information All_OK)")" "p:both"
  check "a fatal issue fails" 1 "1 2" \
    "$(bundle "$(outcome a.json fatal Error_parsing_JSON)" "$(outcome b.json information All_OK)")"
  check "an unknown profile fails at any severity" 1 "1 2" \
    "$(bundle "$(outcome a.json warning VALIDATION_VAL_PROFILE_UNKNOWN_NOT_POLICY)" "$(outcome b.json information All_OK)")"
  check "a record with no outcome fails" 1 "" \
    "$(bundle "$(outcome a.json information All_OK)" "$(outcome c.json information All_OK)")"
  check "no validator output fails" 1 "" '{"resourceType":"Parameters"}'
  rm -f "$scratch/records/"*.json
  check "no record fails" 1 "" '{"resourceType":"OperationOutcome","issue":[]}'
  if [[ "$wrong" -ne 0 ]]; then
    return 1
  fi
  echo "self-test: $cases cases judged as expected"
}

case "${1:-}" in
--self-test)
  self_test
  ;;
--fetch)
  prepare
  ;;
--cache-key)
  rows | shasum -a 256 | cut -c1-16
  ;;
--judge)
  [[ "$#" -eq 4 ]] || {
    echo "usage: $0 --judge OUTPUT RECORDS OUT" >&2
    exit 2
  }
  judge "$2" "$3" "$4"
  ;;
-* | '')
  echo "usage: $0 RECORDS OUT | --judge OUTPUT RECORDS OUT | --fetch | --cache-key | --self-test" >&2
  exit 2
  ;;
*)
  [[ "$#" -eq 2 ]] || {
    echo "usage: $0 RECORDS OUT" >&2
    exit 2
  }
  run "$1" "$2"
  ;;
esac
