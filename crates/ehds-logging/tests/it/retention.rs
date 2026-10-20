// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! How long a record is kept (Regulation (EU) 2025/327 Art 9(2), Annex II
//! 3.4): never under three years from the date of access, the longest its
//! categories and origins call for, and the longest declared anywhere for an
//! access that is unclassified.

use std::collections::BTreeMap;

use ehds_logging::category::Category;
use ehds_logging::classify::{Basis, Classification, Evidence, RootObject};
use ehds_logging::retention::{FLOOR_YEARS, Ground, RetentionError, RetentionPolicy, Years};
use jiff::Timestamp;
use jiff::civil::date;
use proptest::prelude::*;

use super::support::{DISCHARGE, LAB_REPORT, NATIONAL, UNMAPPED, map};

/// The classification of an access that returned compositions of each of
/// `templates`.
fn returned(templates: &[&str]) -> Classification {
    map().classify(&Evidence::reached(
        Basis::Returned,
        templates
            .iter()
            .map(|template| RootObject {
                template_id: Some((*template).to_owned()),
                ..RootObject::default()
            })
            .collect(),
    ))
}

fn table(entries: &[(&str, u16)]) -> BTreeMap<String, u16> {
    entries
        .iter()
        .map(|(key, years)| ((*key).to_owned(), *years))
        .collect()
}

/// The policy of `years` for every record, `categories` and `origins`.
fn policy(
    years: u16,
    categories: &[(&str, u16)],
    origins: &[(&str, u16)],
) -> Result<RetentionPolicy, RetentionError> {
    RetentionPolicy::declare(years, &table(categories), &table(origins), &map())
}

fn at(text: &str) -> Timestamp {
    text.parse().expect("a timestamp")
}

#[test]
fn the_default_policy_keeps_every_record_three_years() {
    let retention = RetentionPolicy::default().retain(
        at("2027-03-05T10:00:00Z"),
        &returned(&[LAB_REPORT]),
        ["node-a"],
    );
    assert_eq!(
        retention.period().years().expect("a period in years"),
        Years::FLOOR
    );
    assert_eq!(
        retention.period().years().expect("a period in years").get(),
        FLOOR_YEARS
    );
    assert_eq!(retention.ground(), &Ground::Default);
}

#[test]
fn a_period_under_three_years_is_refused_wherever_it_is_declared() {
    assert_eq!(
        policy(2, &[], &[]),
        Err(RetentionError::UnderFloor {
            key: "years".to_owned(),
            years: 2,
        })
    );
    assert_eq!(
        policy(3, &[("Laboratory-Reports", 1)], &[]),
        Err(RetentionError::UnderFloor {
            key: "categories.Laboratory-Reports".to_owned(),
            years: 1,
        })
    );
    assert_eq!(
        policy(3, &[], &[("node-a", 0)]),
        Err(RetentionError::UnderFloor {
            key: "origins.node-a".to_owned(),
            years: 0,
        })
    );
    assert_eq!(Years::new(2), None);
    assert_eq!(Years::new(3), Some(Years::FLOOR));
}

#[test]
fn a_category_the_map_does_not_declare_is_refused() {
    assert_eq!(
        policy(3, &[("nl-undeclared", 10)], &[]),
        Err(RetentionError::UnknownCategory {
            code: "nl-undeclared".to_owned(),
        })
    );
    let national = policy(3, &[(NATIONAL, 10)], &[]).expect("a declared national category");
    assert_eq!(
        national.longest().years().expect("a period in years").get(),
        10
    );
}

#[test]
fn an_empty_or_malformed_origin_is_refused() {
    assert_eq!(policy(3, &[], &[("", 5)]), Err(RetentionError::OriginKey));
    assert_eq!(
        policy(3, &[], &[("node\na", 5)]),
        Err(RetentionError::OriginKey)
    );
}

#[test]
fn a_record_is_kept_for_the_longest_its_categories_and_origins_call_for() {
    let policy = policy(
        5,
        &[("Laboratory-Reports", 10), ("Discharge-Reports", 20)],
        &[("node-a", 15), ("node-b", 30)],
    )
    .expect("a policy");
    let recorded = at("2027-03-05T10:00:00Z");
    let lab = policy.retain(recorded, &returned(&[LAB_REPORT]), ["node-c"]);
    assert_eq!(lab.period().years().expect("a period in years").get(), 10);
    assert_eq!(lab.ground(), &Ground::Category(Category::MedicalTestResult));
    let both = policy.retain(recorded, &returned(&[LAB_REPORT, DISCHARGE]), ["node-a"]);
    assert_eq!(both.period().years().expect("a period in years").get(), 20);
    assert_eq!(both.ground(), &Ground::Category(Category::DischargeReport));
    let origin = policy.retain(recorded, &returned(&[LAB_REPORT]), ["node-c", "node-a"]);
    assert_eq!(
        origin.period().years().expect("a period in years").get(),
        15
    );
    assert_eq!(origin.ground(), &Ground::Origin("node-a".to_owned()));
}

#[test]
fn a_period_no_longer_than_the_default_leaves_the_default_as_ground() {
    let policy = policy(10, &[("Laboratory-Reports", 10)], &[("node-a", 4)]).expect("a policy");
    let retention = policy.retain(
        at("2027-03-05T10:00:00Z"),
        &returned(&[LAB_REPORT]),
        ["node-a"],
    );
    assert_eq!(
        retention.period().years().expect("a period in years").get(),
        10
    );
    assert_eq!(retention.ground(), &Ground::Default);
}

/// An access the map cannot classify takes the longest period declared
/// anywhere, for a category or an origin the access did not reach too.
#[test]
fn an_unclassified_access_takes_the_longest_period_declared() {
    let policy = policy(5, &[("Laboratory-Reports", 10)], &[("node-b", 25)]).expect("a policy");
    assert_eq!(
        policy.longest().years().expect("a period in years").get(),
        25
    );
    let mixed = policy.retain(
        at("2027-03-05T10:00:00Z"),
        &returned(&[LAB_REPORT, UNMAPPED]),
        ["node-a"],
    );
    assert_eq!(mixed.period().years().expect("a period in years").get(), 25);
    assert_eq!(mixed.ground(), &Ground::Unclassified);
    assert_eq!(Ground::Unclassified.code(), "unclassified");
}

#[test]
fn the_record_may_be_deleted_from_the_day_after_its_period_ends() {
    let policy = RetentionPolicy::default();
    let lab = returned(&[LAB_REPORT]);
    let ends = |text: &str| {
        policy
            .retain(at(text), &lab, [])
            .ends()
            .expect("a finite period")
    };
    assert_eq!(ends("2027-03-05T10:00:00Z"), date(2030, 3, 6));
    // An access just after midnight east of UTC is dated by its UTC day.
    assert_eq!(ends("2027-03-06T00:30:00+02:00"), date(2030, 3, 6));
    // An access on a leap day is kept until the period after it has passed.
    assert_eq!(ends("2028-02-29T10:00:00Z"), date(2031, 3, 1));
}

#[test]
fn a_ground_is_written_by_its_code() {
    assert_eq!(Ground::Default.code(), "default");
    assert_eq!(
        Ground::Category(Category::DischargeReport).code(),
        "category:http://hl7.eu/fhir/health-data-api/CodeSystem/eehrxf-document-priority-category-cs|Discharge-Reports"
    );
    assert_eq!(Ground::Origin("node-a".to_owned()).code(), "origin:node-a");
}

proptest! {
    /// Whatever the policy declares, a classified record is kept for exactly
    /// the longest period among the default, its categories and its origins,
    /// never under three years, and its deletion date is past the date of
    /// access plus that many years.
    #[test]
    fn the_period_is_the_longest_applicable_and_never_under_the_floor(
        default in FLOOR_YEARS..60u16,
        lab in FLOOR_YEARS..60u16,
        discharge in FLOOR_YEARS..60u16,
        node_a in FLOOR_YEARS..60u16,
        reached_discharge in any::<bool>(),
        seconds in 0i64..4_000_000_000,
    ) {
        let policy = policy(
            default,
            &[("Laboratory-Reports", lab), ("Discharge-Reports", discharge)],
            &[("node-a", node_a)],
        )
        .expect("a policy");
        let templates: &[&str] = if reached_discharge {
            &[LAB_REPORT, DISCHARGE]
        } else {
            &[LAB_REPORT]
        };
        let recorded = Timestamp::from_second(seconds).expect("a timestamp");
        let retention = policy.retain(recorded, &returned(templates), ["node-a"]);
        let mut expected = default.max(lab).max(node_a);
        if reached_discharge {
            expected = expected.max(discharge);
        }
        prop_assert_eq!(retention.period().years().expect("a period in years").get(), expected);
        prop_assert!(retention.period().years().expect("a period in years") >= Years::FLOOR);
        let accessed = jiff::tz::Offset::UTC.to_datetime(recorded).date();
        let kept = accessed
            .checked_add(jiff::Span::new().years(i64::from(expected)))
            .expect("a date");
        prop_assert!(retention.ends().expect("a finite period") > kept);
    }
}

fn periods(
    entries: &[(&str, ehds_logging::retention::Period)],
) -> BTreeMap<String, ehds_logging::retention::Period> {
    entries
        .iter()
        .map(|(key, period)| ((*key).to_owned(), *period))
        .collect()
}

/// Art 9(2) sets a floor and no ceiling: a record may be kept without end,
/// and then names no end date.
#[test]
fn a_record_may_be_kept_without_end() {
    use ehds_logging::retention::{Bounds, FloorGround, Period};

    let policy = RetentionPolicy::declare_within(
        &Bounds::union(),
        Period::Indefinite,
        &BTreeMap::new(),
        &BTreeMap::new(),
        &map(),
    )
    .expect("a policy");
    let retention = policy.retain(at("2027-03-05T10:00:00Z"), &returned(&[LAB_REPORT]), []);
    assert_eq!(retention.period(), Period::Indefinite);
    assert_eq!(retention.ends(), None);
    assert_eq!(retention.floor(), &FloorGround::Union);
    assert_eq!(Period::Indefinite.code(), "indefinite");
}

/// A period in days stands over the three-year floor only when it reaches it
/// from any date of access: 1096 days, the most three years can span.
#[test]
fn a_period_in_days_must_reach_the_floor_from_any_date_of_access() {
    use ehds_logging::retention::{Bounds, Period};

    let declare = |days: u32| {
        RetentionPolicy::declare_within(
            &Bounds::union(),
            Period::Days(days),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &map(),
        )
    };
    assert!(
        matches!(declare(1095), Err(RetentionError::UnderBound { .. })),
        "three years of 365 days miss the floor across a leap day"
    );
    let policy = declare(1096).expect("the longest span of three years");
    let retention = policy.retain(at("2028-02-29T10:00:00Z"), &returned(&[LAB_REPORT]), []);
    assert_eq!(retention.period().code(), "P1096D");
    assert_eq!(
        retention.ends(),
        Some(date(2031, 3, 2)),
        "1096 days, then the day after"
    );
}

/// A deployment outside the Union keeps records under its own law's floor,
/// which may be under three years, and every record names that law, never
/// Art 9(2).
#[test]
fn a_floor_outside_the_union_is_named_and_may_be_under_three_years() {
    use ehds_logging::retention::{Bounds, FloorGround, Period};

    let bounds =
        Bounds::outside_union(Period::Years(1), "CH DSV Art 4 Abs 5".to_owned()).expect("bounds");
    let policy = RetentionPolicy::declare_within(
        &bounds,
        Period::Years(1),
        &BTreeMap::new(),
        &BTreeMap::new(),
        &map(),
    )
    .expect("a one-year policy outside the Union");
    let retention = policy.retain(at("2027-03-05T10:00:00Z"), &returned(&[LAB_REPORT]), []);
    assert_eq!(retention.period(), Period::Years(1));
    assert_eq!(
        retention.floor(),
        &FloorGround::OutsideUnion("CH DSV Art 4 Abs 5".to_owned())
    );
    assert_eq!(retention.floor().code(), "outside-union:CH DSV Art 4 Abs 5");
    assert!(
        matches!(
            RetentionPolicy::declare_within(
                &Bounds::union(),
                Period::Years(1),
                &BTreeMap::new(),
                &BTreeMap::new(),
                &map(),
            ),
            Err(RetentionError::UnderFloor { years: 1, .. })
        ),
        "inside the Union a year is refused"
    );
}

/// A national floor stands at or over Art 9(2), a ceiling over the floor,
/// and a period outside them is refused with its key and its ground.
#[test]
fn national_floors_and_ceilings_hold_every_period() {
    use ehds_logging::retention::{Bounds, FloorGround, Period};

    assert_eq!(
        Bounds::national(Period::Years(2), "a national act".to_owned()),
        Err(RetentionError::NationalUnderUnion {
            floor: Period::Years(2)
        })
    );
    assert_eq!(
        Bounds::national(Period::Indefinite, "a national act".to_owned()),
        Err(RetentionError::FloorIndefinite)
    );
    let five = Bounds::national(Period::Years(5), "NL logging retention decree".to_owned())
        .expect("a national floor");
    assert!(matches!(
        RetentionPolicy::declare_within(
            &five,
            Period::Years(4),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &map()
        ),
        Err(RetentionError::UnderBound { .. })
    ));
    let policy = RetentionPolicy::declare_within(
        &five,
        Period::Years(5),
        &BTreeMap::new(),
        &BTreeMap::new(),
        &map(),
    )
    .expect("five years");
    assert_eq!(
        policy
            .retain(at("2027-03-05T10:00:00Z"), &returned(&[LAB_REPORT]), [])
            .floor(),
        &FloorGround::National("NL logging retention decree".to_owned())
    );

    let capped = Bounds::union()
        .with_ceiling(Period::Years(3), "DE SGB V § 309".to_owned())
        .expect("a ceiling at the floor");
    for (period, key) in [
        (Period::Years(4), "categories.Laboratory-Reports"),
        (Period::Indefinite, "categories.Laboratory-Reports"),
    ] {
        let refused = RetentionPolicy::declare_within(
            &capped,
            Period::Years(3),
            &periods(&[("Laboratory-Reports", period)]),
            &BTreeMap::new(),
            &map(),
        );
        assert!(
            matches!(&refused, Err(RetentionError::OverCeiling { key: refused_key, ground, .. }) if refused_key == key && ground == "DE SGB V § 309"),
            "{refused:?}"
        );
    }
    assert_eq!(
        Bounds::union().with_ceiling(Period::Years(2), "an act".to_owned()),
        Err(RetentionError::CeilingUnderFloor {
            ceiling: Period::Years(2),
            floor: Period::Years(3)
        })
    );
}

/// An origin of stored data that names no endpoint is keyed by its
/// `system_id`.
#[test]
fn an_origin_is_kept_by_its_key() {
    let policy = policy(3, &[], &[("lab.example.org", 12)]).expect("a policy");
    let retention = policy.retain(
        at("2027-03-05T10:00:00Z"),
        &returned(&[LAB_REPORT]),
        ["lab.example.org"],
    );
    assert_eq!(
        retention.ground(),
        &Ground::Origin("lab.example.org".to_owned())
    );
    assert_eq!(retention.period().code(), "P12Y");
}
