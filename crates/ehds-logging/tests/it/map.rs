// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The category map a deployment declares: every category names an Art 14(1)
//! category or a declared national one, `none` is written as such, and the
//! canonical text does not depend on the order of the declarations.
//!
//! The wire form is pinned here for every product that writes the same
//! record: the six codes of HL7 Europe's `EEHRxFDocumentPriorityCategoryCS`
//! (`hl7.fhir.eu.health-data-api` 1.0.0-ballot), compared exactly, and a
//! national category as `<system>|<code>`.

use std::collections::BTreeMap;

use ehds_logging::category::{
    Category, CodeError, NationalCategory, PRIORITY_SYSTEM, PRIORITY_VERSION,
};
use ehds_logging::map::{CategoryMap, Declared, MapError, Mapping, Table};

use super::support::{ADMIN, LAB_REPORT, NATIONAL, SUMMARY_ARCHETYPE, map};

fn one(key: &str, declared: Declared) -> BTreeMap<String, Declared> {
    BTreeMap::from([(key.to_owned(), declared)])
}

fn codes(codes: &[&str]) -> Declared {
    Declared::Codes(codes.iter().map(|code| (*code).to_owned()).collect())
}

#[test]
fn the_six_priority_categories_have_the_codes_of_the_hl7_europe_code_system() {
    let codes: Vec<&str> = Category::PRIORITY.iter().map(Category::code).collect();
    assert_eq!(
        codes,
        [
            "Patient-Summaries",
            "Electronic-Prescriptions",
            "Electronic-Dispensations",
            "Medical-Imaging",
            "Laboratory-Reports",
            "Discharge-Reports"
        ],
        "Art 14(1)(a) to (f), CodeSystem eehrxf-document-priority-category-cs"
    );
    assert_eq!(
        PRIORITY_SYSTEM,
        "http://hl7.eu/fhir/health-data-api/CodeSystem/eehrxf-document-priority-category-cs"
    );
    assert_eq!(PRIORITY_VERSION, "1.0.0-ballot");
    for category in Category::PRIORITY {
        assert_eq!(category.system(), PRIORITY_SYSTEM);
        assert_eq!(category.version(), Some(PRIORITY_VERSION));
        assert_eq!(
            category.token(),
            format!("{PRIORITY_SYSTEM}|{}", category.code())
        );
        assert_eq!(category.to_string(), category.token());
    }
}

#[test]
fn a_priority_code_is_compared_exactly() {
    for spelling in [
        "laboratory-reports",
        "LABORATORY-REPORTS",
        "Laboratory-Report",
    ] {
        assert_eq!(Category::priority(spelling), None, "{spelling}");
    }
    for old in [
        "patient-summary",
        "electronic-prescription",
        "electronic-dispensation",
        "medical-imaging",
        "medical-test-result",
        "discharge-report",
    ] {
        assert_eq!(Category::priority(old), None, "{old}");
    }
}

#[test]
fn a_mapped_key_and_none_read_back() {
    let map = map();
    assert_eq!(
        map.template(LAB_REPORT),
        Some(&Mapping::Categories([Category::MedicalTestResult].into()))
    );
    assert_eq!(map.template(ADMIN), Some(&Mapping::NoCategory));
    let national: NationalCategory = NATIONAL.parse().expect("a national category");
    assert_eq!(
        map.archetype(SUMMARY_ARCHETYPE),
        Some(&Mapping::Categories(
            [Category::PatientSummary, Category::National(national)].into()
        ))
    );
}

#[test]
fn a_priority_category_is_named_by_its_code_or_by_its_system_and_code() {
    let token = format!("{PRIORITY_SYSTEM}|Discharge-Reports");
    let map = CategoryMap::declare(
        &[],
        &one(LAB_REPORT, codes(&[&token, "Discharge-Reports"])),
        &BTreeMap::new(),
    )
    .expect("a map");
    assert_eq!(
        map.template(LAB_REPORT),
        Some(&Mapping::Categories([Category::DischargeReport].into()))
    );
    assert_eq!(map.category(&token), Some(Category::DischargeReport));
}

#[test]
fn a_national_category_is_named_by_its_system_and_code_only() {
    let map = map();
    let national: NationalCategory = NATIONAL.parse().expect("a national category");
    assert_eq!(map.category(NATIONAL), Some(Category::National(national)));
    assert_eq!(map.category("nl-example"), None, "a bare national code");
    assert_eq!(
        map.category("https://example.org/other|nl-example"),
        None,
        "the same code in another system"
    );
}

#[test]
fn a_code_that_names_no_category_is_refused() {
    let refused = CategoryMap::declare(
        &[],
        &one(LAB_REPORT, Declared::Codes(vec!["lab".to_owned()])),
        &BTreeMap::new(),
    );
    assert_eq!(
        refused,
        Err(MapError::Unknown {
            table: Table::Templates,
            key: LAB_REPORT.to_owned(),
            code: "lab".to_owned()
        })
    );
}

#[test]
fn a_code_of_the_priority_system_that_it_does_not_define_is_refused() {
    let token = format!("{PRIORITY_SYSTEM}|nl-example");
    let refused = CategoryMap::declare(
        &[],
        &one(LAB_REPORT, Declared::Codes(vec![token.clone()])),
        &BTreeMap::new(),
    );
    assert_eq!(
        refused,
        Err(MapError::Unknown {
            table: Table::Templates,
            key: LAB_REPORT.to_owned(),
            code: token
        })
    );
}

#[test]
fn an_empty_list_a_word_other_than_none_and_an_empty_key_are_refused() {
    let empty = CategoryMap::declare(
        &[],
        &BTreeMap::new(),
        &one(SUMMARY_ARCHETYPE, Declared::Codes(Vec::new())),
    );
    assert!(matches!(empty, Err(MapError::Empty { .. })), "{empty:?}");
    let word = CategoryMap::declare(
        &[],
        &one(ADMIN, Declared::Word("nothing".to_owned())),
        &BTreeMap::new(),
    );
    assert!(matches!(word, Err(MapError::Word { .. })), "{word:?}");
    let key = CategoryMap::declare(
        &[],
        &one("", Declared::Word("none".to_owned())),
        &BTreeMap::new(),
    );
    assert_eq!(key, Err(MapError::Key(Table::Templates)));
}

#[test]
fn a_national_category_in_the_priority_system_is_refused() {
    let declared = format!("{PRIORITY_SYSTEM}|nl-example");
    let refused = CategoryMap::declare(
        std::slice::from_ref(&declared),
        &BTreeMap::new(),
        &BTreeMap::new(),
    );
    assert_eq!(
        refused,
        Err(MapError::National {
            code: declared,
            source: CodeError::Priority
        })
    );
}

#[test]
fn a_national_category_without_a_system_is_refused() {
    assert_eq!(
        "nl-example".parse::<NationalCategory>(),
        Err(CodeError::NoSystem)
    );
    for system in ["", "example", "1urn:x", "urn:", "urn:ex ample", "urn:a|b"] {
        assert_eq!(
            NationalCategory::new(system, "nl-example"),
            Err(CodeError::System),
            "{system:?}"
        );
    }
}

#[test]
fn a_national_code_is_held_to_the_fhir_code_rules_only() {
    let system = "urn:oid:2.999.1";
    for code in ["Upper-Case", "a b", "x:y", "01", "é"] {
        assert!(NationalCategory::new(system, code).is_ok(), "{code:?}");
    }
    for code in ["", " a", "a ", "a  b", "a\tb", "a\nb", "a\u{1}b"] {
        assert_eq!(
            NationalCategory::new(system, code),
            Err(CodeError::Code),
            "{code:?}"
        );
    }
}

#[test]
fn the_canonical_text_names_every_declaration_in_order() {
    let lab = format!("{PRIORITY_SYSTEM}|Laboratory-Reports");
    let discharge = format!("{PRIORITY_SYSTEM}|Discharge-Reports");
    let summary = format!("{PRIORITY_SYSTEM}|Patient-Summaries");
    assert_eq!(
        map().canonical(),
        format!(
            "national\t{NATIONAL}\n\
             templates\tExample Admin Note.v1\tnone\n\
             templates\tExample Discharge.v1\t{discharge}\n\
             templates\tExample Lab Report.v1\t{lab}\n\
             archetypes\topenEHR-EHR-EVALUATION.problem_diagnosis.v1\t{summary},{NATIONAL}\n\
             archetypes\topenEHR-EHR-OBSERVATION.laboratory_test_result.v1\t{lab}\n"
        )
    );
}

#[test]
fn a_configuration_declares_the_map_in_toml() {
    #[derive(serde::Deserialize)]
    struct Table {
        templates: BTreeMap<String, Declared>,
    }
    let read: Table = toml::from_str(
        "[templates]\n\"Example Lab Report.v1\" = [\"Laboratory-Reports\"]\n\"Example Admin Note.v1\" = \"none\"\n",
    )
    .expect("TOML");
    let map = CategoryMap::declare(&[], &read.templates, &BTreeMap::new()).expect("a map");
    assert_eq!(map.template(ADMIN), Some(&Mapping::NoCategory));
    assert_eq!(
        map.template(LAB_REPORT),
        Some(&Mapping::Categories([Category::MedicalTestResult].into()))
    );
}

/// openEHR BASE 1.3.0, Base Types, Identification Package, "Composite
/// Identifiers and Case": identifiers are case-insensitive and
/// case-preserving.
#[test]
fn an_openehr_map_matches_an_id_apart_from_case_and_keeps_its_case() {
    use ehds_logging::classify::{Basis, Evidence, RootObject};
    use ehds_logging::map::Matching;

    let archetypes = one(
        "openEHR-EHR-OBSERVATION.lab_result.v1",
        codes(&["Laboratory-Reports"]),
    );
    let map = CategoryMap::declare_with(Matching::OpenEhr, &[], &BTreeMap::new(), &archetypes)
        .expect("a map");
    assert!(
        map.archetype("OPENEHR-EHR-OBSERVATION.LAB_RESULT.v1")
            .is_some()
    );
    let read = RootObject {
        archetype_id: Some("openehr-ehr-observation.LAB_result.v1".to_owned()),
        ..RootObject::default()
    };
    let classified = map.classify(&Evidence::reached(Basis::Returned, vec![read]));
    assert!(
        classified
            .categories()
            .contains_key(&Category::MedicalTestResult)
    );
    assert!(classified.unclassified().is_none());
    assert!(
        classified
            .archetypes()
            .contains("openehr-ehr-observation.LAB_result.v1"),
        "the evidence keeps the id as the access named it"
    );
    assert!(map.canonical().starts_with("matching\topenehr\n"));
}

#[test]
fn two_keys_that_differ_only_by_case_are_refused_in_an_openehr_map() {
    use ehds_logging::map::Matching;

    let templates = BTreeMap::from([
        (
            "Example Lab Report.v1".to_owned(),
            codes(&["Laboratory-Reports"]),
        ),
        (
            "example lab report.v1".to_owned(),
            codes(&["Discharge-Reports"]),
        ),
    ]);
    let refused = CategoryMap::declare_with(Matching::OpenEhr, &[], &templates, &BTreeMap::new())
        .expect_err("refused");
    assert!(
        matches!(
            refused.first(),
            Some(MapError::DuplicateKey {
                table: Table::Templates,
                ..
            })
        ),
        "{refused}"
    );
    assert!(
        CategoryMap::declare(&[], &templates, &BTreeMap::new()).is_ok(),
        "an exact map holds both"
    );
}

/// Every defect is reported in one pass, in table and key order.
#[test]
fn every_defect_of_a_declaration_is_reported_at_once() {
    use ehds_logging::map::Matching;

    let templates = BTreeMap::from([
        ("A.v1".to_owned(), Declared::Word("nothing".to_owned())),
        ("B.v1".to_owned(), codes(&[])),
    ]);
    let archetypes = one(
        "openEHR-EHR-OBSERVATION.lab_result",
        codes(&["Laboratory-Reports"]),
    );
    let refused = CategoryMap::declare_with(Matching::Exact, &[], &templates, &archetypes)
        .expect_err("refused");
    let errors = refused.errors();
    assert_eq!(errors.len(), 3, "{refused}");
    assert!(matches!(errors[0], MapError::Word { .. }), "{refused}");
    assert!(matches!(errors[1], MapError::Empty { .. }), "{refused}");
    assert!(
        matches!(errors[2], MapError::ArchetypeId { .. }),
        "{refused}"
    );
    assert_eq!(
        CategoryMap::declare(&[], &templates, &archetypes).err(),
        Some(errors[0].clone()),
        "declare returns the first"
    );
}

/// openEHR BASE 1.3.0, Base Types, Identification Package, Syntaxes: the
/// `ARCHETYPE_ID` grammar, whose major version may be `0` and whose ADL 2 form
/// carries a namespace and a full version.
#[test]
fn an_archetype_key_that_is_no_archetype_id_is_refused() {
    for key in [
        "openEHR-EHR-OBSERVATION.lab_result",
        "openEHR-EHR.lab_result.v1",
        "openEHR-EHR-OBSERVATION.lab_result.v01",
        "openEHR-EHR-OBSERVATION.lab_result.v1draft",
        "openEHR-EHR-OBSERVATION.lab result.v1",
        "openEHR-EHR-OBSERVATION.lab_result.v1.0-rc.1",
        "openEHR-EHR-OBSERVATION.lab_result.v1.0.0.0",
        "org.openehr::openEHR-EHR-OBSERVATION.lab_result.v1.0.0-gamma",
        "org..openehr::openEHR-EHR-OBSERVATION.lab_result.v1",
        "::openEHR-EHR-OBSERVATION.lab_result.v1",
    ] {
        assert_eq!(
            CategoryMap::declare(
                &[],
                &BTreeMap::new(),
                &one(key, codes(&["Laboratory-Reports"]))
            )
            .err(),
            Some(MapError::ArchetypeId {
                key: key.to_owned()
            }),
            "{key}"
        );
    }
    for key in [
        "openEHR-EHR-SECTION.physical_examination-prenatal.v1",
        "Hl7-RIM-Act.progress_note.v12",
        "openEHR-EHR-OBSERVATION.imaging_exam_result.v0",
        "org.openehr::openEHR-EHR-OBSERVATION.blood_pressure.v2.1.4",
        "org.openehr::openEHR-EHR-EVALUATION.problem.v2.0.0-rc.4",
        "openEHR-EHR-OBSERVATION.cbc.v1.0.0",
    ] {
        assert!(
            CategoryMap::declare(
                &[],
                &BTreeMap::new(),
                &one(key, codes(&["Laboratory-Reports"]))
            )
            .is_ok(),
            "{key}"
        );
    }
}

#[test]
fn a_map_iterates_its_keys_in_order_with_their_mappings() {
    let map = map();
    let templates: Vec<&str> = map.templates().map(|(key, _)| key).collect();
    assert_eq!(
        templates,
        ["Example Admin Note.v1", "Example Discharge.v1", LAB_REPORT]
    );
    assert!(
        map.templates()
            .any(|(key, mapping)| key == ADMIN && *mapping == Mapping::NoCategory)
    );
    assert_eq!(map.archetypes().count(), 2);
    let national: Vec<String> = map
        .national()
        .map(|category| format!("{}|{}", category.system(), category.code()))
        .collect();
    assert_eq!(national, [NATIONAL]);
}

#[test]
fn a_declaration_serializes_to_the_form_it_reads() {
    for declared in [
        Declared::Word("none".to_owned()),
        codes(&["Laboratory-Reports"]),
    ] {
        let text = serde_json::to_string(&declared).expect("serialized");
        let read: Declared = serde_json::from_str(&text).expect("read back");
        assert_eq!(read, declared, "{text}");
    }
    assert_eq!(
        serde_json::to_string(&Declared::Word("none".to_owned())).expect("serialized"),
        "\"none\""
    );
}

/// openEHR BASE 1.3.0, Base Types, Identification Package, "Composite
/// Identifiers and Case": two declarations that differ only in the case of a
/// key are one map, so they have one canonical text and one digest.
#[test]
fn an_openehr_map_has_one_canonical_text_whatever_the_case_of_its_keys() {
    use ehds_logging::map::Matching;

    let declare = |template: &str, archetype: &str| {
        CategoryMap::declare_with(
            Matching::OpenEhr,
            &[],
            &one(template, codes(&["Laboratory-Reports"])),
            &one(archetype, codes(&["Medical-Imaging"])),
        )
        .expect("a map")
        .canonical()
    };
    assert_eq!(
        declare(
            "Example Lab Report.v1",
            "openEHR-EHR-COMPOSITION.imaging_report.v1"
        ),
        declare(
            "example lab REPORT.v1",
            "OPENEHR-EHR-COMPOSITION.Imaging_Report.v1"
        )
    );
    let exact = |template: &str| {
        CategoryMap::declare(
            &[],
            &one(template, codes(&["Laboratory-Reports"])),
            &BTreeMap::new(),
        )
        .expect("a map")
        .canonical()
    };
    assert_ne!(
        exact("Example Lab Report.v1"),
        exact("example lab report.v1"),
        "an exact map writes each key as declared"
    );
}
