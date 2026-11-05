#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# scripts/vendor/eu-gdpr.sh
#
# Vendors the General Data Protection Regulation the ehds-logging crate cites
# into docs/specs/eu-gdpr/: Regulation (EU) 2016/679 as published in the
# Official Journal, which the crate cites for the legal ground of an access
# and the record of processing (its Articles 5(2), 6(1), 9(2) and 30(1)(b)),
# and the English corrigendum of 23 May 2018, which corrects that text.
#
# Each artefact is pinned by its Publications Office (Cellar) URL and sha256
# (scripts/vendor/lib/pinned.sh), the XHTML manifestation the CELEX resource
# resolves to, and the "EU GDPR" row of docs/VERSIONS.md carries the pin-set
# digest. Their reuse terms are Commission Decision 2011/833/EU, vendored in
# docs/specs/eu-ehds/ by scripts/vendor/eu-ehds.sh.
#
# Usage:
#   scripts/vendor/eu-gdpr.sh
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

cellar=https://publications.europa.eu/resource/cellar
eu_act='An official EU legal act published in the Official Journal; reuse under Commission Decision 2011/833/EU of 12 December 2011 on the reuse of Commission documents (docs/specs/eu-ehds/dec-eu-2011-833-en.xhtml)'

pin commit reg-eu-2016-679-en.xhtml "OJ L 119, 4.5.2016, p. 1 (CELEX 32016R0679), English" \
  "$cellar/3e485e15-11bd-11e6-ba9a-01aa75ed71a1.0006.03/DOC_1" \
  962539af03738bf552319ff4ce42d69e5f95a576307c4dfed7bf87e81b646b9d "$eu_act" \
  'Regulation (EU) 2016/679 of the European Parliament and of the Council of 27 April 2016 (General Data Protection Regulation), recitals included; applies from 25 May 2018 (its Article 99(2)). Pinned at the Cellar manifestation the CELEX resource resolves to.'
pin commit reg-eu-2016-679-corrigendum-2018-05-23-en.xhtml "OJ L 127, 23.5.2018, p. 2 (CELEX 32016R0679R(02)), English" \
  "$cellar/683d5816-5e52-11e8-ab9c-01aa75ed71a1.0006.03/DOC_1" \
  292bd10918681ec090f23eafbc53afd4dd3bbb7cb976f6748df4a6254f2be54a "$eu_act" \
  'The English corrigendum to Regulation (EU) 2016/679, read with the Regulation wherever it corrects a cited provision. Pinned at the Cellar manifestation the CELEX resource resolves to.'
pinned_corpus eu-gdpr "EU GDPR" \
  "the General Data Protection Regulation ehds-logging cites" \
  "Regulation (EU) 2016/679 (the General Data Protection Regulation) as
published in the Official Journal, and its English corrigendum of 23 May
2018. Both are official EU legal acts published in the Official Journal,
reusable under Commission Decision 2011/833/EU of 12 December 2011 on the
reuse of Commission documents, vendored as
docs/specs/eu-ehds/dec-eu-2011-833-en.xhtml." "" \
  "the citations of crates/ehds-logging (the legal ground and the record of processing of the access record, FerroHEALTH/FerroAUDIT#140)"

say "done"
