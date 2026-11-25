<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Conformance evidence

This page lists what FerroAUDIT's tests check against each logging
requirement, and where the record writes each field. The tests run on every
pull request; the numbers are self-tested, never a certification.

The Xt-EHR deliverables cited here are Joint Action deliverables, not
adopted law: Xt-EHR D8.2 (Digital testing environment specifications,
2026-05-19,
<https://www.xt-ehr.eu/wp-content/uploads/2026/05/Xt-EHR-D8.2.pdf>) turns
the D5.1 requirements into testable assertions, and Xt-EHR D5.2 (EEHRxF
metadata, 2026-04-09,
<https://www.xt-ehr.eu/wp-content/uploads/2026/05/Xt-EHR-D5.2.pdf>) lists
logging metadata in §7.2.7. Regulation (EU) 2025/327 Annex II point 3 is
the law, and where a deliverable asks for more than the Regulation, the
Regulation decides what the record must hold.

## The EHDS access record's logging fields

`ehds-logging` writes the access record as a BALP 1.1.4 `AuditEvent`
(feature `balp`). Each row names the field, the Annex II point it rests on,
the element it is written to and the test in
`crates/ehds-logging/tests/it/balp/fields.rs` that holds it there.

| Field | Source | Annex II | BALP `AuditEvent` element | Test |
|---|---|---|---|---|
| Healthcare provider | D8.2 §5.1.8, §5.1.9 | 3.2(a) | an `agent` with `who.type` `Organization` and the provider's `who.identifier` | `annex_ii_3_2_a_the_provider_is_an_organization_agent` |
| Natural person | D8.2 §5.1.8 | 3.2(b) | `agent:user`: `who.identifier` (the token's issuer and subject), `who.display` (the professional's name), the `ihe-otherId` extension typed `NPI` | `annex_ii_3_2_b_the_natural_person_is_the_user_agent` |
| Categories of data | D8.2 §5.1.8, §5.1.9 | 3.2(c) | the entity `ehds-categories`: `detail` `ehds-category` in HL7 Europe's priority category code system, and its `securityLabel` | `annex_ii_3_2_c_the_categories_are_the_categories_entity` |
| Time and date | D8.2 §5.1.8, §5.1.9 | 3.2(d) | `recorded` | `annex_ii_3_2_d_the_time_and_date_is_recorded` |
| Origin of the data | D8.2 §5.1.8, §5.1.9 | 3.2(e) | one entity `origin` per origin, `what.identifier` its endpoint, `detail` `system-id` | `annex_ii_3_2_e_each_origin_is_an_origin_entity` |
| Accessing EHR system | D8.2 §5.1.9 | 3.2(a), for a system acting for nobody | the `Application` agent (DICOM `110150`) by its `client_id`, and the client agent's `network.address` | `d8_2_log_api_access_names_the_accessing_system` |
| Actor | D5.2 §7.2.7 | 3.2(a), (b) | `agent:user` and the `Application` agent | `d5_2_actor_is_the_user_and_the_client_agent` |
| Object | D5.2 §7.2.7 | none (3.2(c) records its category) | `entity:data` and `entity:patient` | `d5_2_object_is_the_data_and_the_patient_entity` |
| Timestamp | D5.2 §7.2.7 | 3.2(d) | `recorded` | `d5_2_timestamp_is_recorded` |
| Action | D5.2 §7.2.7 | none | `action` and the `restful-interaction` `subtype` | `d5_2_action_is_the_action_and_the_subtype` |
| Purpose, purpose-of-use indicators | D5.2 §7.2.7 | none | `purposeOfEvent` and `agent:user` `purposeOfUse` | `d5_2_purpose_is_the_purpose_of_event_and_of_use` |
| Context | D5.2 §7.2.7 | none | `source.observer` and `entity:transaction` (the request id) | `d5_2_context_is_the_observer_and_the_transaction` |
| User roles and the policies of the access | D5.2 §7.2.7 | none | `agent:user` `role` and `policy` (the token's `jti`, then each data permit) | `d5_2_roles_and_policies_are_the_user_agents` |
| Standard event type | D5.2 §7.2.7 | none | `type` from `audit-event-type` | `d5_2_event_type_is_the_audit_event_type` |
| Retention policy | D5.2 §7.2.7 | 3.4 | the entity `ehds-categories`: `detail` `ehds-retention-period`, `ehds-retention-ends`, `ehds-retention-ground`, `ehds-retention-floor` | `d5_2_retention_policy_is_the_categories_entitys` |
| Legal basis | D5.2 §7.2.7 | none | gap | none |
| Controller and processor roles | D5.2 §7.2.7 | none | gap | none |

Annex II 3.2 asks for five points, and the record writes all five. The two
gaps are D5.2 metadata that the Regulation does not ask of a log entry.
The controller and processor roles belong to a processing activity, whose
record of processing names the controller and each processor (GDPR
Art 30(1)(a) and 30(2)(a)), and the legal basis belongs to each purpose of
processing, which the controller states to the data subject (GDPR
Art 13(1)(c)). Both are the same for every access of one deployment and
purpose, so writing them on each record would repeat them on every entry. A relayed access carries the legal ground of its federation
agreement (`agreement-legal-ground` on the entity `ehds-relayed`), because
that ground differs per agreement.

## Xt-EHR D8.2 logging assertions

| Assertion | D8.2 | Rests on | Evidence |
|---|---|---|---|
| log-UserAccess | §5.1.8 | Annex II 3.2 | `d8_2_log_user_access_records_every_annex_point` |
| log-APIAccess | §5.1.9 | Annex II 3.2 | `d8_2_log_api_access_names_the_accessing_system` |
| log-Review | §5.1.11 | Annex II 3.3 | `ihe-atna` feature `arr`, the Audit Record Repository with the Retrieve Audit Message Option (`docs/architecture.md` §4.5); `d8_2_log_review_retrieves_the_matching_records_and_records_the_retrieval` |
| log-interoperability (SHOULD) | §5.1.12 | none | `d8_2_log_interoperability_claims_a_balp_pattern`, and the BALP validation below |

## BALP validation

Every `AuditEvent` the `ehds-logging` BALP tests write is held to the BALP
1.1.4 pattern it claims, and to the FHIR R4 `AuditEvent`, by the HL7 FHIR
Validator in the CI `conformance` job (`scripts/checks/fhir-validator.sh`,
pinned in `docs/VERSIONS.md`). A record fails on an error, a fatal issue or
a claimed profile the validator could not find. The job's summary and the
`balp-validation` artifact list every issue.

To run it locally, with Java 17 or later:

```sh
BALP_EMIT_DIR=target/balp-records cargo nextest run -p ehds-logging --all-features -E 'test(/^balp::/)'
scripts/checks/fhir-validator.sh target/balp-records target/balp-validation
```
