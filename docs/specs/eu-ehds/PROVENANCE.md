<!-- This file describes vendored third-party material; the bytes beside it
     keep their upstream licence, not the licence of this repository. -->

# Provenance: the European Health Data Space Regulation and the acts ehds-logging cites

Vendored by `scripts/vendor/eu-ehds.sh`, each artefact pinned by its
URL and the sha256 of its bytes. Never edit a file here: change the pins in
the script and the pin-set digest in docs/VERSIONS.md, and re-run the script.

- Pin-set digest (sha256 over the sorted `mode  file  url  sha256` lines of
  the pins): `437682c4109f855d475ed052b80a4540b8d6f805da05f5b4130107445a76e1e8`
- Fetched: 2026-10-20, with the User-Agent `ferroaudit-vendor (scripts/vendor)`
- Artefacts: 5 committed, 0 cache only, 0 needing
  manual retrieval
- Files in this directory: 5 besides this one, each verbatim as the
  publisher serves it
- Tree digest (sha256 over the sorted per-file `sha256  path` listing,
  `PROVENANCE.md` excluded): `e31444d208f701270a84cae37c637c048498b1efc5a2ff9e3bfb524aa5d7acc5`
- Read by: the citations and the tests of crates/ehds-logging

Regulation (EU) 2025/327 on the European Health Data Space, Commission
Implementing Regulation (EU) 2026/2099 on the cross-border identification
and authentication mechanism, the eHealth Network guidelines on the Patient
Summary and the General Guidelines they defer to, and Commission Decision
2011/833/EU of 12 December 2011 on the reuse of Commission documents, which
governs the reuse of all of them. The Regulation, the implementing act and
the Decision are official EU legal acts published in the Official Journal;
the Commission legal notice (https://commission.europa.eu/legal-notice_en)
licenses the guidelines under CC BY 4.0.

## Artefacts

### `reg-eu-2025-327-en.xhtml` (committed)

- Source: <https://publications.europa.eu/resource/cellar/531b8c37-f962-11ef-b7db-01aa75ed71a1.0006.03/DOC_1>
- Version: OJ L, 2025/327, 5.3.2025 (CELEX 32025R0327), English
- sha256: `bf331ac48264118fb0461f793123a2726da88195a11d4bccb2075fd6326909da`
- Licence: An official EU legal act published in the Official Journal; reuse under Commission Decision 2011/833/EU of 12 December 2011 on the reuse of Commission documents
- Note: Pinned at the Publications Office (Cellar) manifestation, the XHTML EUR-Lex serves at https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=OJ:L_202500327. The EUR-Lex copy adds a monitoring script tag with per-request ids, so its sha256 differs on every fetch.

### `impl-reg-eu-2026-2099-en.xhtml` (committed)

- Source: <https://publications.europa.eu/resource/cellar/dff777d4-b61d-11f1-81de-01aa75ed71a1.0006.03/DOC_1>
- Version: OJ L, 2026/2099, 22.9.2026 (CELEX 32026R2099), English
- sha256: `6389d047750ed8fca879d53ebeda22af444d1f1619ed3bf6f13560097aa887d0`
- Licence: An official EU legal act published in the Official Journal; reuse under Commission Decision 2011/833/EU of 12 December 2011 on the reuse of Commission documents
- Note: Commission Implementing Regulation (EU) 2026/2099 of 21 September 2026 on the cross-border identification and authentication mechanism, adopted under Article 16(2) of Regulation (EU) 2025/327; applies from 26 March 2027, its Article 3(3) and Article 5(2) from 26 March 2029 (its Article 9). Pinned at the Cellar manifestation the CELEX resource resolves to.

### `ehn-guidelines-patientsummary.pdf` (committed)

- Source: <https://health.ec.europa.eu/document/download/e020f311-c35b-45ae-ba3d-03212b57fa65_en?filename=ehn_guidelines_patientsummary_en.pdf>
- Version: Release 3.4, November 2024
- sha256: `9daaab30ef8e8cb5f8ab2be1480d80869b17702d118ab3d13268324184267f5d`
- Licence: A Commission document, reusable under Commission Decision 2011/833/EU (dec-eu-2011-833-en.xhtml, here) and CC BY 4.0 under the European Commission legal notice (https://commission.europa.eu/legal-notice_en): "Unless otherwise indicated (e.g. in individual copyright notices), content owned by the EU on this website is licensed under the Creative Commons Attribution 4.0 International (CC BY 4.0) licence". The PDF states no licence of its own

### `ehn-guidelines-general.pdf` (committed)

- Source: <https://health.ec.europa.eu/document/download/d8f7955c-4a2c-4ba2-9aea-25f3044896e8_en?filename=ehealth_health-data_electronic-exchange_general-guidelines_en.pdf>
- Version: Release 3.1, November 2024
- sha256: `041ee26c324c47a72564cb040865df2d9a7d7b25a5ed08e2e2568abc54c2198f`
- Licence: A Commission document, reusable under Commission Decision 2011/833/EU (dec-eu-2011-833-en.xhtml, here) and CC BY 4.0 under the European Commission legal notice (https://commission.europa.eu/legal-notice_en): "Unless otherwise indicated (e.g. in individual copyright notices), content owned by the EU on this website is licensed under the Creative Commons Attribution 4.0 International (CC BY 4.0) licence". The PDF states no licence of its own
- Note: The eHealth Network General Guidelines, Release 3.1, to which the Patient Summary and ePrescription guidelines defer.

### `dec-eu-2011-833-en.xhtml` (committed)

- Source: <https://publications.europa.eu/resource/cellar/cb76d4a0-c886-40bd-99d7-8db018a723d0.0010.03/DOC_1>
- Version: OJ L 330, 14.12.2011, p. 39 (CELEX 32011D0833), English
- sha256: `2d5bc877b9a5aad948af21c680aca1d3409f41df5dd0dcc57ef9b1b225f60982`
- Licence: An official EU legal act published in the Official Journal; reuse under Commission Decision 2011/833/EU of 12 December 2011 on the reuse of Commission documents
- Note: Commission Decision 2011/833/EU of 12 December 2011 on the reuse of Commission documents, the reuse terms of the acts and the guidelines here (its Articles 3 and 6), kept as their licence evidence. It replaces the Commission legal notice page, whose bytes changed with every render. Pinned at the Cellar manifestation the CELEX resource resolves to.
