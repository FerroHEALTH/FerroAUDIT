<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.0] - 2026-11-09

`ihe-atna`'s `smart` feature follows `ferrohealth-smart` to the 0.13 line,
the release that refuses a malformed JSON document without quoting it, so
a product hands the FHIR Feed bearer the same `TokenSource` it hands its
other clients. The change breaks the public API. Products pin `=0.8.0`; a
product that hands its token source to the FHIR Feed moves to
`ferrohealth-smart` 0.13 with it.

### Changed

- `ihe-atna` requires `ferrohealth-smart` 0.13 (caret `0.13.0`) in place of
  0.12, so the `smart` feature's `atna::bearer::Bearer` takes a
  `TokenSource` of the 0.13 line. None of the smart API the bearer reads
  changed. To upgrade, move the product to `ferrohealth-smart` 0.13 and
  pin `ihe-atna` and `ehds-logging` at `=0.8.0` (#183).

## [0.7.1] - 2026-11-09

`ihe-atna`'s `retrieve` feature reads the ITI-81 search values with
`fhir-search` 0.1.129, the version the rest of the family requires, so a
product that takes `ihe-atna/retrieve` beside a crate on 0.1.129 resolves.
The public API is unchanged. Products pin `=0.7.1`.

### Fixed

- `ihe-atna` requires `fhir-search` `=0.1.129` in place of `=0.1.128`.
  `ihe-iti` requires 0.1.129 through its `rest` feature, so a product that
  took `ihe-atna/retrieve` and `ihe-iti/pixm` together could not resolve
  its dependencies. `ehds-logging` follows on its exact requirement (#181).

## [0.7.0] - 2026-11-09

`ihe-atna`'s `smart` feature follows `ferrohealth-smart` to the 0.12 line,
so a product hands the FHIR Feed bearer the same `TokenSource` it hands its
other clients. The change breaks the public API. Products pin `=0.7.0`; a
product that hands its token source to the FHIR Feed moves to
`ferrohealth-smart` 0.12 with it.

### Changed

- `ihe-atna` requires `ferrohealth-smart` 0.12 (caret `0.12.0`) in place of
  0.10, so the `smart` feature's `atna::bearer::Bearer` takes a
  `TokenSource` of the 0.12 line. None of the smart API the bearer reads
  changed. To upgrade, move the product to `ferrohealth-smart` 0.12 and
  pin `ihe-atna` and `ehds-logging` at `=0.7.0` (#179).

## [0.6.0] - 2026-11-08

`ihe-atna` takes the syslog sender's TLS configuration from its caller,
reads ITI-81 search values through the family's FHIR search grammar, and its
`smart` feature follows `ferrohealth-smart` to the 0.10 line. All three break
the public API. Products pin `=0.6.0`: a product builds the rustls
`ClientConfig` for `Repository::tls` with `ferrohealth-server`'s
`TlsClient` from its family TLS keys, and a product that hands its token
source to the FHIR Feed moves to `ferrohealth-smart` 0.10.

### Changed

- `atna::repository::Repository::tls` takes the caller's
  `Arc<rustls::ClientConfig>` in place of `TlsSettings`, so the trust roots
  and the client identity of the RFC 5425 connection come from the
  product's own TLS keys (`client_identity`, `client_identity_file`,
  `trust_roots_file`) through `ferrohealth_server::tls_client::TlsClient`,
  whose `platform_config` keeps the platform's roots beside the extra ones.
  A rustls configuration speaks TLS 1.2 or 1.3 alone, so the ITI TF-2
  §3.20.4.1.2.1.1 floor holds by the type. To upgrade, build the
  configuration with `TlsClient` and pass it in `Arc` (#173, #177).
- `retrieve` reads the ITI-81 search values through the family's FHIR
  search grammar, `fhir-search` (exact `=0.1.128`), the one copy FerroBRIDGE's
  facade and `ihe-iti` read theirs with. `Value::Token` carries
  `fhir_search::token::Token`, whose `Debug` never shows the code;
  `DateParam::date` (a `SearchDate`, written back as read) replaces
  `DateParam::value`, `DateParam::prefix` returns `fhir_search::prefix::Prefix`,
  and `Bound::Empty` is what `gt` or `sa` matches on a range that runs past
  the last instant. Two readings move to FHIR R4 search §3.1.1.4.7: a time
  without an offset is read in the repository zone instead of refused, and
  a time without seconds spans its minute instead of its first second. To
  upgrade, match `Value::Token` on the `fhir_search` token forms and read a
  date with `DateParam::date` (#174).
- `ihe-atna` requires `ferrohealth-smart` 0.10 (caret `0.10.1`) in place of
  0.9, so the `smart` feature's `atna::bearer::Bearer` takes a
  `TokenSource` of the 0.10 line. A product that hands its token source to
  the FHIR Feed moves to `ferrohealth-smart` 0.10 with this release, which
  adds the onward `cache` setting and the TRC patient type and reads a
  Keycloak 26.8 impersonation as issued (#170).

### Removed

- `atna::repository::TlsSettings`, the PEM parsing behind it and the
  `RepositoryError` variants `Roots`, `Identity`, `NoCertificate` and
  `Tls`; `ihe-atna` no longer depends on `rustls-platform-verifier` (#173).
- `retrieve::Token`, `retrieve::TokenSystem` and `retrieve::date::Prefix`,
  replaced by the `fhir-search` types (#174).

## [0.5.2] - 2026-11-08

The access record carries the two Wabvpz Art. 15f refusals FerroEHR
records for a caller under a Dutch agreement. The change is additive.
Products pin `=0.5.2`.

### Added

- `Binding::RefusedWabvpzRole` and `Binding::RefusedWabvpzOrganisationType`,
  the codes `refused-wabvpz-role` and `refused-wabvpz-organisation-type` a
  recording system writes when it refuses a caller under a Dutch agreement
  by Wabvpz Art. 15f; each is a refusal and neither a security event
  (#167).

## [0.5.1] - 2026-11-08

A recording system that writes its own DICOM audit message names and
formats the binding and relayed members of an access record as both
renderings of `ehds-logging` do, with no feature, and the record carries
the four caller-kind refusals FerroEHR records. Every change is additive.
Products pin `=0.5.1`.

### Added

- `ehds_logging::detail`, which needs no feature: the names of the
  `ehds-accessor` and `ehds-relayed` entities and of every `detail` entry
  the BALP record and the DICOM message write. A recording system that
  writes its own DICOM message names the binding and relayed members by
  them without the `balp` feature. `balp::detail`, `balp::ACCESSOR_ENTITY`
  and `balp::RELAYED_ENTITY` keep their names and take each value from it
  (#161).
- `ehds_logging::detail::accessor` and `ehds_logging::detail::relayed`,
  which need no feature: the entries of the `ehds-accessor` and
  `ehds-relayed` entities, each an `Entry` of a name and its value in the
  form both renderings write, in the order the BALP record writes them.
  The `balp` rendering writes both entities from them. An `Entry`'s `Debug`
  shows its name and no value (#164).
- `Binding::RefusedSystemPersonalScope`, `Binding::RefusedNoAgreement`,
  `Binding::RefusedScopeBeyondAgreement` and
  `Binding::RefusedCorrelationMissing`, the codes
  `refused-system-personal-scope`, `refused-no-agreement`,
  `refused-scope-beyond-agreement` and `refused-correlation-missing`; each
  is a refusal and none a security event (#162).

## [0.5.0] - 2026-11-08

The access record carries every binding field a recording system holds:
the four refusal codes 0.4.0 left out, the relationship of a
representation, the provider an accessor acts for with the facilities of a
system commit, and a federation access that states only the fields its
agreement requires. This release breaks the public API of `ehds-logging`:
each break below names the upgrade step. Products pin `=0.5.0`.

### Added

- `Binding::RefusedDeprovisioned`, `Binding::RefusedWrongMarker`,
  `Binding::RefusedRepresentationUnrecorded` and `Binding::RefusedTrcMissing`,
  the codes `refused-deprovisioned`, `refused-wrong-marker`,
  `refused-representation-unrecorded` and `refused-trc-missing`; the last
  is a security event (#158).
- `Accessor::acts_for` and `Accessor::facilities`, each an `IssuedId`: the
  healthcare provider a bound person or a system client acts for, and the
  facilities a system client's commit names (Annex II 3.2(a)), written as
  the `acts-for` and `facility` details of the `ehds-accessor` entity in
  the BALP record and the DICOM message (#158).
- `Representation::relationship`, the relationship code of the
  representative's openEHR `PARTY_RELATED` committer, written as the
  `represents-relationship` detail (#158).
- `RelayedParts::asserted`, each further field a relaying issuer asserted
  by name, written as an `asserted-field` detail `<name>=<value>` (#158).

### Changed

- Breaking: `Representation::represented` is the subject's identifier as
  the request asserted it, a `String` in place of a `PartyRef`, and
  `Representation::ground` is optional. Upgrade: pass the asserted
  identifier, wrap the ground in `Some`, and set `relationship` (#158).
- Breaking: `Accessor` has two more public fields. Upgrade: add
  `acts_for: None` and `facilities: Vec::new()`, or the values the
  recording system holds, to every struct literal (#158).
- Breaking: for a federation access the 2026/2099 Annex fields are
  optional, since the regulation binds a contact point and the agreement
  declares what a federation must state. `RelayedParts::country_code` and
  `RelayedParts::provider`, `RelayedProfessional::name`, `identifier` and
  `issuing_authority`, and `RelayedProvider::issuing_authority` are
  `Option`s, and `RelayedParts` has the field `asserted`. A contact point
  still states each, refused with `RelayedError::ContactPointFieldMissing`
  naming the field. A federation professional named by no identifier is no
  DICOM `ActiveParticipant`, which PS3.15 A.5.1 gives a mandatory `UserID`.
  Upgrade: wrap the values in `Some` and add `asserted: BTreeMap::new()`
  (#158).

## [0.4.0] - 2026-11-06

The access record says how the recording system bound the accessor to a
party, and records verified professionals and organisations of other
parties, relayed by a MyHealth@EU contact point or a domestic federation,
with the eHDSI fields that join it to the contact point's own audit entry.
`ehds-logging` writes it as a DICOM PS3.15 audit message too (feature
`atna`), and both crates escape every value they write. This release breaks
the public API of both crates: each break below names the upgrade step.
`ihe-atna`'s `smart` feature follows `ferrohealth-smart` to the 0.9 line.
Products pin `=0.4.0`, and a product that hands its token source to the
FHIR Feed moves to `ferrohealth-smart` 0.9 with it.

### Added

- `ehds_logging::record::Binding` and `Accessor::binding`: the binding
  outcome, one of `bound` (with a `guest` mark), `provisional`, `external`,
  `organisation`, `system`, `self`, `refused-unbound`, `refused-ambiguous`,
  `refused-inactive`, `refused-abandoned`, `refused-key-claim-missing`
  (with the claim), `refused-groups-unavailable`, `refused-guest`,
  `refused-external-incomplete` (with the fields) and
  `refused-trc-patient-mismatch` (a security event). The party, a
  `PartyRef` triple, rides on `bound` and `provisional` with the groups
  (`BoundParty`), and on `refused-inactive` and `refused-abandoned` alone,
  so no other outcome can carry one; `Accessor::party` reads it (#140).
- `Accessor::actors`, the token's actor chain outermost first (RFC 8693
  §4.1), and `Accessor::represents`, whom a representative acts for and on
  which ground (#140).
- `Acting::Delegated`, `Acting::External` and `Acting::System`, beside
  `Person` and `Client`, which stays readable in stored records (#140).
- The eHDSI members of a relayed access (eHDSI SAML Profile 10.0.0 §2.1,
  §2.3): `RelayedProfessional::name` (the `subject-id` full name),
  `healthcare_provider_identifier` (2026/2099 Annex Table 1), `name_id`
  (`NameId`, written `SPProvidedID<NameID@Issuer>` as the Audit Trail
  Profile 10.1.0 §2.3.41 fixes), `authentication_method` (one of the seven
  two-factor `AuthenticationMethod` classes, in place of an assurance
  level), `on_behalf_of` and `permissions`; `RelayedProvider::facility_type`,
  `home_community_id` and `organisation_type`; and on the relayed access the
  point of care, the IdA and TRC ids, the request message UUID and the
  contact points' certificate CNs (`Gateways`) (#140).
- `Agreement`, with its name, version, coded `legal_ground`, consent
  decision and `AccessClass` (`automated` or `chained`), on every
  federation access (GDPR Art 5(2), Art 30(1)(b)) (#140).
- `Correlation` and `CorrelationSource`: where the correlation id was read,
  an attested token claim, an attested `authorization_details` member or a
  request header. `AccessRecord::validate` refuses a `chained` organisation
  access without an attested one, and `BalpSink` refuses such a record
  before it reaches the recorder (#140).
- The BALP record writes the new members in the `ehds-relayed` entity and a
  new `ehds-accessor` entity, and the bound party as the `agent:user`
  `otherId` typed `PRN` (BALP 1.1.4 §3:5.7.5.4, `ihe_iua:person_id`) (#140).
- `ehds_logging::atna` (feature `atna`): the record as a DICOM PS3.15 A.5
  audit message, written from the same exchange as its BALP record by the
  `RESTful` ATNA Rev. 3.6 Table 3.20.4.2.2.1-1 mapping. A relayed access
  names the requester by `SPProvidedID<NameID@Issuer>` with the `subject-id`
  name and the role, and each contact point by its certificate CN (Audit
  Trail Profile 10.1.0 §2.3.13 to §2.3.15); the patient is its HL7 v2 CX
  identifier (§2.3.17). The message carries the assertion and message ids
  and no security header (eHDSI Section I Security Policies 10.0.0 SR 5)
  (#140).
- `ihe_atna::escape::untrusted`, which writes each control, bidirectional
  formatting and line separator character as its code point (`\u{202e}`);
  `ihe_atna::user::PersonId` with `User::with_person_id`, written as
  `otherId` typed `PRN`; `EventIdentification::purposes_of_use`, written as
  `PurposeOfUse` (ITI TF-2 §3.20.7.1.2); and `ParticipantObject::name`,
  written as `ParticipantObjectName` (#140).

### Changed

- Breaking: `ihe-atna` requires `ferrohealth-smart` 0.9 (caret `0.9.0`) in
  place of 0.8, so the `smart` feature's `atna::bearer::Bearer` takes a
  `TokenSource` of the 0.9 line; `ehds-logging` follows through `ihe-atna`.
  The client credentials API `Bearer` uses (`TokenSource`,
  `ClientCredentials`, `Endpoint`, `TokenError`) is unchanged in 0.9.
  Upgrade: a product that hands its token source to the FHIR Feed moves to
  `ferrohealth-smart` 0.9 with this release, following that crate's own
  0.9.0 upgrade steps for its `[auth]` section (#141).
- Breaking: `Relayed` is built with `Relayed::try_from(RelayedParts { .. })`
  and read with `Relayed::as_parts`; its fields are no longer public. It
  refuses a contact point that names no professional, a federation access
  with no agreement, an organisation whose agreement declares no access
  class, a contact point provider named by no `urn:oid:` or `urn:hl7ii:`
  identifier, and an ISCO `44` clerical worker without `on_behalf_of`
  (`RelayedError`). Upgrade: build a `RelayedParts` and convert it, and
  handle the error.
- Breaking: `Relayed::contact_point` is replaced by `RelayedParts::via`.
  Upgrade: `contact_point: issuer` becomes
  `via: Via::ContactPoint { issuer }`, and a domestic federation issuer is
  `Via::Federation { issuer }`.
- Breaking: the relayed professional is optional
  (`RelayedParts::professional: Option<RelayedProfessional>`). Upgrade:
  wrap it in `Some`; `None` records an organisation without a named person,
  admitted only under a federation agreement with an access class.
- Breaking: `RelayedProfessional` has the full name `name`, and
  `family_name` and `given_name` are optional; `RelayedProvider::name` and
  `address` are optional. Upgrade: set `name` to the `subject-id`, wrap the
  parts of the name in `Some`, and name the new members (`None` or empty
  when the issuer does not state them).
- Breaking: `Accessor` has the public fields `binding`, `actors` and
  `represents`. Upgrade: a struct literal names them, `None`, `Vec::new()`
  and `None` for an accessor that carries none.
- Breaking: `Request::correlation` is an `Option<Correlation>`. Upgrade:
  `Some(id)` becomes `Some(Correlation { value: id, source })` with the
  source it was read from.
- Breaking: the BALP detail `contact-point` is replaced by `relayed-via`
  and `relayed-issuer`, and `balp::detail::CONTACT_POINT` by `RELAYED_VIA`
  and `RELAYED_ISSUER`. Upgrade: a reader of stored records reads the new
  names for a record written by this release.
- Breaking: `ihe_atna::atna::message::EventIdentification` has the public
  field `purposes_of_use` and `ParticipantObject` the public field `name`.
  Upgrade: a struct literal names them, `Vec::new()` and `None` for a
  message that carries neither.
- Every string the BALP `AuditEvent` carries, and every attribute and text
  value of the DICOM message, is written through `escape::untrusted`, so a
  control, bidirectional formatting or line separator character in a value
  is written as its code point. A detail value and a query of the DICOM
  message stay base64 of the value as given (DICOM PS3.15 A.5.1.1). A
  reader that compared a written value with one holding such a character
  compares it with the escaped form (#140).

## [0.3.3] - 2026-10-31

`ehds-logging` keeps access records about a resource held from the person
under Regulation (EU) 2025/327 Art 3(3) out of the person's own access trail
(Art 9) until the hold is released or expires. Products pin `=0.3.3`; a
product that delays the person's access builds a `withheld::Holds` register
for the person's read.

### Added

- `ehds_logging::withheld`: the person's own access trail under a hold of
  Regulation (EU) 2025/327 Art 3(3). The product that delays a person's
  access to a resource sets the hold with `Holds::hold` (withheld from the
  person until a time) and lifts it with `Holds::release`; the person-facing
  read (`Holds::shows` or `Holds::trail` with `Reader::Person`) leaves out
  every access record that names the resource until the hold is released or
  expires, then shows it unchanged with its original time (Art 9). A hold on
  an openEHR versioned object covers each of its versions. Professional and
  supervisory reads are unchanged (#130).

## [0.3.2] - 2026-10-29

`ehds-logging`'s BALP record names the data of a read, create, update or
delete by `what.identifier` when its id is no FHIR literal reference, so an
independent Audit Record Repository accepts it. Products pin `=0.3.2`; a
product whose object ids are no FHIR resources, such as FerroEHR's openEHR
version uids, declares their namespace with `ObjectIds::Identifiers`.

### Added

- `ehds_logging::balp::ObjectIds`, `BalpSink::with_object_ids` and
  `balp::exchange_with`, through which a recording system declares what
  the ids it passes as `Request::resource` are: literal references to the
  FHIR resources it serves (`ObjectIds::References`, the default), or
  identifiers it issues in a namespace it names
  (`ObjectIds::Identifiers { system }`) (#127).
- `ihe_atna::balp::is_literal_reference`, which holds a value to the form
  FHIR R4 gives a literal reference: `[type]/[id]` of an R4 resource type,
  with an optional `/_history/[vid]`, or an absolute `http` or `https` URL
  (references.html §2.3.0.2) (#127).

### Fixed

- The BALP record of a read, create, update or delete names its data
  (`AuditEvent.entity[data].what`) by `what.identifier` when the id is no
  FHIR literal reference, such as the openEHR version uid FerroEHR passes,
  in place of a `what.reference` holding that id, which an independent
  Audit Record Repository refuses (HAPI FHIR answers `400`, `HAPI-0505`).
  A literal reference stays `what.reference`; under
  `ObjectIds::Identifiers` every id is `what.identifier` with the declared
  `system`, which the ITI-81 `entity.identifier` search matches. A request
  that addressed no resource names its operation by `what.identifier` in
  place of `what.reference` (FHIR R4 references.html §2.3.0.3; BALP 1.1.4
  `entity:data.what` admits either) (#127).

## [0.3.1] - 2026-10-27

`ihe-atna`'s `smart` feature follows `ferrohealth-smart` to the 0.8 line.
Products pin `=0.3.1`; a product that hands its token source to the FHIR
Feed moves to `ferrohealth-smart` 0.8 with it.

### Changed

- `ihe-atna` requires `ferrohealth-smart` 0.8 (caret `0.8.2`) in place of
  0.7.4, so the `smart` feature's `atna::bearer::Bearer` takes a
  `TokenSource` of the 0.8 line; `ehds-logging` follows through `ihe-atna`.
  A product that hands its token source to the FHIR Feed moves to
  `ferrohealth-smart` 0.8 with this release (#120).

## [0.3.0] - 2026-10-27

### Added

- `ehds_logging::record::Accessor::policies`: the identifier of every
  policy or plan an access was authorised under beside the token, such as
  the data permit of a health data access body (Regulation (EU) 2025/327
  Art 68) a secondary-use caller declares (#116). The BALP record writes
  each as an `agent:user` `policy` after the token's `jti`, as FHIR R4
  defines `AuditEvent.agent.policy` ("the policy or plan that authorized
  the activity being recorded", `0..*`). `Debug` shows how many, never one.
- `ihe_atna::user::User::with_policies` and `User::policies`, which carry
  those policies to `agent[user].policy`.

### Changed

- Breaking: `ehds_logging::record::Accessor` has the public field
  `policies`, so a product that builds an `Accessor` with a struct literal
  names it: `policies: Vec::new()` for an access authorised under no
  policy, or the permit identifiers it was declared under.
- A BALP record whose user acts under a policy beside the token no longer
  claims `IHE.BasicAudit.OAUTHaccessTokenUse.Comprehensive`: that pattern
  fixes `agent:oUser.policy` to the `jti` alone (`1..1`). A record without
  such a policy claims it as before.
- The DICOM PS3.15 `ActiveParticipant` of `ihe_atna::atna::message` says why
  it carries no policy: PS3.15 A.5.1.1 defines no element for one, and the
  `RESTful` ATNA mapping names a `ParticipantRoleIDCode` that schema does
  not hold, so a permit reaches an Audit Record Repository in the FHIR
  `AuditEvent` alone.

## [0.2.15] - 2026-10-26

### Changed

- The BALP `AuditEvent` of an EHDS access record writes each origin's
  security labels once, as the `securityLabel` of its `origin` entity, and
  no longer repeats them in `meta.security` (#111), on the ground #108 used
  for the categories: RESTful ATNA Rev. 3.6 Table 3.81.4.2.2.1-1 maps
  `entity.securityLabel` to the DICOM `ParticipantObjectSensitivity` and maps
  no `meta`. The record now writes no `meta.security`. A store that filtered
  on the FHIR `_security` search parameter, which reads `meta.security`,
  finds no label there and reads the entities' `securityLabel` instead.

## [0.2.14] - 2026-10-26

### Changed

- The BALP `AuditEvent` of an EHDS access record writes the labels of the
  data categories once, as the `securityLabel` of the `ehds-categories`
  entity, and no longer repeats them in `meta.security` (#108). RESTful ATNA
  Rev. 3.6 Table 3.81.4.2.2.1-1 maps `entity.securityLabel` to the DICOM
  `ParticipantObjectSensitivity` and maps no `meta`, BALP puts the labels of
  the data on its entity (`IHE.BasicAudit.Read`, `entity:data.securityLabel`),
  and that entity is the record of the categories accessed (Regulation (EU)
  2025/327 Annex II 3.2(c)). An unclassified access carried all six
  categories in both places; the record is about 155 bytes smaller after
  compression. `meta.security` keeps the origins' labels. A store that
  limits access by category reads the entity's `securityLabel`.
  `label::Labels::origins` returns the origins' labels each once.

## [0.2.13] - 2026-10-26

### Changed

- `ihe-atna` and `ehds-logging` require `ferrohealth-smart` 0.7.4 and
  `ferrohealth-secret` 0.2.0 as caret requirements, so a patch of either
  reaches a product through its lockfile without a FerroAUDIT release first.
  The products keep pinning `ihe-atna` and `ehds-logging` exactly.

## [0.2.12] - 2026-10-26

`ihe-atna` renders its masks through `ferrohealth-secret`, the family's one
mask, and the `smart` feature follows `ferrohealth-smart` to 0.7.3, which
renders through the same mask. Products pin `=0.2.12`; nothing they call
changes.

### Changed

- `redact::REDACTED` and `redact::RedactedUrl` render through
  `ferrohealth-secret`, the family's one mask, so every product shows a
  secret and a masked URL the same way. The rendering is unchanged (`***`,
  `scheme://***@host/path?***`), and both items keep their place and form.
- The `smart` feature takes `ferrohealth-smart` 0.7.3 in place of 0.7.1.

## [0.2.11] - 2026-10-26

The `smart` feature follows `ferrohealth-smart` to 0.7.1, which fixes
GHSA-vphj-3f8c-5hpx. Products pin `=0.2.11`; nothing they call changes.

### Security

- The `smart` feature takes `ferrohealth-smart` 0.7.1 in place of 0.7.0:
  every earlier release can render a configured URL's credentials in full
  (CWE-532, GHSA-vphj-3f8c-5hpx). A product that takes `ihe-atna` with
  `smart` can now take the patched `ferrohealth-smart`.

## [0.2.10] - 2026-10-25

The `smart` feature follows `ferrohealth-smart` to 0.7.0, so a product moves
both crates together. Products pin `=0.2.10`; nothing they call changes.

### Changed

- The `smart` feature takes `ferrohealth-smart` 0.7.0 in place of 0.6.0, the
  line the products move to together; `atna::bearer::Bearer` is unchanged.

## [0.2.9] - 2026-10-25

A BALP record names the audit source id in `source.observer.identifier`, so
a repository answers the ITI-81 `source.identifier` parameter. Products pin
`=0.2.9`; nothing they call changes.

### Changed

- A BALP record names the audit source id in `source.observer.identifier`
  beside the `display` (#97): the `RESTful` ATNA supplement maps
  `AuditSourceId` to `source.observer`, and the ITI-81 `source.identifier`
  parameter matches `source.observer.identifier`, so a repository that keeps
  these records can answer it.

## [0.2.8] - 2026-10-25

Two additions a product takes in place of its own code: the ITI-81 retrieval
behind `retrieve`, and the `ferrohealth-smart` client credentials token as a
FHIR Feed credential source behind `smart`. Products pin `=0.2.8`; nothing
they already use changes.

### Added

- `ihe-atna` serves Retrieve ATNA Audit Event (ITI-81) behind the new
  `retrieve` feature (#89). `retrieve::Query::parse` reads the query string
  of the FHIR R4 `AuditEvent` search into its `date` parameters and typed
  criteria: the four token parameters of the RESTful ATNA supplement and the
  R4 search parameters, typed by the `fhir-types` registry, with repeated
  parameters combined by AND, values by OR, and an unsupported parameter
  ignored and named. A query with no `date`, a modifier on a supported
  parameter or a value outside its form is refused with a `QueryError` that
  carries no value. `retrieve::date::DateParam::bounds` gives the interval of
  `recorded` instants a `date` matches, and `retrieve::bundle::searchset`
  and `retrieve::bundle::refusal` write the answer as `fhir_types::r4`
  resources.
- `ihe-atna` takes the client credentials token of `ferrohealth-smart` as
  the credential source of a FHIR Feed repository, behind the new `smart`
  feature (#90). `atna::bearer::Bearer`, built from a `TokenSource`, gives
  each request the cached token's `Authorization` value and drops a token the
  repository refused with `401`, so the request is sent once more under a
  token fetched anew (RFC 6750 §3.1). A product removes its own adapter.

### Changed

- The workspace requires `fhir-types` 0.1.128, the release the products
  build on, in place of the 0.1.113 line.

## [0.2.7] - 2026-10-25

### Changed

- `SECURITY.md` states the response times every product of the Ferro family
  states: an acknowledgement within about 5 business days in place of seven
  days, an initial assessment within about 10 business days, both best
  effort, and a fixed release published within 7 days of the confirmed
  assessment for a Critical vulnerability, 30 days for High, 90 days for
  Medium and with the next release for Low.

### Fixed

- `atna::repository::Connection::is_closed` answers at once on a live
  connection. It polled its read inside a zero-length tokio timeout, which
  waits for the next timer tick of about a millisecond, so a sender that
  probes before every frame (as the forwarder and FerroEHR's syslog sink do)
  was held to roughly 880 records per second on one connection; it now polls
  the read once without a timer and measures about 47 000 (#86).

## [0.2.6] - 2026-10-24

### Added

- `atna::forwarder::Forwarder::stop` ends the delivery loop: the forwarder
  finishes the message it is writing, closes its syslog connection with
  `Connection::close` and returns, so a product stops it and awaits the
  task where it aborted it before. A stop also ends a backoff (#82).

### Fixed

- The forwarder dropped its syslog connection, with the repository's TLS
  session tickets unread, when it reconnected after the repository ended
  the connection, and when its task was aborted, so the frames it wrote
  last could be lost to a reset after they had left the spool. It now
  closes a connection the repository ended with `Connection::close` before
  it reconnects, and closes its open connection when it is stopped (RFC 5425
  §4.4, ITI TF-2 §3.20.4.1.1) (#82).

## [0.2.5] - 2026-10-24

### Added

- `atna::repository::Connection::close` ends a connection as RFC 5425 §4.4
  asks of a transport sender: a TLS `close_notify`, the write half shut, and
  the stream read to the repository's end within the send timeout, under the
  new `Step::Close`. A product closes its connection whenever no further
  frame follows on it (#77).

### Fixed

- A frame sent just before a connection was dropped could be lost. Dropping
  the connection closed the socket with the repository's TLS session tickets
  unread, so the kernel reset it and discarded what was still queued; on
  Linux the repository received none of it. Closing with
  `Connection::close` delivers every frame sent. `Repository::connect` now
  sets `TCP_NODELAY`, so a frame never waits on the acknowledgement of the
  one before it (#77).

### Changed

- The release checks the family's Security changelog rules: every
  `### Security` entry names a GHSA advisory or an OpenVEX statement under
  `security/vex/`, and a Security section beside other changes opens with
  "Shipped with functional changes because:" and its reason (Regulation (EU)
  2024/2847 Annex I Part II(2), (4), (8)). CI checks `[Unreleased]`, and the
  release `plan` job the section it cuts, through `scripts/release/plan.sh`,
  which `scripts/release/preflight.sh` replays before the tag
  (`docs/release.md`). The release notes state the month the five years of
  security fixes end (Art 13(19)). CI fails an advisory exception in
  `deny.toml` that no longer names a finding. `SECURITY.md` names the
  `info@cadasto.com` reporting route and the advisory each fixed
  vulnerability is published as (#68).

## [0.2.4] - 2026-10-21

A patch release with no API change: a product pins `=0.2.4` in place of
`=0.2.3`.

### Fixed

- `ehds-logging`: the category map admits every archetype key the
  `ARCHETYPE_ID` grammar of openEHR BASE 1.3.0 admits. It refused a major
  version of `0`, which CKM gives its draft archetypes
  (`openEHR-EHR-OBSERVATION.imaging_exam_result.v0`), and the namespaced
  ADL 2 form (`org.openehr::openEHR-EHR-OBSERVATION.blood_pressure.v2.1.4`).
  A namespaceless key with a full version (`.v1.0.0`) is admitted too, as
  FerroEHR's `openehr-base` admits it, and a test holds the map's check and
  that parser to one answer on a shared corpus. The refusal's message names
  the optional namespace (#66).

### Changed

- The vendored openEHR BASE source moves from `Release-1.1.0` to commit
  `707b8ac28cea1ccc4410f8e930665d473475350f` (BASE Release 1.3.0,
  unreleased), the commit FerroEHR vendors.

## [0.2.3] - 2026-10-21

A patch release with no API change: a product pins `=0.2.3` in place of
`=0.2.2`.

### Fixed

- `ehds-logging`: a category map declared with `Matching::OpenEhr` writes
  its canonical text, and so its digest, with each key in its comparison
  form, so two declarations that differ only in the case of a key name one
  map, as openEHR BASE says they are. An exact map's text is unchanged. The
  digest of an openEHR-matched map whose keys are not all lower case moves
  once (#64).

## [0.2.2] - 2026-10-21

A patch release with no API change: a product pins `=0.2.2` in place of
`=0.2.1` and changes nothing else.

### Changed

- Both crates declare `rust-version = "1.98"` in place of 1.97, and the
  repository builds with Rust 1.99.1 in place of 1.98.1: the FerroHEALTH
  family builds with the latest stable toolchain and declares the release
  before it as its MSRV. A product on a toolchain older than 1.98 can no
  longer take the crates. The `balp` feature now builds on the declared
  MSRV too, and the `msrv` job checks every feature as well as no feature
  and `atna`.

## [0.2.1] - 2026-10-21

A patch release with no API change: a product pins `=0.2.1` in place of
`=0.2.0` and changes nothing else.

### Added

- `ihe-atna`: the feature `recording` compiles `balp::recording` alone (tokio
  and thiserror only), and the feature `redact` compiles
  `redact::RedactedUrl` and `fhir_base` alone (url only), so a profile crate
  such as FerroPIX's `ihe-iti` takes the record bound and the URL helpers
  without the BALP or syslog stack and drops its own copies (#61).
- `ihe-atna`: `balp::recording::Late::new` (and `Default`), so an audited
  client outside the crate can return the late-record failure it reports;
  `Late` is `#[non_exhaustive]`, which kept 0.2.0's callers from making one
  (#61).

## [0.2.0] - 2026-10-21

The audit content the FerroFED and FerroEHR migrations found missing, the
correctness of the EHDS retention, and the packaging those migrations asked
for. Public APIs of both crates break, so a product changes what the
**Changed** entries name.

### Added

- `ihe-atna`: `balp::USER_AUTHENTICATION`, `LOGIN`, `LOGOUT`, `EXPORT`,
  `IMPORT` (DICOM PS3.15 A.5.3) and `OPERATION` (FHIR R4
  `restful-interaction`) (#43); `Outcome::MajorFailure`, written as `12`
  (#44); `Exchange::outcome_description` (`outcomeDesc`), `Observer::source_types`
  (`source.type`) and the `SOURCE_*` codes of `security-source-type` (#45);
  `User::with_token_id`, written at `agent[user].policy`, and the claim of
  `IHE.BasicAudit.OAUTHaccessTokenUse.Comprehensive` when the record holds to
  it (#28); `User::local`, `with_issuer` and `with_client_id`, for a user no
  OAuth token named (#47); `Exchange::audit_event_value` (#57).
- `ihe-atna`: the optional DICOM elements `EventOutcomeDescription`,
  `AuditSourceTypeCode` (`message::SourceType`) and
  `ParticipantObjectDataLifeCycle` (#49), and `atna::codes`, the DICOM and
  RFC 3881 code tables (#52).
- `ihe-atna`: `balp::recording` with `Late` and `within`, the bound on an
  audited exchange's record (#8); public `redact::{REDACTED, RedactedUrl}`
  and `fhir_base::under_base` (#9); `atna::targets`, the stable `tracing`
  targets `ihe_atna::atna::spool` and `ihe_atna::atna::forwarder`, which
  every spool and forwarder event names (#10).
- `ihe-atna`: `feed::Credentials` and `FeedRepository::with_credentials`:
  each Feed request is authorized from the source, and a refused token is
  renewed once (RFC 6750 §3.1) (#53); `forwarder::Outbox` with
  `Forwarder::syslog_from` and `Forwarder::fhir_feed_from`, delivery from a
  caller's own durable store (#55); `Connection` is `Sync` (#51).
- `ehds-logging`: `map::Matching::OpenEhr` and `CategoryMap::declare_with`,
  case-insensitive template and archetype keys as openEHR BASE asks, with the
  evidence kept in its own case (#16); every defect of a declaration at once
  (`MapErrors`) (#17); archetype keys held to the `ARCHETYPE_ID` grammar
  (#18); `CategoryMap::templates`, `archetypes` and `national` (#19);
  `Declared` serializes (#20).
- `ehds-logging`: `classify::ResourceKind`, `Evidence::resource`,
  `RootObject::kind` and `Basis::ResourceKind`, so a record of no category
  says which resource it reached and a no-category object sits beside content
  (#21, #22, #24); `CategoryMap::with_evidence_limit` and
  `Classification::evidence_counts` (#23).
- `ehds-logging`: `Accessor::roles`, written at `agent[user].role` beside the
  acting mode, which `balp::exchange` no longer overwrites (#27);
  `Accessor::token_id` (#28); `AccessRecord::origin_count` (#32),
  `AccessRecord::outcome_description` and `Outcome::MajorFailure`;
  `Request::operation_system`, the operation written as a subtype (#36).
- `ehds-logging`: `retention::Period` (years, days or `Indefinite`) and
  `retention::Bounds` with national floors, ceilings and floors outside the
  Union, each with its legal ground, through `RetentionPolicy::declare_within`;
  a record kept without end names no end date, a period in days must reach the
  floor from any date of access, and every record names the ground of its
  floor (#37, #38, #39, #40).
- The vendored IETF syslog and audit RFCs, the openEHR BASE Base Types
  specification and the pin of DICOM PS3.15 2026c, read locally and never
  committed; doctests for the syslog sender, the FHIR Feed and the BALP sink
  (#13); `ehds-logging`'s own feature-boundary test (#12).

### Changed

- **`ihe-atna`: the `balp` feature no longer implies `atna`** and compiles no
  syslog sender, DICOM message or RFC 5425 TLS stack; the spool, the
  forwarder and the Feed compile under it. `ihe-atna` no longer enables
  `reqwest`'s `rustls` feature: a product enables its TLS backend on its own
  `reqwest` (#11). A product that used the syslog sender with `balp` enables
  `atna` too.
- **`ihe-atna`: `balp::Coded` holds `Cow<'static, str>`** and is no longer
  `Copy`; build one with `Coded::fixed` or `Coded::new`. `EventKind::subtypes`
  is a `Vec<Coded>` (#42).
- **`ihe-atna`: `Direction::Received::endpoint` is a `NetworkAddress`** (use
  `NetworkAddress::uri(&url)` where a URL was given) (#46); `Entity::Patient`
  takes an optional `system` and a `namespace` (#33); `Exchange` has
  `outcome_description` and `Observer` has `source_types`.
- **`ihe-atna`: `User::issuer` and `User::client_id` return `Option<&str>`**;
  `atna_user_name` of a user with no issuer is the account (#47).
- **`ihe-atna`: `FeedRepository::cleartext_for_development` is now
  `FeedRepository::cleartext`** (#54), and a `401` is `FeedError::Unauthorized`,
  retried and never quarantined (#53).
- **`ihe-atna`: `Forwarder` is generic over its store** (`Forwarder<O: Outbox
  = Spool>`), so `Forwarder` alone still names the spool forwarder (#55).
- **`ehds-logging`: `Accessor::issuer`, `client_id` and `acting` are
  `Option`** (#25, #26); `PatientIdentifier::namespace`, `Origin::endpoint`,
  `Origin::status` and `Request::id` are `Option` (#33, #31, #35); `Origin::key`
  is the origin's endpoint, else its `system_id`.
- **`ehds-logging`: `BalpSink::new` and `balp::exchange` take a
  `NetworkAddress`** in place of a `Url` (#46).
- **`ehds-logging`: `Retention::years` is now `Retention::period`**,
  `Retention::ends` returns `Option<Date>` and `RetentionPolicy::longest`
  returns a `Period`; the record writes `ehds-retention-period` and
  `ehds-retention-floor`, and `ehds-retention-years` and `ehds-retention-ends`
  only when they apply.

## [0.1.1] - 2026-10-20

A patch release with no API change: a product pins `=0.1.1` in place of
`=0.1.0` and changes nothing else.

### Fixed

- Both crates declare `rust-version = "1.97"`, the FerroHEALTH family floor,
  in place of 1.98, which no feature needed. A product whose MSRV is 1.97,
  such as FerroEHR, can now take them, and its `cargo hack check
  --rust-version` lane passes. CI checks the declared MSRV on every pull
  request with no feature and with `atna`, so a change that needs a newer
  toolchain fails before it merges. The `balp` feature of both crates takes
  `fhir-types`, which declares `rust-version = "1.98"` itself, so a build
  with `balp` still needs 1.98, and Cargo says so by naming `fhir-types`
  (#14).

## [0.1.0] - 2026-10-20

The first release: the shared IHE audit crate of the FerroHEALTH products,
moved with its history and its tests from the `atna` and `balp` features of
FerroFED's `ihe-iti`, the EHDS logging component `ehds-logging`, moved from
FerroFED with its tests, and the repository setup. A product that used
`ihe_iti::atna`, `ihe_iti::balp` or `ihe_iti::user` depends on
`ihe-atna = "=0.1.0"` with the same feature and reads the same items from
`ihe_atna::atna`, `ihe_atna::balp` and `ihe_atna::user`.

### Added

- `atna`: the sending half of ITI-20 Record Audit Event. The DICOM PS3.15
  Annex A.5 audit message with its participant objects held as secrets; the
  RFC 5424 syslog message with the PRI `<85>` and the MSGID `IHE+RFC-3881`
  ITI TF-2 §3.20.4.1.2 fixes, in its RFC 5425 frame; the TLS connection to an
  Audit Record Repository with the platform's roots and an optional client
  certificate; the bounded store-and-forward spool of §3.20.4.1.1, one
  fsynced file per message in a private directory, with a time bound on each
  write and a quarantine for an unreadable message; and the forwarder that
  stores first, delivers oldest first and retries with a jittered backoff.
- `balp`: the FHIR R4 `AuditEvent` of an IHE FHIR transaction, written from
  one `Exchange` to the BALP 1.1.4 patterns, with the user's agents and
  purposes of use; the `AuditRecorder` seam an audited client records
  through; and the ATX: FHIR Feed of the RESTful ATNA supplement, a FHIR
  `create` of each stored record, a refused record quarantined.
- `user`: whom an audited exchange is made for, compiled with no feature.
- `ehds-logging`, moved from FerroFED (0.0.26 there) onto this repository's
  version line: the logging software component of an EHR system under
  Regulation (EU) 2025/327, with the Annex II 3.2 access record, the Article
  14(1) categories and the map that classifies an access, the Article 9(2)
  retention, the Annex II 3.4 security labels, and the record written as a
  BALP `AuditEvent`. Its `balp` feature now builds on `ihe-atna` in place of
  `ihe-iti`, and `BalpSink::new` and `balp::exchange` name their URL
  parameter `base`. A product that used `ehds-logging` 0.0.26 pins
  `ehds-logging = "=0.1.0"`; the API is otherwise unchanged (#4).
- The repository setup: the design of record (`docs/architecture.md`), the
  pin matrix, the vendored ITI-20 page and RESTful ATNA supplement, the BALP
  package and pages, the EHDS Regulation with the acts and guidelines
  `ehds-logging` cites, the HL7 Europe and MyHealth@EU packages, CI with a single `conclusion` check, the vendored
  specification guard, CodeQL, Scorecard, Dependabot, and the release lane
  with crates.io Trusted Publishing.

[Unreleased]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.5.3...HEAD
[0.5.3]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.5.2...v0.5.3
[0.5.2]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.5.1...v0.5.2
[0.5.1]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.3.3...v0.4.0
[0.3.3]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.15...v0.3.0
[0.2.15]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.14...v0.2.15
[0.2.14]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.13...v0.2.14
[0.2.13]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.12...v0.2.13
[0.2.12]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.11...v0.2.12
[0.2.11]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.10...v0.2.11
[0.2.10]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.9...v0.2.10
[0.2.9]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.8...v0.2.9
[0.2.8]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.7...v0.2.8
[0.2.7]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.6...v0.2.7
[0.2.6]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/FerroHEALTH/FerroAUDIT/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/FerroHEALTH/FerroAUDIT/releases/tag/v0.1.0
