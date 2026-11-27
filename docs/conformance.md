<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Conformance evidence

This page lists what FerroAUDIT's tests check against each logging
requirement, and where the record writes each field. The tests run on every
pull request; the numbers are self-tested, never a certification.

The Xt-EHR deliverables cited here are Joint Action deliverables, not
adopted law: Xt-EHR D8.2 (EHR Conformity Assessment Scheme - Assertion Document and
Checklists, 2026-05-19,
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
| Legal basis | D5.2 §7.2.7 | none | the entity `ehds-categories`: `detail` `ehds-legal-basis` (the GDPR Art 6(1) point) and `ehds-legal-condition` (the Art 9(2) point) | `d5_2_legal_basis_is_the_categories_entitys` |
| Controller and processor roles | D5.2 §7.2.7 | none | the entity `ehds-categories`: `detail` `ehds-controller` per controller and `ehds-processor` per processor | `d5_2_controller_and_processor_roles_are_the_categories_entitys` |

Annex II 3.2 asks for five points, and the record writes all five. The
legal basis and the controller and processor roles are D5.2 regulatory
metadata ("Metadata SHALL capture legal basis, controller/processor roles,
and retention policies", §7.2.7) that the Regulation does not ask of a log
entry. The record carries them as `ehds_logging::regulatory::Regulatory`:
the GDPR Art 6(1) point and the Art 9(2) condition, each controller (Art
4(7), Art 26) and each processor (Art 4(8)). A deployment declares them in
its configuration, and a recording system overrides them for one access
with `Regulatory::overridden_by`, since a legal basis can differ per access
(an explicit consent stored beside it) and a relayed access carries the
legal ground of its federation agreement (`agreement-legal-ground` on the
entity `ehds-relayed`). A deployment that declares none writes none.

### Annex II 3.3 and 3.4

Annex II point 3 asks two more things of the harmonised software
components beside the log entry of 3.2. The README badge counts 3.2(a) to
(e), 3.3 and 3.4.

| Point | Requirement | Evidence | Test |
|---|---|---|---|
| 3.3 | tools or mechanisms to review and analyse the log data, or the connection and use of external software for the same purposes | `ihe-atna` feature `arr`: ITI-81 Retrieve ATNA Audit Event, the connection an external review tool searches the log through | `annex_ii_3_3_external_software_reviews_a_patients_accesses` in `crates/ihe-atna/tests/it/arr/search.rs` |
| 3.4 | different retention periods and access rights that take into account the origins and categories of the data | the retention period of `ehds-categories` (`ehds-retention-period`) and the `securityLabel` of `ehds-categories` and of each `origin`, per category and per origin | `annex_ii_3_4_retention_and_access_rights_follow_the_categories` |

## Xt-EHR D8.2 logging assertions

| Assertion | D8.2 | Rests on | Evidence |
|---|---|---|---|
| log-UserAccess | §5.1.8 | Annex II 3.2 | `d8_2_log_user_access_records_every_annex_point` |
| log-APIAccess | §5.1.9 | Annex II 3.2 | `d8_2_log_api_access_names_the_accessing_system` |
| log-UserID | §5.1.10 | Annex II 3.1, 3.2(b) | `agent:user.who.identifier` is the token's issuer and subject, with the eIDAS level of assurance (Regulation (EU) No 910/2014 Art 8(2)) in `ihe-assuranceLevel`: `d8_2_log_user_id_names_each_user_by_issuer_subject_and_assurance`. The eIDAS relying party D8.2 suggests is the identity provider's, outside this library. |
| log-Review | §5.1.11 | Annex II 3.3 | `ihe-atna` feature `arr`, the Audit Record Repository with the Retrieve Audit Message Option (`docs/architecture.md` §4.5); `d8_2_log_review_retrieves_the_matching_records_and_records_the_retrieval` |
| log-interoperability (SHOULD) | §5.1.12 | none | `d8_2_log_interoperability_claims_a_balp_pattern`, and the BALP validation below |
| log-Retention | §5.1.13 (out of scope for automated testing) | Annex II 3.4 | the retention period and the security labels follow the origin as well as the category: `d8_2_log_retention_retention_and_access_rights_follow_the_origins` |

## BALP validation

Every `AuditEvent` the `ehds-logging` BALP tests and the `ihe-atna` pattern
tests write is held to the BALP 1.1.4 pattern it claims, and to the FHIR R4
`AuditEvent`, by the HL7 FHIR Validator in the CI `conformance` job (`scripts/checks/fhir-validator.sh`,
pinned in `docs/VERSIONS.md`). A record fails on an error, a fatal issue or
a claimed profile the validator could not find. The job's summary and the
`balp-validation` artifact list every issue.

The README badge counts the `AuditEvent` profiles of the pinned BALP 1.1.4
package (`docs/specs/ihe-balp/package`, 19 of them) that at least one valid
record claims in `meta.profile`; a record that fails validation counts for
none of its claims. Each profile and the test whose record claims it:

| BALP 1.1.4 profile | Test that writes a valid record of it |
|---|---|
| `IHE.BasicAudit.Query` | `balp::population::a_population_access_states_its_scope_in_its_own_entity` |
| `IHE.BasicAudit.PatientQuery` | `balp::each_action_is_written_as_its_pattern` |
| `IHE.BasicAudit.Read` | `balp::an_access_that_names_no_patient_is_written_as_its_plain_pattern` |
| `IHE.BasicAudit.PatientRead` | `balp::each_action_is_written_as_its_pattern` |
| `IHE.BasicAudit.Create` | `balp::an_access_that_names_no_patient_is_written_as_its_plain_pattern` |
| `IHE.BasicAudit.PatientCreate` | `balp::each_action_is_written_as_its_pattern` |
| `IHE.BasicAudit.Update` | `balp::an_access_that_names_no_patient_is_written_as_its_plain_pattern` |
| `IHE.BasicAudit.PatientUpdate` | `balp::each_action_is_written_as_its_pattern` |
| `IHE.BasicAudit.Delete` | `balp::an_access_that_names_no_patient_is_written_as_its_plain_pattern` |
| `IHE.BasicAudit.PatientDelete` | `balp::each_action_is_written_as_its_pattern` |
| `IHE.BasicAudit.OAUTHaccessTokenUse.Comprehensive` | `balp::a_data_permit_is_the_persons_policy_after_the_token` |
| `IHE.BasicAudit.OAUTHaccessTokenUse.Minimal` | `ihe-atna` `balp::patterns::oauth_minimal_is_a_jwt_whose_client_is_not_named` |
| `IHE.BasicAudit.OAUTHaccessTokenUse.Opaque` | `ihe-atna` `balp::patterns::oauth_opaque_keeps_the_last_32_characters_of_the_token` |
| `IHE.BasicAudit.SAMLaccessTokenUse.Minimal` | `ihe-atna` `balp::patterns::saml_minimal_names_the_user_by_issuer_and_name_id` |
| `IHE.BasicAudit.SAMLaccessTokenUse.Comprehensive` | `ihe-atna` `balp::patterns::saml_comprehensive_keeps_the_attributes_and_the_organisation` |
| `IHE.BasicAudit.AuthZconsent` | `ihe-atna` `balp::patterns::authz_consent_names_the_client_the_user_the_organisation_and_the_authorizer` |
| `IHE.BasicAudit.PrivacyDisclosure.Source` | `ihe-atna` `balp::patterns::privacy_disclosure_source_is_an_export_from_the_recording_system` |
| `IHE.BasicAudit.PrivacyDisclosure.Recipient` | `ihe-atna` `balp::patterns::privacy_disclosure_recipient_is_an_import_by_the_recording_system` |
| `IHE.IUA.71` | `ihe-atna` `balp::patterns::iua_71_records_the_token_request_at_the_authorization_server` |

To run it locally, with Java 17 or later:

```sh
BALP_EMIT_DIR=target/balp-records cargo nextest run --workspace --all-features \
  -E '(package(ehds-logging) & test(/^balp::/)) | (package(ihe-atna) & test(/^balp::patterns::/))'
scripts/checks/fhir-validator.sh target/balp-records target/balp-validation
```

## ITI-81 search parameters

`ihe-atna` feature `arr` answers Retrieve ATNA Audit Event over the
repository's store. Each search parameter RESTful ATNA Rev. 3.6
§3.81.4.1.2 asks an Audit Record Repository to support has its test in
`crates/ihe-atna/tests/it/arr/search.rs`.

| Parameter | Section | Matched against | Test |
|---|---|---|---|
| `date` | §3.81.4.1.2.1 | `recorded` | `date_bounds_the_recorded_time` |
| `address` | §3.81.4.1.2.2 | `agent.network.address`, as a substring | `address_matches_a_substring_of_the_network_address` |
| `agent.identifier` | §3.81.4.1.2.2 | `agent.who.identifier` | `agent_identifier_matches_the_agents_identifier` |
| `patient.identifier` | §3.81.4.1.2.2 | the identifier of an agent or entity that is the patient | `patient_identifier_matches_the_patients_identifier` |
| `entity.identifier` | §3.81.4.1.2.2 | `entity.what.identifier` | `entity_identifier_matches_the_entitys_identifier` |
| `entity-type` | §3.81.4.1.2.2 | `entity.type` | `entity_type_matches_the_entitys_type` |
| `entity-role` | §3.81.4.1.2.2 | `entity.role` | `entity_role_matches_the_entitys_role` |
| `source.identifier` | §3.81.4.1.2.2 | `source.observer.identifier` | `source_identifier_matches_the_observers_identifier` |
| `type` | §3.81.4.1.2.2 | `type` | `type_matches_the_event_type` |
| `subtype` | §3.81.4.1.2.2 | `subtype` | `subtype_matches_the_event_subtype` |
| `outcome` | §3.81.4.1.2.2 | `outcome` | `outcome_matches_any_of_its_values` |

## The README badges

The conformance badges of the README are shields.io endpoint files under
`conformance/badges/`, each N of M from the CI `conformance` job, which
fails when a committed file differs from its run
(`scripts/checks/conformance.sh`). They count self-tests: no badge says
certified, compliant or conformant, and a guard fails one that does.
