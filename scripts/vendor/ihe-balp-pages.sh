#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# scripts/vendor/ihe-balp-pages.sh
#
# Vendors the narrative pages of the IHE Basic Audit Log Patterns the code
# cites by section number (BALP `§1:52.1.1.1`, `§3:5.7.5.4`) into
# docs/specs/ihe-balp-pages/, beside the FHIR package of
# scripts/vendor/ihe-balp.sh: the Volume 1 and Volume 3 pages as
# profiles.ihe.net renders them at the version the package corpus pins. The
# FHIR package carries the profiles and examples, not the text the section
# numbers name.
#
# Each page is pinned by its versioned URL and sha256
# (scripts/vendor/lib/pinned.sh), and the corpus row of docs/VERSIONS.md
# carries its pin-set digest. The pages are the published implementation
# guide, licensed CC-BY-4.0 by its ImplementationGuide resource, with the
# page footer's licence link to IHE General Introduction §9. The script and
# the pins are carried from FerroHEALTH/FerroFED's
# scripts/vendor/ihe-iti-pages.sh, whose BALP corpus this is.
#
# Usage:
#   scripts/vendor/ihe-balp-pages.sh
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

ig=https://profiles.ihe.net/ITI
read_by='the section citations of crates/ihe-atna'

# licence PACKAGE VERSION: the licence statement of one IG's pages.
licence() {
  printf 'CC-BY-4.0: the license of the ImplementationGuide resource of %s %s; the page footer links IHE General Introduction §9 (https://profiles.ihe.net/GeneralIntro/ch-9.html)' "$1" "$2"
}

balp="$(licence ihe.iti.balp 1.1.4)"
pin commit volume-1.html "BALP 1.1.4, 1:52" "$ig/BALP/1.1.4/volume-1.html" \
  e2503b35872f1ea70f420ad1c5274e100f82f6282b558cc5071ca66fcc59e8c8 "$balp"
pin commit content.html "BALP 1.1.4, 3:5.7" "$ig/BALP/1.1.4/content.html" \
  a3c220e25086f97941544d9919ea521a63126023d66c27bfcd5e2e7ed17f9d09 "$balp"
pinned_corpus ihe-balp-pages "IHE BALP narrative pages" \
  "the IHE BALP 1.1.4 narrative pages" \
  "The Volume 1 page (1:52) and the Volume 3 content page (3:5.7) of the
Basic Audit Log Patterns 1.1.4 implementation guide, the text of the
package vendored in docs/specs/ihe-balp/, committed under CC-BY-4.0.
Attribution: IHE International, IT Infrastructure Technical Committee,
*Basic Audit Log Patterns (BALP)* 1.1.4." "" "$read_by"

say "done"
