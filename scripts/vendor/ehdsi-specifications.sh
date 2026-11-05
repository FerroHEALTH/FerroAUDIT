#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# scripts/vendor/ehdsi-specifications.sh
#
# Vendors the eHDSI specifications the access record cites into
# docs/specs/ehdsi-specifications/: the Audit Trail Profile, the SAML Profile
# and Section I Security Policies FerroHEALTH/FerroAUDIT#140 reads, with the
# NCPeH Architecture and Components Specifications and the OpenNCP
# Implementation that describe the NCPeH audit those records correlate with.
# The eHDSI wiki sits behind EU Login, so this script downloads nothing. The
# owner places the files in a local directory, and the script finds each one
# by the sha256 it pins (scripts/vendor/lib/pinned.sh, mode `supplied`) and
# commits it verbatim under a normalised name. The first three are the files
# FerroHEALTH/FerroFED vendors, identified by the same sha256.
#
# By the owner's decision of 2026-11-05 a document is committed when its
# cover states distribution level PU and that reuse is authorised provided
# the source is acknowledged.
#
# Usage:
#   scripts/vendor/ehdsi-specifications.sh DIR   commit the files DIR holds
#   scripts/vendor/ehdsi-specifications.sh --check
#                                                check the committed tree
#
# Requires: shasum, awk.

set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

case "${1:-}" in
  --check) export PINNED_CHECK=1 ;;
  '')
    if [[ "${PINNED_DIGESTS_ONLY:-}" != "1" ]]; then
      echo "usage: $0 DIR | --check" >&2
      exit 2
    fi
    ;;
  *)
    [[ -d "$1" ]] || {
      echo "$0: '$1' is not a directory" >&2
      exit 2
    }
    PINNED_SUPPLIED="$(cd "$1" && pwd)"
    export PINNED_SUPPLIED
    ;;
esac
cd "$root"

# shellcheck source=scripts/vendor/lib/corpus.sh
# shellcheck disable=SC1091 # shellcheck is not run with -x; the library is checked on its own
. "$root/scripts/vendor/lib/corpus.sh"
# shellcheck source=scripts/vendor/lib/pinned.sh
# shellcheck disable=SC1091 # shellcheck is not run with -x; the library is checked on its own
. "$root/scripts/vendor/lib/pinned.sh"

corpus_require shasum awk
pinned_begin

wiki='https://webgate.ec.europa.eu/fpfis/wikis/display/EHDSI/2.+eHDSI+INTEROPERABILITY+SPECIFICATIONS%2C+Requirements+and+Frameworks'
reuse='"Reuse is authorised, provided the source is acknowledged." and "Distribution level*: PU", where "PU = Public" (the cover and control page, read 2026-11-05)'
docx_title='The docx core properties carry a stale title (dc:title "D3.7.2 Section I - Security Policies_v0.8"), so the sha256 identifies the file, not the metadata.'
pdf_title='The PDF document information carries a stale title ("D3.7.2 Section I - Security Policies_v0.8"), so the sha256 identifies the file, not the metadata.'

# ack SOURCE_LINE: the acknowledgement the cover names, quoted from its
# source line.
ack() { printf 'Acknowledgement: "Source: %s", the source line of the cover.' "$1"; }

pin supplied ehdsi-audit-trail-profile-10.1.0.docx \
  'Audit Trail Profile, document version 10.1.0, 05/10/2026, status Wave 10 Operation Ready' \
  "$wiki" 8cbc4edbc39071d7fdad09e1d913a83f1585e3a41009ece3b63942b2fba146e3 "$reuse" \
  "$(ack 'DG SANTE, CEF eHealth DSI, 2026') The owner supplied eHDSI_Audit_Trail_Profile_v10.1.0.docx. $docx_title"
pin supplied ehdsi-saml-profile-10.0.0.pdf \
  'SAML Profile, document version 10.0.0, 01/06/2026, status Wave 10 Operation Ready' \
  "$wiki" edd17e79b49ec56cd3799a58fd392ef51a09aa53c4bc726a05b4252dd7c2501d "$reuse" \
  "$(ack 'DG SANTE, MyHealth@EU, 2026') The owner supplied eHDSI_SAML_Profile_v10.0.0.pdf. $pdf_title"
pin supplied ehdsi-section-i-security-policies-10.0.0.pdf \
  'Section I – Security Policies, document version 10.0.0, 01/06/2026, status Wave 10 Operation Ready' \
  "$wiki" 010bed197540bdcd92574a0df4824d21d7ca615fa5a905dea469a7cc70c89167 "$reuse" \
  "$(ack 'DG SANTE, CEF eHealth DSI, 2026') The owner supplied eHDSI_Section_I-Security_Policies_v10.0.0.pdf. $pdf_title"
pin supplied ehdsi-ncpeh-architecture-specifications-10.0.0.pdf \
  'NCPeH Architecture Specifications (document name NCPeH Architecture Specification), document version 10.0.0, 01/06/2026, status Wave 10 Operation Ready' \
  "$wiki" 9b53ee18c1c89be3fa97e4735da42c78455d95da7c3bd4eaeee56c0a3f578627 "$reuse" \
  "$(ack 'DG SANTE, CEF eHealth DSI, 2026') The owner supplied eHDSI_NCPeH_Architecture_Specifications_v10.0.0.pdf. $pdf_title"
pin supplied ehdsi-ncpeh-components-specifications-10.0.0.pdf \
  'NCPeH Components Specifications, document version 10.0.0, 01/06/2026, status Wave 10 Operation Ready' \
  "$wiki" 5da44873fefdf3f9bb2a855d3d6c053990f8730a25a0b5ffe1ccfe75e0e73ba2 "$reuse" \
  "$(ack 'DG SANTE, CEF MyHealth@EU, 2026') The owner supplied eHDSI_NCPeH_Components_Specifications_v10.0.0.pdf. $pdf_title"
pin supplied ehdsi-openncp-implementation-10.0.0.pdf \
  'OpenNCP Implementation, document version 10.0.0, 01/06/2026, status Wave 10 Operation Ready' \
  "$wiki" 1dcb77cfa1c7eac20bd922db66fc9a8a20c1f1d7c5d86ac415134aa30b546e04 "$reuse" \
  "$(ack 'DG SANTE, CEF eHealth DSI, 2026') The owner supplied eHDSI_OpenNCP_Implementation_v10.0.0.pdf. $pdf_title"
pinned_corpus ehdsi-specifications "eHDSI specifications" \
  "the eHDSI specifications supplied by the owner" \
  "The eHDSI Audit Trail Profile 10.1.0, SAML Profile 10.0.0 and Section I
Security Policies 10.0.0, with the NCPeH Architecture Specifications, the
NCPeH Components Specifications and the OpenNCP Implementation, each
10.0.0, retrieved by the owner from the eHDSI wiki, which sits behind EU
Login. Each cover states distribution level PU (Public) and \"Reuse is
authorised, provided the source is acknowledged.\", read on 2026-11-05, so
by the owner's decision of that day each is committed verbatim, as the owner
supplied it, with the source line of its cover as the acknowledgement. DG
SANTE, the Directorate-General for Health and Food Safety of the European
Commission, publishes them for MyHealth@EU. The first three are the files
FerroHEALTH/FerroFED vendors under the same names, identified by the same
sha256." none \
  "FerroHEALTH/FerroAUDIT#140 (the eHDSI fields of the relayed access, their DICOM rendering and the Section I SR 5 rule)"

say "done"
