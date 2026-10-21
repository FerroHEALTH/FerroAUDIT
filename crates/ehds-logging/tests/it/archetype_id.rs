// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The category map's `ARCHETYPE_ID` check against the archetype-id parser of
//! `openehr-base`, the family's other reading of the same grammar.
//!
//! openEHR BASE 1.3.0, Base Types, Identification Package, Syntaxes. The map
//! keeps its own check so that it carries no 33,000-line generated crate for
//! one predicate; this test holds the two readings to one answer per id.

use std::collections::BTreeMap;
use std::str::FromStr;

use ehds_logging::map::{CategoryMap, Declared, MapError};
use openehr_base::v1_3::base_types::identification::archetype_id::ArchetypeId;

/// Ids from both families' grammar tests and the forms CKM publishes.
const CORPUS: &[&str] = &[
    "openEHR-EHR-COMPOSITION.encounter.v1",
    "openEHR-EHR-CLUSTER.laboratory_test_analyte.v2",
    "openEHR-EHR-OBSERVATION.blood_pressure-simple.v0",
    "openEHR-EHR-OBSERVATION.imaging_exam_result.v0",
    "openEHR-EHR-OBSERVATION.blood_pressure.v10",
    "openEHR-EHR-SECTION.physical_examination-prenatal.v1",
    "Hl7-RIM-Act.progress_note.v12",
    "org.openehr::openEHR-EHR-OBSERVATION.blood_pressure.v2.1.4",
    "com.nedap::openEHR-EHR-OBSERVATION.pulse.v1.1.3",
    "org.openehr::openEHR-EHR-OBSERVATION.blood_pressure.v2",
    "org.openehr::openEHR-EHR-OBSERVATION.blood_pressure.v2.1",
    "org.openehr::openEHR-EHR-EVALUATION.problem.v2.0.0-rc.4",
    "org.openehr::openEHR-EHR-EVALUATION.problem.v2.0.0-alpha",
    "org.open-ehr::openEHR-EHR-EVALUATION.problem.v1.0.0-beta.12",
    "openEHR-EHR-OBSERVATION.cbc.v1.0.0",
    "openEHR-EHR-COMPOSITION.cnf_adl2_versioned.v1.0.0",
    "openEHR-EHR-COMPOSITION.encounter.v1draft",
    "openEHR-EHR.encounter.v1",
    "openEHR-EHR-COMPOSITION.encounter",
    "openEHR-EHR-COMPOSITION.1concept.v1",
    "openEHR-EHR-COMPOSITION.encounter.v01",
    "openEHR-EHR-COMPOSITION.encounter.v1.01.0",
    "openEHR-EHR-COMPOSITION.encounter.v1.0.0.0",
    "openEHR-EHR-COMPOSITION.encounter.v1..0",
    "openEHR-EHR-COMPOSITION.encounter.vx",
    "openEHR-EHR-COMPOSITION.encounter.v",
    "openEHR-EHR-COMPOSITION.encounter.v1.0-rc.1",
    "openEHR-EHR-COMPOSITION.encounter.v1.0.0-rc",
    "openEHR-EHR-COMPOSITION.lab result.v1",
    "openEHR-EHR-COMPOSITION-X.encounter.v1",
    "openEHR-EHR-COMPOSITION.encounter-.v1",
    "org.openehr::openEHR-EHR-COMPOSITION.encounter.v1.0.0-gamma",
    "org.openehr::openEHR-EHR-COMPOSITION.encounter.v1.0.0-rc.",
    "org.openehr::openEHR-EHR-COMPOSITION.encounter.v1.0.0-rc.x",
    "::openEHR-EHR-COMPOSITION.encounter.v1",
    "org..openehr::openEHR-EHR-COMPOSITION.encounter.v1",
    "1org.openehr::openEHR-EHR-COMPOSITION.encounter.v1",
    "org.openehr::openEHR-EHR.encounter.v1",
    "org.openehr::",
    "1-2-3.@@@.vXYZ",
];

/// Whether the category map admits `id` as an archetype key.
fn admitted_by_the_map(id: &str) -> bool {
    let archetypes = BTreeMap::from([(
        id.to_owned(),
        Declared::Codes(vec!["Laboratory-Reports".to_owned()]),
    )]);
    match CategoryMap::declare(&[], &BTreeMap::new(), &archetypes) {
        Ok(_) => true,
        Err(MapError::ArchetypeId { .. }) => false,
        Err(other) => panic!("{id:?}: an unexpected refusal {other:?}"),
    }
}

#[test]
fn the_map_and_openehr_base_agree_on_every_archetype_id() {
    for id in CORPUS {
        assert_eq!(
            admitted_by_the_map(id),
            ArchetypeId::from_str(id).is_ok(),
            "{id:?}: the category map and openehr-base disagree"
        );
    }
}

#[test]
fn the_corpus_holds_admitted_and_refused_ids() {
    let admitted = CORPUS.iter().filter(|id| admitted_by_the_map(id)).count();
    assert!(admitted > 0 && admitted < CORPUS.len());
}
