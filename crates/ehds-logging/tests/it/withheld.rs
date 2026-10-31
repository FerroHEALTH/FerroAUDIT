// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The person's own access trail under a hold (Regulation (EU) 2025/327 Art
//! 3(3), Art 9): a record about a held resource is left out of the person's
//! read until the hold is released or expires, then shown unchanged with its
//! original time; professional and supervisory reads see every record.

use ehds_logging::classify::{Basis, Evidence, RootObject};
use ehds_logging::label::Labels;
use ehds_logging::map::CategoryMap;
use ehds_logging::record::{
    AccessRecord, Accessor, Action, DataSubject, Origin, Outcome, Request, WrittenVersion,
};
use ehds_logging::retention::RetentionPolicy;
use ehds_logging::withheld::{Hold, HoldError, Holds, Reader};
use jiff::{SignedDuration, Timestamp};

use super::support::{LAB_REPORT, map};

/// The versioned object a professional holds, and two of its versions.
const HELD: &str = "6c1d0b8e-41a7-4e55-9a51-0f3c2b7d5e10";
const HELD_V1: &str = "6c1d0b8e-41a7-4e55-9a51-0f3c2b7d5e10::cdr.example.org::1";
const HELD_V2: &str = "6c1d0b8e-41a7-4e55-9a51-0f3c2b7d5e10::cdr.example.org::2";
/// A version of another object, never held.
const OTHER_V1: &str = "0a9e7f3c-2b18-4d6a-8c4e-5f1d7b2a9c33::cdr.example.org::1";

fn at(hours: i64) -> Timestamp {
    let registered: Timestamp = "2026-10-31T09:00:00Z".parse().expect("a timestamp");
    registered + SignedDuration::from_hours(hours)
}

/// A read of `resource`, recorded at `recorded`, that reached `reached`.
fn read_with(
    map: &CategoryMap,
    resource: Option<&str>,
    reached: &[&str],
    recorded: Timestamp,
) -> AccessRecord {
    let objects = reached
        .iter()
        .map(|version| RootObject {
            template_id: Some(LAB_REPORT.to_owned()),
            version_uid: Some((*version).to_owned()),
            ..RootObject::default()
        })
        .collect();
    let categories = map.classify(&Evidence::reached(Basis::Returned, objects));
    let retention = RetentionPolicy::default().retain(recorded, &categories, ["node-a"]);
    AccessRecord {
        action: if resource.is_some() {
            Action::Read
        } else {
            Action::Query
        },
        recorded,
        outcome: Outcome::Success,
        outcome_description: None,
        accessor: Accessor {
            issuer: None,
            subject: "Qz7-professional-61".to_owned(),
            client_id: None,
            audience: None,
            provider: None,
            acting: None,
            roles: Vec::new(),
            token_id: None,
            assurance: None,
            professional: ehds_logging::record::Professional::default(),
            alt_id: None,
            purposes: Vec::new(),
            policies: Vec::new(),
            relayed: None,
        },
        subject: DataSubject::default(),
        categories,
        delivered: Some(reached.len()),
        origins: vec![Origin {
            endpoint: Some("node-a".to_owned()),
            node: None,
            system_id: None,
            status: None,
            rows: None,
            version: None,
            created_ehr: None,
            categories: None,
        }],
        origin_count: None,
        request: Request {
            id: None,
            operation: "composition_get".to_owned(),
            operation_system: None,
            resource: resource.map(str::to_owned),
            query: None,
            stored_query: None,
            client_address: None,
            correlation: None,
        },
        retention,
        labels: Labels::default(),
        emergency: None,
        inserted: None,
        transformations: Vec::new(),
    }
}

fn read(resource: &str, recorded: Timestamp) -> AccessRecord {
    read_with(&map(), Some(resource), &[resource], recorded)
}

/// The register after a professional held [`HELD`] for 72 hours at `at(0)`.
fn held() -> Holds {
    let mut holds = Holds::default();
    holds.hold(HELD, at(72));
    holds
}

/// The `recorded` time of each record `reader` sees at `now`.
fn seen(holds: &Holds, reader: Reader, records: &[AccessRecord], now: Timestamp) -> Vec<Timestamp> {
    holds
        .trail(reader, records, now)
        .map(|record| record.recorded)
        .collect()
}

#[test]
fn before_release_the_person_does_not_see_records_about_the_held_resource() {
    let records = [read(HELD_V1, at(1)), read(OTHER_V1, at(2))];
    assert_eq!(seen(&held(), Reader::Person, &records, at(3)), [at(2)]);
}

#[test]
fn after_release_the_person_sees_them_with_their_original_times() {
    let records = [read(HELD_V1, at(1)), read(OTHER_V1, at(2))];
    let mut holds = held();
    holds.release(HELD, at(5)).expect("the hold");
    assert_eq!(seen(&holds, Reader::Person, &records, at(4)), [at(2)]);
    assert_eq!(
        seen(&holds, Reader::Person, &records, at(5)),
        [at(1), at(2)]
    );
}

#[test]
fn after_expiry_the_person_sees_them_with_their_original_times() {
    let records = [read(HELD_V1, at(1)), read(HELD_V2, at(70))];
    let holds = held();
    assert_eq!(
        seen(&holds, Reader::Person, &records, at(71)),
        Vec::<Timestamp>::new()
    );
    assert_eq!(
        seen(&holds, Reader::Person, &records, at(72)),
        [at(1), at(70)]
    );
}

#[test]
fn a_professional_and_a_supervisor_see_every_record_while_held() {
    let records = [read(HELD_V1, at(1)), read(OTHER_V1, at(2))];
    for reader in [Reader::Professional, Reader::Supervisory] {
        assert_eq!(
            seen(&held(), reader, &records, at(3)),
            [at(1), at(2)],
            "{reader:?}"
        );
    }
}

#[test]
fn a_hold_on_the_object_covers_each_of_its_versions_and_the_object_itself() {
    let holds = held();
    for resource in [HELD, HELD_V1, HELD_V2] {
        assert!(holds.withholds(resource, at(1)), "{resource}");
    }
    assert!(!holds.withholds(OTHER_V1, at(1)));
}

#[test]
fn a_hold_on_one_version_leaves_the_other_versions_shown() {
    let mut holds = Holds::default();
    holds.hold(HELD_V2, at(72));
    assert!(holds.withholds(HELD_V2, at(1)));
    assert!(!holds.withholds(HELD_V1, at(1)));
    assert!(!holds.withholds(HELD, at(1)));
}

#[test]
fn a_query_that_reached_the_held_version_is_withheld_from_the_person() {
    let query = read_with(&map(), None, &[OTHER_V1, HELD_V1], at(1));
    let holds = held();
    assert!(!holds.shows(Reader::Person, &query, at(2)));
    assert!(holds.shows(Reader::Professional, &query, at(2)));
}

#[test]
fn a_write_that_left_a_version_of_the_held_object_is_withheld_from_the_person() {
    let mut write = read_with(&map(), None, &[], at(1));
    write.action = Action::Update;
    if let Some(origin) = write.origins.first_mut() {
        origin.version = Some(WrittenVersion::Named(HELD_V2.to_owned()));
    }
    let holds = held();
    assert!(!holds.shows(Reader::Person, &write, at(2)));
    assert!(holds.shows(Reader::Person, &write, at(72)));
}

#[test]
fn an_origin_that_reached_the_held_version_is_withheld_from_the_person() {
    let mut query = read_with(&map(), None, &[OTHER_V1], at(1));
    let reached = read_with(&map(), None, &[HELD_V1], at(1)).categories;
    if let Some(origin) = query.origins.first_mut() {
        origin.categories = Some(reached);
    }
    assert!(!held().shows(Reader::Person, &query, at(2)));
}

/// A record whose versions the evidence limit cut may name the held
/// resource among those it dropped, so the person sees it only once no hold
/// is in force.
#[test]
fn a_record_whose_versions_were_cut_waits_for_every_hold_to_end() {
    let cut = map().with_evidence_limit(1);
    let query = read_with(
        &cut,
        None,
        &[OTHER_V1, "1f2e3d4c::cdr.example.org::1"],
        at(1),
    );
    assert!(
        query.categories.evidence_counts().versions > query.categories.versions().len(),
        "the limit cut the versions"
    );
    let holds = held();
    assert!(!holds.shows(Reader::Person, &query, at(2)));
    assert!(holds.shows(Reader::Professional, &query, at(2)));
    assert!(holds.shows(Reader::Person, &query, at(72)));
    assert!(Holds::default().shows(Reader::Person, &query, at(2)));
}

#[test]
fn releasing_a_resource_that_is_not_held_is_refused() {
    let mut holds = held();
    assert_eq!(holds.release(OTHER_V1, at(1)), Err(HoldError::NotHeld));
    assert_eq!(holds.release(HELD_V1, at(1)), Err(HoldError::NotHeld));
}

#[test]
fn a_second_hold_in_force_extends_to_the_later_end() {
    let mut holds = held();
    holds.hold(HELD, at(24));
    assert_eq!(holds.get(HELD), Some(&Hold::new(at(72))));
    holds.hold(HELD, at(96));
    assert_eq!(holds.get(HELD).map(Hold::expires), Some(at(96)));
}

#[test]
fn a_hold_after_a_release_withholds_again() {
    let mut holds = held();
    holds.release(HELD, at(2)).expect("the hold");
    assert!(!holds.withholds(HELD, at(3)));
    holds.hold(HELD, at(48));
    assert!(holds.withholds(HELD, at(3)));
    assert_eq!(holds.get(HELD).and_then(Hold::released), None);
}

#[test]
fn a_hold_ends_at_its_earlier_release_or_its_expiry() {
    let mut hold = Hold::new(at(72));
    assert_eq!(hold.ends(), at(72));
    hold.release(at(10));
    hold.release(at(20));
    assert_eq!(hold.released(), Some(at(10)));
    assert_eq!(hold.ends(), at(10));
    let mut late = Hold::new(at(72));
    late.release(at(80));
    assert_eq!(late.ends(), at(72));
    assert!(!late.withholds(at(72)));
}

#[test]
fn a_shown_record_is_the_record_as_stored() {
    let records = [read(HELD_V1, at(1))];
    let mut holds = held();
    holds.release(HELD, at(5)).expect("the hold");
    let shown: Vec<&AccessRecord> = holds.trail(Reader::Person, &records, at(6)).collect();
    let [record] = shown.as_slice() else {
        panic!("one record shown, got {}", shown.len());
    };
    assert_eq!(record.recorded, at(1));
    assert_eq!(record.request.resource.as_deref(), Some(HELD_V1));
}
