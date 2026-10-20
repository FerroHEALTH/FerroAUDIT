#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# scripts/vendor/eu-ehds.sh
#
# Vendors the European Union law the ehds-logging crate implements and cites
# into docs/specs/eu-ehds/: Regulation (EU) 2025/327 on the European Health
# Data Space (the access record of Annex II 3.2 to 3.4, the categories of
# Article 14(1), the retention of Article 9(2)), Commission Implementing
# Regulation (EU) 2026/2099 on the cross-border identification and
# authentication mechanism, which the crate cites for the assurance level,
# the eHealth Network Patient Summary and General Guidelines, which it cites
# for the transformation record and the emergency access, and Commission
# Decision 2011/833/EU on the reuse of Commission documents, the reuse terms
# of all of them.
#
# Each artefact is pinned by URL and sha256 (scripts/vendor/lib/pinned.sh),
# and the corpus row of docs/VERSIONS.md carries its pin-set digest. The pins
# are carried from FerroHEALTH/FerroFED's scripts/vendor/eu.sh, whose eu-ehds
# corpus holds these five documents beside others this crate does not cite.
#
# Usage:
#   scripts/vendor/eu-ehds.sh
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

pin commit reg-eu-2025-327-en.xhtml "OJ L, 2025/327, 5.3.2025 (CELEX 32025R0327), English" \
  https://publications.europa.eu/resource/cellar/531b8c37-f962-11ef-b7db-01aa75ed71a1.0006.03/DOC_1 \
  bf331ac48264118fb0461f793123a2726da88195a11d4bccb2075fd6326909da \
  'An official EU legal act published in the Official Journal; reuse under Commission Decision 2011/833/EU of 12 December 2011 on the reuse of Commission documents' \
  'Pinned at the Publications Office (Cellar) manifestation, the XHTML EUR-Lex serves at https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=OJ:L_202500327. The EUR-Lex copy adds a monitoring script tag with per-request ids, so its sha256 differs on every fetch.'
eu_act='An official EU legal act published in the Official Journal; reuse under Commission Decision 2011/833/EU of 12 December 2011 on the reuse of Commission documents'
pin commit impl-reg-eu-2026-2099-en.xhtml "OJ L, 2026/2099, 22.9.2026 (CELEX 32026R2099), English" \
  https://publications.europa.eu/resource/cellar/dff777d4-b61d-11f1-81de-01aa75ed71a1.0006.03/DOC_1 \
  6389d047750ed8fca879d53ebeda22af444d1f1619ed3bf6f13560097aa887d0 "$eu_act" \
  'Commission Implementing Regulation (EU) 2026/2099 of 21 September 2026 on the cross-border identification and authentication mechanism, adopted under Article 16(2) of Regulation (EU) 2025/327; applies from 26 March 2027, its Article 3(3) and Article 5(2) from 26 March 2029 (its Article 9). Pinned at the Cellar manifestation the CELEX resource resolves to.'
ec_notice='A Commission document, reusable under Commission Decision 2011/833/EU (dec-eu-2011-833-en.xhtml, here) and CC BY 4.0 under the European Commission legal notice (https://commission.europa.eu/legal-notice_en): "Unless otherwise indicated (e.g. in individual copyright notices), content owned by the EU on this website is licensed under the Creative Commons Attribution 4.0 International (CC BY 4.0) licence". The PDF states no licence of its own'
pin commit ehn-guidelines-patientsummary.pdf "Release 3.4, November 2024" \
  'https://health.ec.europa.eu/document/download/e020f311-c35b-45ae-ba3d-03212b57fa65_en?filename=ehn_guidelines_patientsummary_en.pdf' \
  9daaab30ef8e8cb5f8ab2be1480d80869b17702d118ab3d13268324184267f5d "$ec_notice"
pin commit ehn-guidelines-general.pdf "Release 3.1, November 2024" \
  'https://health.ec.europa.eu/document/download/d8f7955c-4a2c-4ba2-9aea-25f3044896e8_en?filename=ehealth_health-data_electronic-exchange_general-guidelines_en.pdf' \
  041ee26c324c47a72564cb040865df2d9a7d7b25a5ed08e2e2568abc54c2198f "$ec_notice" \
  'The eHealth Network General Guidelines, Release 3.1, to which the Patient Summary and ePrescription guidelines defer.'
pin commit dec-eu-2011-833-en.xhtml "OJ L 330, 14.12.2011, p. 39 (CELEX 32011D0833), English" \
  https://publications.europa.eu/resource/cellar/cb76d4a0-c886-40bd-99d7-8db018a723d0.0010.03/DOC_1 \
  2d5bc877b9a5aad948af21c680aca1d3409f41df5dd0dcc57ef9b1b225f60982 "$eu_act" \
  'Commission Decision 2011/833/EU of 12 December 2011 on the reuse of Commission documents, the reuse terms of the acts and the guidelines here (its Articles 3 and 6), kept as their licence evidence. It replaces the Commission legal notice page, whose bytes changed with every render. Pinned at the Cellar manifestation the CELEX resource resolves to.'
pinned_corpus eu-ehds "EU EHDS Regulation" \
  "the European Health Data Space Regulation and the acts ehds-logging cites" \
  "Regulation (EU) 2025/327 on the European Health Data Space, Commission
Implementing Regulation (EU) 2026/2099 on the cross-border identification
and authentication mechanism, the eHealth Network guidelines on the Patient
Summary and the General Guidelines they defer to, and Commission Decision
2011/833/EU of 12 December 2011 on the reuse of Commission documents, which
governs the reuse of all of them. The Regulation, the implementing act and
the Decision are official EU legal acts published in the Official Journal;
the Commission legal notice (https://commission.europa.eu/legal-notice_en)
licenses the guidelines under CC BY 4.0." "" "the citations and the tests of crates/ehds-logging"

say "done"
