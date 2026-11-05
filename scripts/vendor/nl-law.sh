#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# scripts/vendor/nl-law.sh
#
# Vendors the Dutch acts the ehds-logging crate cites into docs/specs/nl-law/:
# the Wet aanvullende bepalingen verwerking persoonsgegevens in de zorg
# (Wabvpz, BWBR0023864), whose Article 15a(1) lets a care provider make a
# client's data available through an electronic exchange system only with
# the client's explicit consent, and the Uitvoeringswet Algemene verordening
# gegevensbescherming (UAVG, BWBR0040940), whose Article 30(3)(a) lifts the
# ban on processing health data for care providers under GDPR Art. 9(2)(h).
# Each is the consolidation FerroHEALTH/FerroEHR's scripts/vendor/law-nl.sh
# pins, the one the undated wetten.overheid.nl permalink resolved to on
# 2026-11-05.
#
# Each act is fetched as the XML the BWB repository publishes for that
# consolidation, which is what wetten.overheid.nl renders and exports
# ("Exporteer regeling", XML). The HTML page stamps the day it is read
# ("Geraadpleegd op"), so its sha256 moves every day and cannot be pinned.
# Each artefact is pinned by URL and sha256 (scripts/vendor/lib/pinned.sh),
# and the "Dutch acts" row of docs/VERSIONS.md carries the pin-set digest.
#
# Usage:
#   scripts/vendor/nl-law.sh
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

bwb=https://repository.officiele-overheidspublicaties.nl/bwb
statute='Auteurswet Art. 11 (BWBR0001886): "Er bestaat geen auteursrecht op wetten, besluiten en verordeningen, door de openbare macht uitgevaardigd, noch op rechterlijke uitspraken en administratieve beslissingen." Overheid.nl, Bestanden en hergebruik (https://www.overheid.nl/help/officiele-bekendmakingen/bestanden-en-hergebruik), read 2026-11-05: "Dit betekent dat deze informatie vrij mag worden hergebruikt, tenzij dat in de publicatie anders is aangegeven." The act states no other terms'

pin commit wabvpz-bwbr0023864-2025-07-05.xml "BWBR0023864, consolidation in force from 2025-07-05 (wetten.overheid.nl/BWBR0023864/2025-07-05)" \
  "$bwb/BWBR0023864/2025-07-05_0/xml/BWBR0023864_2025-07-05_0.xml" \
  35ddfbbb3533ab20e0555eaaa259724cc5e540bf2fed6763b76b2d2e4f4bdbc0 "$statute" \
  'Wet aanvullende bepalingen verwerking persoonsgegevens in de zorg (Wabvpz). The ehds-logging crate cites its Article 15a.'
pin commit uavg-bwbr0040940-2026-09-01.xml "BWBR0040940, consolidation in force from 2026-09-01 (wetten.overheid.nl/BWBR0040940/2026-09-01)" \
  "$bwb/BWBR0040940/2026-09-01_0/xml/BWBR0040940_2026-09-01_0.xml" \
  3c251b75faaa79bf1c356cb55550f9c1745ac47398438f5da18734642b6b8d3a "$statute" \
  'Uitvoeringswet Algemene verordening gegevensbescherming (UAVG). The ehds-logging crate cites its Article 30(3)(a).'
pinned_corpus nl-law "Dutch acts" \
  "the Dutch acts ehds-logging cites" \
  "The Wabvpz and the UAVG, each the whole consolidation in the XML the BWB
repository (the Basiswettenbestand behind wetten.overheid.nl) publishes for
it, byte for byte. No copyright subsists in a Dutch statute (Auteurswet
Art. 11), and Overheid.nl states that such texts may be reused freely
unless the publication says otherwise; neither act says otherwise." "" \
  "the citations of crates/ehds-logging (the consent condition and the Dutch legal ground of the access record, FerroHEALTH/FerroAUDIT#140)"

say "done"
