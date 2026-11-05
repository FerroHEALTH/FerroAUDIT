#!/usr/bin/env bash
# SPDX-FileCopyrightText: Cadasto B.V.
# SPDX-License-Identifier: BUSL-1.1
# scripts/vendor/ietf-oauth.sh
#
# Vendors the IETF OAuth RFC the ehds-logging crate cites into
# docs/specs/ietf-oauth/: RFC 8693 (OAuth 2.0 Token Exchange), whose `act`
# claim (§4.1) the access record's actor chain records.
#
# The RFC is pinned by its RFC Editor URL and the sha256 of its text
# (scripts/vendor/lib/pinned.sh); a published RFC never changes, so a moved
# hash is a fetch fault. The "IETF OAuth RFCs" row of docs/VERSIONS.md
# carries the pin-set digest.
#
# Usage:
#   scripts/vendor/ietf-oauth.sh
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

pin commit rfc8693.txt "RFC 8693, January 2020, Standards Track" "$editor/rfc8693.txt" \
  358d658014738bf4dc56c8aae2094ce716ede23b6d5a80488a7dc8e8d72da609 "$trust" \
  'Its Copyright Notice states the document is subject to BCP 78 and the IETF Trust Legal Provisions Relating to IETF Documents (https://trustee.ietf.org/license-info) in effect on the date of publication. Vendored with FerroHEALTH/FerroAUDIT#142 into docs/specs/ietf-syslog/ and moved here, at the same URL and sha256.'
pinned_corpus ietf-oauth "IETF OAuth RFCs" \
  "the IETF OAuth RFCs" \
  "The text form of RFC 8693 (OAuth 2.0 Token Exchange), the whole document
as the RFC Editor publishes it, with every legend and notice it carries,
which is what the IETF Trust's grant covers." "" \
  "the citations of crates/ehds-logging (the actor chain of the access record, FerroHEALTH/FerroAUDIT#140)"

say "done"
