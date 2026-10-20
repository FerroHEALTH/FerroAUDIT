#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# scripts/vendor/ietf-syslog.sh
#
# Vendors the IETF RFCs the ihe-atna crate cites into docs/specs/ietf-syslog/:
# the Syslog Protocol (RFC 5424), its TLS transport (RFC 5425) and its UDP
# transport (RFC 5426), which ITI TF-2 §3.20.3 references; RFC 3881, whose
# participant object codes the IHE audit tables write as `RFC-3881`; and RFC
# 6750, the bearer token usage the FHIR Feed's credential source follows.
#
# Each RFC is pinned by its RFC Editor URL and the sha256 of its text
# (scripts/vendor/lib/pinned.sh); a published RFC never changes, so a moved
# hash is a fetch fault. The "IETF syslog and audit RFCs" row of
# docs/VERSIONS.md carries the pin-set digest.
#
# Usage:
#   scripts/vendor/ietf-syslog.sh
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

editor=https://www.rfc-editor.org/rfc
trust='IETF Trust Legal Provisions (BCP 78), section 3.c: the right "to copy, publish, display and distribute IETF Contributions and IETF Documents in full and without modification" (https://trustee.ietf.org/license-info)'

pin commit rfc3881.txt "RFC 3881, September 2004, Informational" "$editor/rfc3881.txt" \
  3dea919cf0b565890a47089560a2f22dbbb9ba1af34442c56cee9372fd7b4308 "$trust"
pin commit rfc5424.txt "RFC 5424, March 2009, Standards Track" "$editor/rfc5424.txt" \
  24d91f90a05d2536730ada929c296f45ae9cd72226e22b6775ad8c04cae37a53 "$trust"
pin commit rfc5425.txt "RFC 5425, March 2009, Standards Track" "$editor/rfc5425.txt" \
  a807ec7024fb200e9cf91590cfe41ac06029fb48bf9ddba088862bc134647639 "$trust"
pin commit rfc5426.txt "RFC 5426, March 2009, Standards Track" "$editor/rfc5426.txt" \
  e39b8df74e60d463d06fa9301f7347688184ad043e4ff026f9bf7697eae16bf5 "$trust"
pin commit rfc6750.txt "RFC 6750, October 2012, Standards Track" "$editor/rfc6750.txt" \
  9dc385cf4ecdd85024e5a95e447ede9230e11700bf35251906b25b37557d604b "$trust"
pinned_corpus ietf-syslog "IETF syslog and audit RFCs" \
  "the IETF syslog and audit RFCs" \
  "The text form of RFC 3881 (Security Audit and Access Accountability
Message XML Data Definitions for Healthcare Applications), RFC 5424 (The
Syslog Protocol), RFC 5425 (Transport Layer Security (TLS) Transport Mapping
for Syslog), RFC 5426 (Transmission of Syslog Messages over UDP) and RFC 6750
(The OAuth 2.0 Authorization Framework: Bearer Token Usage), each the whole
document as the RFC Editor publishes it, with every legend and notice it
carries, which is what the IETF Trust's grant covers." "" \
  "the citations of crates/ihe-atna (the syslog message and frame, the triage of the UDP transport, the RFC 3881 codes, the FHIR Feed credential source)"

say "done"
