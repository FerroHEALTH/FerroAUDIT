// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The security labels of a record (Regulation (EU) 2025/327 Annex II 3.4):
//! the categories' on the `ehds-categories` entity alone, and each origin's
//! on its `origin` entity alone, never in `meta.security`.

use ehds_logging::category::{PRIORITY_SYSTEM, PRIORITY_VERSION};
use ehds_logging::classify::{Basis, Evidence, RootObject};
use ehds_logging::record::{Action, Outcome};
use serde_json::{Value, json};

use super::{ORIGIN_SYSTEM, entity, labels, record, written};
use crate::support::map;

/// The `Coding`s at `pointer` of `value`.
fn codings<'a>(value: &'a Value, pointer: &str) -> &'a [Value] {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

/// The lab report category's label: its own code, at its system's version.
fn lab() -> Value {
    json!({"system": PRIORITY_SYSTEM, "version": PRIORITY_VERSION, "code": "Laboratory-Reports"})
}

/// Node A's label: its endpoint id in the origin system.
fn node_a() -> Value {
    json!({"system": ORIGIN_SYSTEM, "code": "node-a"})
}

/// `RESTful` ATNA Rev. 3.6 Table 3.81.4.2.2.1-1 carries `entity.securityLabel`
/// and no `meta`, so every label rides on its entity alone and the record
/// writes no `meta.security` (#108, #111).
#[test]
fn the_record_writes_no_meta_security() {
    let written = written(&record(Action::Query, Outcome::Success));
    assert!(written.pointer("/meta/security").is_none(), "{written}");
    assert!(
        written["meta"]["profile"][0].is_string(),
        "the labels keep the pattern: {written}"
    );
}

/// `RESTful` ATNA Rev. 3.6 Table 3.20.4.2.2.1-1 maps `entity.securityLabel`
/// to the DICOM `ParticipantObjectSensitivity`, so a repository that keeps
/// the DICOM form keeps each label with the entity it belongs to.
#[test]
fn each_label_is_also_on_the_entity_it_belongs_to() {
    let written = written(&record(Action::Read, Outcome::Success));
    let categories = entity(&written, "ehds-categories");
    let [categories] = categories.as_slice() else {
        panic!("one categories entity: {written}");
    };
    assert_eq!(codings(categories, "/securityLabel"), [lab()]);
    let origins = entity(&written, "origin");
    let [origin] = origins.as_slice() else {
        panic!("one origin entity: {written}");
    };
    assert_eq!(codings(origin, "/securityLabel"), [node_a()]);
    for unlabelled in ["ehr"] {
        for entity in entity(&written, unlabelled) {
            assert!(entity.get("securityLabel").is_none(), "{entity}");
        }
    }
}

/// A failed delete claims no pattern, and its labels still reach the record.
#[test]
fn a_record_that_claims_no_pattern_is_labelled_all_the_same() {
    let written = written(&record(Action::Delete, Outcome::MinorFailure));
    assert!(written.pointer("/meta").is_none(), "{written}");
    let categories = entity(&written, "ehds-categories");
    assert_eq!(codings(categories[0], "/securityLabel"), [lab()]);
    let origins = entity(&written, "origin");
    assert_eq!(codings(origins[0], "/securityLabel"), [node_a()]);
}

/// Every label coding of `value` in a `security` or `securityLabel` list.
fn labels_in<'a>(value: &'a Value, found: &mut Vec<&'a Value>) {
    match value {
        Value::Object(members) => {
            for (name, member) in members {
                if matches!(name.as_str(), "security" | "securityLabel") {
                    found.extend(member.as_array().into_iter().flatten());
                } else {
                    labels_in(member, found);
                }
            }
        }
        Value::Array(items) => items.iter().for_each(|item| labels_in(item, found)),
        _ => {}
    }
}

/// An access the map could not classify carries every category's label
/// (Annex II 3.4), and each of them is written once, on the
/// `ehds-categories` entity, never again in `meta.security` (#108).
#[test]
fn every_category_label_of_an_unclassified_access_is_written_once_on_its_entity() {
    let unclassified = map().classify(&Evidence::reached(
        Basis::Returned,
        vec![RootObject {
            template_id: Some("synthetic.unmapped.v0".to_owned()),
            ..RootObject::default()
        }],
    ));
    let mut access = record(Action::Read, Outcome::Success);
    access.labels = labels().label(&unclassified, ["node-a"]);
    let written = written(&access);
    let mut found = Vec::new();
    labels_in(&written, &mut found);
    let categories: Vec<&str> = found
        .iter()
        .filter(|coding| coding["system"] == PRIORITY_SYSTEM)
        .filter_map(|coding| coding["code"].as_str())
        .collect();
    assert_eq!(
        6,
        categories.len(),
        "the six priority categories: {written}"
    );
    let mut once = categories.clone();
    once.sort_unstable();
    once.dedup();
    assert_eq!(
        categories.len(),
        once.len(),
        "each category once: {written}"
    );
    let entities = entity(&written, "ehds-categories");
    let [on_entity] = entities.as_slice() else {
        panic!("one categories entity: {written}");
    };
    let on_entity: Vec<&Value> = codings(on_entity, "/securityLabel").iter().collect();
    assert_eq!(
        6,
        on_entity
            .iter()
            .filter(|coding| coding["system"] == PRIORITY_SYSTEM)
            .count(),
        "{written}"
    );
    let elsewhere = found.len() - on_entity.len();
    assert_eq!(
        1, elsewhere,
        "only the origin's label, on its entity: {written}"
    );
    assert!(written.pointer("/meta/security").is_none(), "{written}");
}

/// Each origin's label is written once, on its `origin` entity, never again
/// in `meta.security` (#111).
#[test]
fn each_origin_label_is_written_once_on_its_entity() {
    let mut access = record(Action::Read, Outcome::Success);
    let mut second = access.origins[0].clone();
    second.endpoint = Some("node-b".to_owned());
    second.node = Some("cdr-b".to_owned());
    second.system_id = Some("cdr-b.example.org".to_owned());
    access.origins.push(second);
    access.labels = labels().label(&access.categories, ["node-a", "node-b"]);
    let written = written(&access);
    let mut found = Vec::new();
    labels_in(&written, &mut found);
    let origin_labels: Vec<&str> = found
        .iter()
        .filter(|coding| coding["system"] == ORIGIN_SYSTEM)
        .filter_map(|coding| coding["code"].as_str())
        .collect();
    assert_eq!(origin_labels, ["node-a", "node-b"], "{written}");
    for origin in entity(&written, "origin") {
        assert_eq!(codings(origin, "/securityLabel").len(), 1, "{origin}");
    }
    assert!(written.pointer("/meta/security").is_none(), "{written}");
}
