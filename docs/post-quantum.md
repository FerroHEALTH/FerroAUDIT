<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Post-quantum inventory

This page lists every cryptographic use in the FerroAUDIT crates and their
release workflows, the algorithm each uses, and whether it resists a
cryptographically relevant quantum computer. The machine-readable copy is
`conformance/post-quantum.tsv`, one row per use, with five tab-separated
columns: `use`, `algorithm`, `safe` (`yes` or `no`), `pq_standard` (the
standard that defines the post-quantum form, `no standard yet`, or, for a
classical use FerroAUDIT does not choose, whose choice it is and the text
that says so) and `location`. Every change that adds, removes or changes a
cryptographic use updates both in the same pull request.
`scripts/checks/post-quantum.sh --update` rewrites the table below and the
README badge (`conformance/badges/post-quantum.json`) from the file, and CI
fails when either disagrees with it or a location names a file the tree
does not hold.

## The sources

- **NIST IR 8547 (final):** ECDSA, EdDSA and RSA signatures (§4.1.1) and
  ECDH, DH and RSA key establishment (§4.1.2) are deprecated after 2030 and
  disallowed after 2035. Symmetric primitives with at least 128 bits of
  classical security stay (§4.1.3).
- **The EU coordinated implementation roadmap (NIS Cooperation Group,
  v1.1):** a use case is high-risk when its confidentiality must hold for
  10 years or more, which health records and their audit trail need, and
  high-risk use cases complete the transition by 31.12.2030 (§4.2), after
  which they use no quantum-vulnerable public-key mechanism stand-alone
  (§4.1).
- **RFC 10024:** the TLS 1.3 hybrid group X25519MLKEM768 (codepoint
  0x11EC), within the hybrid design of RFC 9954.
- **RFC 9964:** ML-DSA for JOSE. **RFC 9881:** ML-DSA in X.509, CRLs and
  OCSP.

A row is `safe` when every key it depends on is established or verified by
a post-quantum or hybrid mechanism, or when it is a symmetric primitive at
128 bits or more. A row whose post-quantum form has no standard yet counts
as not safe. A row whose keys the operator holds counts as safe when the
code presents and verifies their post-quantum form under its standard.

## What FerroAUDIT does

`ihe-atna` speaks TLS over the rustls configuration its caller hands it:
the syslog sender (`atna::repository`), the syslog receiver
(`arr::syslog`) and the FHIR Feed's client (`atna::feed`). A product builds
that configuration with `ferrohealth-server`'s `TlsClient`, which
`ferrohealth-smart`'s `ServerTls` calls, and that client offers the hybrid
X25519MLKEM768 first in the TLS 1.3 key exchange, then X25519, P-256 and
P-384 for a peer that lacks it. The tests hold both transports to it: over
the configuration `ServerTls` builds, the syslog sender and receiver and the
FHIR Feed agree on X25519MLKEM768, and a peer that offers only X25519 still
connects.

FerroAUDIT takes `ferrohealth-server` 0.12.4 through `ferrohealth-smart`
0.36. The revocation check of that release, which a product may turn on,
verifies CRLs and OCSP answers signed with RSA or ECDSA; the verification of
revocation data an ML-DSA CA signs (RFC 9881) arrives with the next
`ferrohealth-smart` release, the one on `ferrohealth-server` 0.13.

The certificates stay classical in practice for now. The products verify
an outbound peer through `TlsClient::platform_config`, the platform's
verifier: on Linux that is webpki, which verifies ML-DSA certificates in
TLS 1.3 (a Linux-only test holds the syslog transport to it), but the macOS
verifier refuses an ML-DSA certificate (FerroHEALTH/FerroSYS#470), so the
row counts as not safe.

## Whose choice the classical rows are

- **The TLS 1.2 key exchange:** X25519MLKEM768 is a TLS 1.3 group
  (RFC 10024), and no post-quantum key exchange will be specified for
  TLS 1.2 (RFC 9851 §2). A caller's configuration may allow TLS 1.2, which
  ITI TF-2 §3.20.4.1.2.1.1 allows for the syslog transport.
- **crates.io publishing:** Trusted Publishing exchanges an OIDC token that
  GitHub issues, and GitHub's issuer lists RS256 as its only signing
  algorithm (`id_token_signing_alg_values_supported` at
  <https://token.actions.githubusercontent.com/.well-known/openid-configuration>).
- **Release build provenance:** `actions/attest-build-provenance` signs
  through Sigstore, and Sigstore's algorithm registry marks ML-DSA-44,
  ML-DSA-65 and ML-DSA-87 as not yet operational
  (<https://github.com/sigstore/architecture-docs/blob/60730f3251c28a96920c718bc65a59ad69ef2dc4/algorithm-registry.md>).

## The inventory

| Use | Algorithm | Safe | Post-quantum standard | Location |
|---|---|---|---|---|
| TLS 1.3 key exchange of the syslog sender, the syslog receiver and the FHIR Feed client | X25519MLKEM768 first, then X25519, P-256 or P-384, over the configuration ferrohealth-server's TlsClient builds through ferrohealth-smart's ServerTls | yes | RFC 10024 | `crates/ihe-atna/src/atna/repository.rs, crates/ihe-atna/src/arr/syslog.rs, crates/ihe-atna/src/atna/feed.rs` |
| TLS 1.2 key exchange where the caller's configuration allows TLS 1.2 | ECDHE over X25519, P-256 or P-384 | no | none for TLS 1.2 (RFC 9851 §2); the operator's choice, which ITI TF-2 §3.20.4.1.2.1.1 allows | `crates/ihe-atna/src/atna/repository.rs (Repository::tls)` |
| TLS certificate authentication of the syslog and FHIR Feed connections | ML-DSA-44, ML-DSA-65 or ML-DSA-87 in TLS 1.3 where the caller's verifier is webpki, as on Linux, or ECDSA, RSA or Ed25519, as the operator's certificates hold; the macOS platform verifier refuses ML-DSA (FerroHEALTH/FerroSYS#470) | no | RFC 9881; draft-ietf-tls-mldsa-06 (IANA TLS SignatureScheme 0x0904 to 0x0906) | `crates/ihe-atna/src/atna/repository.rs, crates/ihe-atna/src/arr/syslog.rs, crates/ihe-atna/src/atna/feed.rs` |
| TLS record protection | AES-128-GCM, AES-256-GCM, ChaCha20-Poly1305 | yes | NIST IR 8547 4.1.3 | `crates/ihe-atna/src/atna/repository.rs (rustls)` |
| OAuth client assertion of the FHIR Feed's client credentials grant | ML-DSA-44, ML-DSA-65 or ML-DSA-87 with an ML-DSA key, or RS384, ES256, ES384 or EdDSA, as the key ferrohealth-smart's outbound client is given holds | yes | RFC 9964 | `crates/ihe-atna/src/atna/feed.rs (ferrohealth-smart client)` |
| crates.io publishing | Trusted Publishing (OIDC token from GitHub, RS256) | no | none offered: GitHub's choice (its OIDC issuer lists only RS256) | `.github/workflows/release.yml, .github/workflows/publish-crates.yml` |
| Release build provenance | Sigstore attestation (ECDSA P-256) | no | none operational: Sigstore's choice (its algorithm registry marks ML-DSA not operational) | `.github/workflows/release-crate.yml` |
