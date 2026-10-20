<!-- This file describes vendored third-party material; the bytes beside it
     keep their upstream licence, not the licence of this repository. -->

# Provenance: DICOM PS3.15 2026c

Vendored by `scripts/vendor/dicom-ps3-15.sh`, each artefact pinned by its
URL and the sha256 of its bytes. Never edit a file here: change the pins in
the script and the pin-set digest in docs/VERSIONS.md, and re-run the script.

- Pin-set digest (sha256 over the sorted `mode  file  url  sha256` lines of
  the pins): `9a9b6eb4218785a873402cc045eff8aa16c05aa89a6df5909628301dc7a69ac7`
- Fetched: 2026-10-20, with the User-Agent `ferroaudit-vendor (scripts/vendor)`
- Artefacts: 0 committed, 1 cache only, 0 needing
  manual retrieval
- Files in this directory: 0 besides this one, each verbatim as the
  publisher serves it
- Tree digest (sha256 over the sorted per-file `sha256  path` listing,
  `PROVENANCE.md` excluded): `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`
- Read by: the citations of crates/ihe-atna (the audit message, its optional elements and the DICOM code tables)

DICOM PS3.15 2026c, Security and System Management Profiles, the single
HTML page NEMA publishes for the release. The code cites Annex A.5.1 (the
audit message schema, with the optional EventOutcomeDescription,
AuditSourceTypeCode and ParticipantObjectDataLifeCycle and the outcome
indicator 12) and Annex A.5.3 (the audit message tables that fix the EventID
and EventTypeCode of each event). Nothing of the page is committed.

## Artefacts

### `part15.html` (cache only, not redistributed)

- Source: <https://dicom.nema.org/medical/dicom/2026c/output/html/part15.html>
- Version: DICOM PS3.15 2026c
- sha256: `4cab4e7b7f5e14a43d5eb6ec13ae7c46915cef378e09c0dd75ea263827f17f32`
- Licence: Copyright NEMA; the publication's Notice and Disclaimer grants no licence to redistribute, so it is read locally and not committed
- Not redistributed: the script fetches it into `.vendor-cache/dicom-ps3-15/`, which git
  ignores; this repository carries none of its content.
