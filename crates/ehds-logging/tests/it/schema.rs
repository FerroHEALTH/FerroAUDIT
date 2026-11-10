// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The schema of a map declaration, as a product's configuration reference
//! reads it: the word `none` or a non-empty list of categories, held by a
//! JSON Schema 2020-12 validator.

use ehds_logging::map::{Declared, NONE};
use serde_json::json;

fn validator() -> jsonschema::Validator {
    jsonschema::draft202012::new(schemars::schema_for!(Declared).as_value())
        .expect("the schema compiles")
}

#[test]
fn none_and_a_list_of_categories_are_valid() {
    let validator = validator();
    for valid in [json!(NONE), json!(["P1", "urn:oid:1.2|X"])] {
        assert!(validator.is_valid(&valid), "{valid} is refused");
    }
}

#[test]
fn an_empty_list_and_another_word_are_refused() {
    let validator = validator();
    for invalid in [json!([]), json!("other"), json!([1])] {
        assert!(!validator.is_valid(&invalid), "{invalid} is accepted");
    }
}

#[test]
fn every_value_the_schema_admits_is_one_serde_reads() {
    for valid in [json!(NONE), json!(["P1"])] {
        let declared: Declared = serde_json::from_value(valid.clone()).expect("serde reads it");
        assert_eq!(serde_json::to_value(declared).expect("it writes"), valid);
    }
}

#[test]
fn the_schema_carries_the_description_of_the_type() {
    let schema = schemars::schema_for!(Declared);
    let description = schema
        .get("description")
        .and_then(|description| description.as_str())
        .expect("a description");
    assert!(description.contains("`none`"), "{description}");
    assert!(description.contains("`<system>|<code>`"), "{description}");
}
