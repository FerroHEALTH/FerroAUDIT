#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# scripts/vendor/ehds-categories.sh
#
# Vendors the FHIR packages the Article 14(1) categories of the ehds-logging
# access record are read against (Regulation (EU) 2025/327): the HL7 Europe
# EU Health Data API, whose code system codes the six priority categories,
# the HL7 Europe Imaging Report and Hospital Discharge Report guides, and the
# MyHealth@EU Master Value Sets Catalogue. One corpus per package, each under
# docs/specs/.
#
# Each package is the FHIR package registry's tarball, pinned by URL and
# sha256 (scripts/vendor/lib/pinned.sh) and committed whole, so its sha256
# stays the pin; each corpus row of docs/VERSIONS.md carries its pin-set
# digest. Every package declares CC0-1.0 in its manifest. The pins are
# carried from FerroHEALTH/FerroFED's scripts/vendor/eehrxf.sh, the corpora
# ehds-logging was written against there.
#
# Usage:
#   scripts/vendor/ehds-categories.sh
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

registry=https://packages.fhir.org
cc0='CC0-1.0: the license of the package manifest and of the ImplementationGuide resource'
whole='The registry tarball as served, package manifest included; the archive is committed whole so its sha256 stays the pin.'
read_by_categories=$'the tests of crates/ehds-logging (the wire form of the Article 14(1)\n  categories in the access record)'

pin commit hl7.fhir.eu.health-data-api-1.0.0-ballot.tgz "1.0.0-ballot (FHIR 4.0.1, 2026-03-13)" \
  "$registry/hl7.fhir.eu.health-data-api/1.0.0-ballot" \
  5204af60bf7f74f301a627e641a34683d392014f53d5df8ba3651c964e92977b "$cc0" \
  "$whole It defines EEHRxFDocumentPriorityCategoryCS, the code system of the six priority categories."
pinned_corpus eu-hl7-health-data-api "HL7 Europe EU Health Data API" \
  "the HL7 Europe EU Health Data API hl7.fhir.eu.health-data-api" \
  "The European health data API guide in its ballot version, canonical
\`http://hl7.eu/fhir/health-data-api\`, whose
\`CodeSystem/eehrxf-document-priority-category-cs\` codes the priority
categories of Regulation (EU) 2025/327 Article 14(1), committed under
CC0-1.0." "" "$read_by_categories"

pin commit hl7.fhir.eu.imaging-1.0.0-ballot.tgz "1.0.0-ballot (FHIR 4.0.1, 2026-03-16)" \
  "$registry/hl7.fhir.eu.imaging/1.0.0-ballot" \
  bb6012bd7a019ac82f5e939dd6e892e6db6b15fd2931dcf7cc445c1ad92c3d70 "$cc0" \
  "$whole Its Composition and DiagnosticReport profiles require the priority category code Medical-Imaging."
pinned_corpus eu-hl7-imaging "HL7 Europe Imaging Report" \
  "the HL7 Europe Imaging Report R4 guide hl7.fhir.eu.imaging" \
  "The European imaging report guide in its ballot version, canonical
\`http://hl7.eu/fhir/imaging\`, committed under CC0-1.0." "" "$read_by_categories"

pin commit hl7.fhir.eu.hdr-0.1.0-ballot.tgz "0.1.0-ballot (FHIR 4.0.1, 2025-06-03)" \
  "$registry/hl7.fhir.eu.hdr/0.1.0-ballot" \
  458b8ba0edcf22d75d455d6149c9264d486bb8ec052251a90afd8654410482ee "$cc0" \
  "$whole It carries no priority category code."
pinned_corpus eu-hl7-hdr "HL7 Europe Hospital Discharge Report" \
  "the HL7 Europe Hospital Discharge Report guide hl7.fhir.eu.hdr" \
  "The European hospital discharge report guide in its ballot version,
canonical \`http://hl7.eu/fhir/hdr\`, committed under CC0-1.0." "" "$read_by_categories"

pin commit myhealth.eu.fhir.mvc-package-9.1.0.tgz "9.1.0 (FHIR 4.0.1, 2026-05-08)" \
  "$registry/myhealth.eu.fhir.mvc-package/9.1.0" \
  3f99350f36a2a1ea17b7fde37c39ff766bd5c3bc9a29c6183af1acfd07b78ce1 "$cc0" \
  "$whole Its document type value sets code each MyHealth@EU service by LOINC, with no category code system."
pinned_corpus ehdsi-mvc "MyHealth@EU Master Value Sets Catalogue" \
  "the MyHealth@EU Master Value Sets Catalogue myhealth.eu.fhir.mvc-package" \
  "The value sets MyHealth@EU exchanges are coded with, canonical
\`http://fhir.ehdsi.eu/mvc-package\`, committed under CC0-1.0." "" "$read_by_categories"

say "done"
