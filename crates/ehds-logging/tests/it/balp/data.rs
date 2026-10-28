// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The data a read, create, update or delete accessed (`entity:data`, BALP
//! 1.1.4): a literal reference to a FHIR resource is `what.reference`, and
//! any other id, such as an openEHR version uid, is `what.identifier`, in
//! the namespace the recording system declares (FHIR R4 references.html
//! §2.3.0.2, §2.3.0.3).

use std::sync::Arc;

use ehds_logging::balp::{BalpSink, ObjectIds, exchange_with};
use ehds_logging::record::{AccessRecord, Action, Outcome};
use ehds_logging::sink::AccessSink as _;
use fhir_types::r4::schema::SCHEMAS;
use ihe_atna::balp::{AuditRecorder, Observer, is_literal_reference};
use secrecy::ExposeSecret as _;
use serde_json::Value;
use url::Url;

use super::{Kept, entity, gateway, record};

/// The operation the test records name, which is the data entity's `name`.
const OPERATION: &str = "composition_get_at_version";
/// An openEHR `OBJECT_VERSION_ID`, the id FerroEHR passes.
const VERSION_UID: &str = "01a18433-b15c-7695-bd26-eeca892cfa02::ferroehr.local::1";
/// A namespace a recording system issues its object ids in.
const NAMESPACE: &str = "https://cdr.example.org/fhir/NamingSystem/version-uid";

fn observer() -> Observer {
    Observer {
        source_id: "gateway.example.org".to_owned(),
        site: None,
        host: ihe_atna::balp::NetworkAddress::host("gateway.example.org"),
        source_types: Vec::new(),
    }
}

/// A record of `action` that addressed `resource`.
fn addressing(action: Action, resource: Option<&str>) -> AccessRecord {
    let mut record = record(action, Outcome::Success);
    record.request.operation = OPERATION.to_owned();
    record.request.resource = resource.map(str::to_owned);
    record.request.query = None;
    record
}

fn json(exchange: &ihe_atna::balp::Exchange) -> Value {
    let bytes = exchange
        .audit_event(&observer())
        .expect("a record")
        .into_bytes();
    serde_json::from_slice(bytes.expose_secret()).expect("JSON")
}

fn written_as(record: &AccessRecord, ids: &ObjectIds) -> Value {
    json(&exchange_with(record, &gateway(), ids))
}

/// The `what` of the record's data entity.
fn data(written: &Value) -> Value {
    let found = entity(written, OPERATION);
    assert_eq!(found.len(), 1, "one entity:data");
    let data = found.first().expect("entity:data");
    assert_eq!(data["type"]["code"], "2", "entity:data is a system object");
    data["what"].clone()
}

/// Whether `reference` is a literal reference as FHIR R4 references.html
/// §2.3.0.2 states it, read apart from the crate's own check: the relative
/// form of the published pattern, `[type]/[id]` with an optional
/// `/_history/[vid]`, or an absolute `http` or `https` URL.
fn r4_literal(reference: &str) -> bool {
    if let Ok(url) = Url::parse(reference) {
        return matches!(url.scheme(), "http" | "https") && url.host_str().is_some();
    }
    let id = |value: &str| {
        (1..=64).contains(&value.len())
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
    };
    let segments: Vec<&str> = reference.split('/').collect();
    let typed = |segments: &[&str]| {
        segments
            .first()
            .is_some_and(|name| SCHEMAS.resources.contains(name))
            && segments.get(1).is_some_and(|value| id(value))
    };
    match segments.as_slice() {
        [_, _] => typed(&segments),
        [_, _, "_history", version] => typed(&segments) && id(version),
        _ => false,
    }
}

/// Holds `what` to the R4 `Reference` rules: a `reference` is a literal
/// reference and no local one (invariant `ref-1`, the entity contains no
/// resource), and an `identifier` has a `value` and, when it names one, a
/// `system` that is an absolute URI (R4 `Identifier.system`).
fn assert_r4_reference(what: &Value) {
    let object = what.as_object().expect("what is a Reference");
    assert!(
        object.contains_key("reference") || object.contains_key("identifier"),
        "what names the data"
    );
    if let Some(reference) = what.get("reference") {
        let reference = reference.as_str().expect("a string");
        assert!(!reference.starts_with('#'), "no local reference");
        assert!(r4_literal(reference), "{reference} is no literal reference");
    }
    if let Some(identifier) = what.get("identifier") {
        let value = identifier["value"].as_str().expect("an identifier value");
        assert!(!value.is_empty(), "an identifier value");
        if let Some(system) = identifier.get("system") {
            let system = system.as_str().expect("a string");
            assert!(Url::parse(system).is_ok(), "{system} is an absolute URI");
        }
    }
}

#[test]
fn an_openehr_version_uid_is_an_identifier_never_a_reference() {
    for action in [Action::Read, Action::Create, Action::Update, Action::Delete] {
        let what = data(&written_as(
            &addressing(action, Some(VERSION_UID)),
            &ObjectIds::default(),
        ));
        assert!(what.get("reference").is_none(), "{action:?}");
        assert_eq!(what["identifier"]["value"], VERSION_UID, "{action:?}");
        assert!(what["identifier"].get("system").is_none(), "{action:?}");
        assert_r4_reference(&what);
    }
}

#[test]
fn ids_declared_in_a_namespace_are_identifiers_in_it_whatever_their_form() {
    let ids = ObjectIds::Identifiers {
        system: Some(Url::parse(NAMESPACE).expect("a URL")),
    };
    for id in [VERSION_UID, "Composition/looks-literal"] {
        let what = data(&written_as(&addressing(Action::Read, Some(id)), &ids));
        assert!(what.get("reference").is_none(), "{id}");
        assert_eq!(what["identifier"]["system"], NAMESPACE, "{id}");
        assert_eq!(what["identifier"]["value"], id, "{id}");
        assert_r4_reference(&what);
    }
}

#[test]
fn a_literal_reference_to_a_served_resource_stays_a_reference() {
    for reference in [
        "Composition/c-1",
        "Composition/c-1/_history/2",
        "https://cdr.example.org/fhir/Composition/c-1",
    ] {
        let what = data(&written_as(
            &addressing(Action::Read, Some(reference)),
            &ObjectIds::References,
        ));
        assert_eq!(what["reference"], reference);
        assert!(what.get("identifier").is_none(), "{reference}");
        assert_r4_reference(&what);
    }
}

#[test]
fn a_request_that_addressed_no_resource_names_its_operation_by_an_identifier() {
    let what = data(&written_as(
        &addressing(Action::Create, None),
        &ObjectIds::default(),
    ));
    assert!(what.get("reference").is_none());
    assert_eq!(what["identifier"]["value"], OPERATION);
    assert_r4_reference(&what);
}

#[test]
fn no_id_that_is_no_literal_reference_is_written_as_one() {
    for id in [
        VERSION_UID,
        "abc-def::ferroehr.local::1",
        "ehr/7d44b88c-4199-4bad-97dc-d78268e01398/composition/c-1",
        "NoSuchType/1",
        "Composition/",
        "Composition/a/b",
        "#contained",
        "urn:uuid:7d44b88c-4199-4bad-97dc-d78268e01398",
        "Composition/has space",
    ] {
        assert!(!r4_literal(id), "the oracle reads {id} as no literal");
        assert!(!is_literal_reference(id), "{id}");
        let what = data(&written_as(
            &addressing(Action::Read, Some(id)),
            &ObjectIds::References,
        ));
        assert!(what.get("reference").is_none(), "{id}");
        assert_eq!(what["identifier"]["value"], id, "{id}");
        assert_r4_reference(&what);
    }
}

#[tokio::test]
async fn the_sink_writes_the_ids_as_it_was_told() {
    let kept = Arc::new(Kept::default());
    let recorder: Arc<dyn AuditRecorder> = kept.clone();
    let sink = BalpSink::new(recorder, gateway()).with_object_ids(ObjectIds::Identifiers {
        system: Some(Url::parse(NAMESPACE).expect("a URL")),
    });
    sink.store(addressing(Action::Read, Some(VERSION_UID)))
        .await
        .expect("stored");
    sink.reserve()
        .await
        .expect("reserved")
        .store(addressing(Action::Update, Some(VERSION_UID)))
        .await
        .expect("stored");
    let kept = kept.kept.lock().expect("kept");
    assert_eq!(kept.len(), 2);
    for exchange in kept.iter() {
        let what = data(&json(exchange));
        assert_eq!(what["identifier"]["system"], NAMESPACE);
        assert_eq!(what["identifier"]["value"], VERSION_UID);
        assert_r4_reference(&what);
    }
}
