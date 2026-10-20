#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# scripts/vendor/dicom-ps3-15.sh
#
# Pins DICOM PS3.15 (Security and System Management Profiles), whose Annex
# A.5 defines the audit message an ITI-20 syslog message carries (ITI TF-2
# §3.20.3 references it), at the release named in its URL.
#
# The release is the DICOM Standard as NEMA publishes it at
# dicom.nema.org/medical/dicom/<release>/; its Notice and Disclaimer grants
# no licence to redistribute, so the page is a `cache` artefact
# (scripts/vendor/lib/pinned.sh): fetched into the git-ignored
# .vendor-cache/dicom-ps3-15/ to be read locally, never committed. The
# PROVENANCE.md names the release, the URL and the sha256, so every
# citation names the exact text. The "DICOM PS3.15" row of docs/VERSIONS.md
# carries the pin-set digest.
#
# Usage:
#   scripts/vendor/dicom-ps3-15.sh
#
# Requires: curl, shasum, awk.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$root"

# shellcheck source=scripts/vendor/lib/corpus.sh
# shellcheck disable=SC1091 # shellcheck is not run with -x; the library is checked on its own
. "$root/scripts/vendor/lib/corpus.sh"
# shellcheck source=scripts/vendor/lib/pinned.sh
# shellcheck disable=SC1091 # shellcheck is not run with -x; the library is checked on its own
. "$root/scripts/vendor/lib/pinned.sh"

corpus_require curl shasum awk
pinned_begin

pin cache part15.html "DICOM PS3.15 2026c" \
  https://dicom.nema.org/medical/dicom/2026c/output/html/part15.html \
  4cab4e7b7f5e14a43d5eb6ec13ae7c46915cef378e09c0dd75ea263827f17f32 \
  "Copyright NEMA; the publication's Notice and Disclaimer grants no licence to redistribute, so it is read locally and not committed"
pinned_corpus dicom-ps3-15 "DICOM PS3.15" \
  "DICOM PS3.15 2026c" \
  "DICOM PS3.15 2026c, Security and System Management Profiles, the single
HTML page NEMA publishes for the release. The code cites Annex A.5.1 (the
audit message schema, with the optional EventOutcomeDescription,
AuditSourceTypeCode and ParticipantObjectDataLifeCycle and the outcome
indicator 12) and Annex A.5.3 (the audit message tables that fix the EventID
and EventTypeCode of each event). Nothing of the page is committed." "" \
  "the citations of crates/ihe-atna (the audit message, its optional elements and the DICOM code tables)"

say "done"
