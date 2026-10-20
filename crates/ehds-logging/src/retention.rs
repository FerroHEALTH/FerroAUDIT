// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! How long an access record is kept (Regulation (EU) 2025/327 Art 9(2),
//! Annex II 3.4).
//!
//! Art 9(2) keeps the information on each access "available for at least
//! three years from each date of access", and Annex II 3.4 asks for
//! "different retention periods ... that take into account the origins and
//! categories" of the data. A [`RetentionPolicy`] declares a [`Period`] for
//! every record, per category and per origin, within [`Bounds`]: inside the
//! Union never under [`FLOOR_YEARS`], over a national floor and under a
//! ceiling when the deployment names them, and outside the Union over the
//! floor its own law sets, which every record then names as its ground in
//! place of Art 9(2). A record is kept for the longest period its categories
//! and origins call for, and an access the map could not classify for the
//! longest the policy declares anywhere, so no record is kept for less than
//! one of its parts needs (no specification governs how origin and category
//! combine: our own design).
//!
//! The crate carries no table of national law: the deployment knows where it
//! runs and which law binds it, and states each bound with its legal ground.
//!
//! The store that holds the records applies the period: the record states it
//! as a [`Retention`], with the first date it may be deleted, or none for a
//! record kept without end.

use std::collections::BTreeMap;
use std::fmt;

use jiff::civil::Date;
use jiff::tz::Offset;
use jiff::{Span, Timestamp};

use crate::category::Category;
use crate::classify::Classification;
use crate::map::CategoryMap;

/// The fewest years a record is kept inside the Union: "at least three years
/// from each date of access" (Art 9(2)).
pub const FLOOR_YEARS: u16 = 3;

/// A retention period in whole years, never under [`FLOOR_YEARS`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Years(u16);

impl Years {
    /// The floor of Art 9(2), three years.
    pub const FLOOR: Self = Self(FLOOR_YEARS);

    /// The period of `years`, when it is not under the floor of Art 9(2).
    #[must_use]
    pub const fn new(years: u16) -> Option<Self> {
        if years < FLOOR_YEARS {
            None
        } else {
            Some(Self(years))
        }
    }

    /// The number of years.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

/// How long a record is kept from the date of access.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Period {
    /// Whole years.
    Years(u16),
    /// Days.
    Days(u32),
    /// Without end: Art 9(2) sets a floor, never a ceiling, so a record may
    /// be kept for as long as the deployment keeps it.
    Indefinite,
}

impl Period {
    /// The years of a period in years that stands over the floor of Art
    /// 9(2).
    #[must_use]
    pub fn years(self) -> Option<Years> {
        match self {
            Self::Years(years) => Years::new(years),
            Self::Days(_) | Self::Indefinite => None,
        }
    }

    /// The period as a record writes it: an ISO 8601 duration (`P3Y`,
    /// `P1100D`), or `indefinite`.
    #[must_use]
    pub fn code(self) -> String {
        match self {
            Self::Years(years) => format!("P{years}Y"),
            Self::Days(days) => format!("P{days}D"),
            Self::Indefinite => "indefinite".to_owned(),
        }
    }

    /// The fewest days the period spans from any date of access; `None`
    /// without end.
    fn fewest_days(self) -> Option<u64> {
        match self {
            Self::Years(years) => Some(u64::from(years).saturating_mul(365)),
            Self::Days(days) => Some(u64::from(days)),
            Self::Indefinite => None,
        }
    }

    /// The most days the period can span from a date of access: any run of
    /// years holds at most one leap day in four, rounded up; `None` without
    /// end.
    fn most_days(self) -> Option<u64> {
        match self {
            Self::Years(years) => {
                let years = u64::from(years);
                Some(years.saturating_mul(365).saturating_add(years.div_ceil(4)))
            }
            Self::Days(days) => Some(u64::from(days)),
            Self::Indefinite => None,
        }
    }

    /// Whether this period reaches `floor` from every date of access.
    fn reaches(self, floor: Self) -> bool {
        if let (Self::Years(kept), Self::Years(needed)) = (self, floor) {
            return kept >= needed;
        }
        match (self.fewest_days(), floor.most_days()) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(kept), Some(needed)) => kept >= needed,
        }
    }

    /// Whether this period stays within `ceiling` from every date of access.
    fn within(self, ceiling: Self) -> bool {
        if let (Self::Years(kept), Self::Years(limit)) = (self, ceiling) {
            return kept <= limit;
        }
        match (self.most_days(), ceiling.fewest_days()) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(kept), Some(limit)) => kept <= limit,
        }
    }

    /// The first UTC date a record of an access on `accessed` kept for this
    /// period may be deleted on: the date of access plus the period, plus
    /// one day; `None` for a period without end.
    fn ends(self, accessed: Date) -> Option<Date> {
        let span = match self {
            Self::Years(years) => Span::new()
                .try_years(i64::from(years))
                .and_then(|span| span.try_days(1)),
            Self::Days(days) => Span::new().try_days(i64::from(days).saturating_add(1)),
            Self::Indefinite => return None,
        };
        // NOTE: Art 9(2) sets a floor, so a date past the calendar's end keeps the record longer,
        // never shorter; the added day keeps a leap day's access its full period.
        Some(span.map_or(Date::MAX, |span| accessed.saturating_add(span)))
    }

    /// Whether `other` keeps a record of an access on `accessed` longer
    /// than this period does.
    fn outlasted_by(self, other: Self, accessed: Date) -> bool {
        match (self.ends(accessed), other.ends(accessed)) {
            (None, _) => false,
            (Some(_), None) => true,
            (Some(ours), Some(theirs)) => theirs > ours,
        }
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.code())
    }
}

/// On what legal ground a record's floor stands.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FloorGround {
    /// Art 9(2) of Regulation (EU) 2025/327: three years from each date of
    /// access.
    Union,
    /// A national law of a Member State that keeps records longer than Art
    /// 9(2), as the deployment cites it.
    National(String),
    /// The law of a deployment outside the Union, as the deployment cites
    /// it, which Art 9(2) does not bind; its floor may be under three years.
    OutsideUnion(String),
}

impl FloorGround {
    /// The ground as a record writes it: `eu-2025-327-art-9-2`,
    /// `national:<citation>` or `outside-union:<citation>`.
    #[must_use]
    pub fn code(&self) -> String {
        match self {
            Self::Union => "eu-2025-327-art-9-2".to_owned(),
            Self::National(citation) => format!("national:{citation}"),
            Self::OutsideUnion(citation) => format!("outside-union:{citation}"),
        }
    }
}

/// The bounds a retention policy is declared within: a floor with its legal
/// ground, and a ceiling with its legal ground when a law sets one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bounds {
    floor: Period,
    floor_ground: FloorGround,
    ceiling: Option<(Period, String)>,
}

impl Default for Bounds {
    fn default() -> Self {
        Self::union()
    }
}

impl Bounds {
    /// The bounds of a deployment inside the Union with no national law of
    /// its own: the three years of Art 9(2), and no ceiling.
    #[must_use]
    pub fn union() -> Self {
        Self {
            floor: Period::Years(FLOOR_YEARS),
            floor_ground: FloorGround::Union,
            ceiling: None,
        }
    }

    /// The bounds of a deployment inside the Union whose national law keeps
    /// records at least `floor`, cited as `ground`.
    ///
    /// # Errors
    ///
    /// [`RetentionError::NationalUnderUnion`] for a floor that does not
    /// reach the three years of Art 9(2) from every date of access, and
    /// [`RetentionError::FloorIndefinite`] for a floor without end.
    pub fn national(floor: Period, ground: String) -> Result<Self, RetentionError> {
        if floor == Period::Indefinite {
            return Err(RetentionError::FloorIndefinite);
        }
        if !floor.reaches(Period::Years(FLOOR_YEARS)) {
            return Err(RetentionError::NationalUnderUnion { floor });
        }
        Ok(Self {
            floor,
            floor_ground: FloorGround::National(ground),
            ceiling: None,
        })
    }

    /// The bounds of a deployment outside the Union, which keeps records at
    /// least `floor` under its own law, cited as `ground`; the floor may be
    /// under three years, and every record names `ground`, never Art 9(2).
    ///
    /// # Errors
    ///
    /// [`RetentionError::FloorIndefinite`] for a floor without end.
    pub fn outside_union(floor: Period, ground: String) -> Result<Self, RetentionError> {
        if floor == Period::Indefinite {
            return Err(RetentionError::FloorIndefinite);
        }
        Ok(Self {
            floor,
            floor_ground: FloorGround::OutsideUnion(ground),
            ceiling: None,
        })
    }

    /// These bounds, under a law that keeps records at most `ceiling`, cited
    /// as `ground`.
    ///
    /// # Errors
    ///
    /// [`RetentionError::CeilingUnderFloor`] for a ceiling that does not
    /// reach the floor from every date of access.
    pub fn with_ceiling(mut self, ceiling: Period, ground: String) -> Result<Self, RetentionError> {
        if !ceiling.reaches(self.floor) {
            return Err(RetentionError::CeilingUnderFloor {
                ceiling,
                floor: self.floor,
            });
        }
        self.ceiling = Some((ceiling, ground));
        Ok(self)
    }

    /// The floor.
    #[must_use]
    pub fn floor(&self) -> Period {
        self.floor
    }

    /// The legal ground of the floor.
    #[must_use]
    pub fn floor_ground(&self) -> &FloorGround {
        &self.floor_ground
    }

    /// The ceiling and its legal ground, when a law sets one.
    #[must_use]
    pub fn ceiling(&self) -> Option<(Period, &str)> {
        self.ceiling
            .as_ref()
            .map(|(period, ground)| (*period, ground.as_str()))
    }

    /// `period`, declared at `key`, when it stands within these bounds.
    fn admit(&self, key: String, period: Period) -> Result<Period, RetentionError> {
        if !period.reaches(self.floor) {
            if let (Period::Years(years), FloorGround::Union) = (period, &self.floor_ground) {
                return Err(RetentionError::UnderFloor { key, years });
            }
            return Err(RetentionError::UnderBound {
                key,
                period,
                floor: self.floor,
                ground: self.floor_ground.code(),
            });
        }
        if let Some((ceiling, ground)) = &self.ceiling
            && !period.within(*ceiling)
        {
            return Err(RetentionError::OverCeiling {
                key,
                period,
                ceiling: *ceiling,
                ground: ground.clone(),
            });
        }
        Ok(period)
    }
}

/// What called for a record's period.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Ground {
    /// The period every record is kept for.
    Default,
    /// The access is unclassified, so it is kept for the longest period
    /// declared anywhere.
    Unclassified,
    /// One of the categories of the data accessed.
    Category(Category),
    /// One of the origins of the data, by its key: its endpoint, or the
    /// `system_id` of an origin that names no endpoint.
    Origin(String),
}

impl Ground {
    /// The ground as a record writes it: `default`, `unclassified`,
    /// `category:<system>|<code>` or `origin:<key>`.
    #[must_use]
    pub fn code(&self) -> String {
        match self {
            Self::Default => "default".to_owned(),
            Self::Unclassified => "unclassified".to_owned(),
            Self::Category(category) => format!("category:{}", category.token()),
            Self::Origin(key) => format!("origin:{key}"),
        }
    }
}

/// How long one record is kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retention {
    period: Period,
    ground: Ground,
    floor: FloorGround,
    ends: Option<Date>,
}

impl Retention {
    /// The period.
    #[must_use]
    pub fn period(&self) -> Period {
        self.period
    }

    /// What called for it.
    #[must_use]
    pub fn ground(&self) -> &Ground {
        &self.ground
    }

    /// The legal ground of the floor the period was declared over.
    #[must_use]
    pub fn floor(&self) -> &FloorGround {
        &self.floor
    }

    /// The first date, in UTC, on which the record may be deleted: the date
    /// of access plus the period, plus one day; `None` for a record kept
    /// without end.
    #[must_use]
    pub fn ends(&self) -> Option<Date> {
        self.ends
    }
}

/// Why a retention policy or its bounds were refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RetentionError {
    /// A period in years is under the floor of Art 9(2).
    #[error(
        "{key} keeps a record {years} years, under the {FLOOR_YEARS} years from each date of access Regulation (EU) 2025/327 Art 9(2) asks for"
    )]
    UnderFloor {
        /// The key that declares it: `years`, `categories.<code>` or
        /// `origins.<key>`.
        key: String,
        /// The period declared.
        years: u16,
    },
    /// A period does not reach the floor of the bounds from every date of
    /// access.
    #[error(
        "{key} keeps a record {period}, which does not reach the floor of {floor} ({ground}) from every date of access"
    )]
    UnderBound {
        /// The key that declares it.
        key: String,
        /// The period declared.
        period: Period,
        /// The floor.
        floor: Period,
        /// The floor's legal ground, as a record writes it.
        ground: String,
    },
    /// A period passes the ceiling of the bounds.
    #[error("{key} keeps a record {period}, past the ceiling of {ceiling} ({ground})")]
    OverCeiling {
        /// The key that declares it.
        key: String,
        /// The period declared.
        period: Period,
        /// The ceiling.
        ceiling: Period,
        /// The ceiling's legal ground.
        ground: String,
    },
    /// A national floor does not reach the three years of Art 9(2).
    #[error(
        "a national floor of {floor} does not reach the {FLOOR_YEARS} years of Regulation (EU) 2025/327 Art 9(2); a deployment outside the Union declares its floor as such"
    )]
    NationalUnderUnion {
        /// The floor declared.
        floor: Period,
    },
    /// A floor is without end.
    #[error("a retention floor is a finite period")]
    FloorIndefinite,
    /// A ceiling does not reach the floor.
    #[error("the ceiling of {ceiling} does not reach the floor of {floor}")]
    CeilingUnderFloor {
        /// The ceiling declared.
        ceiling: Period,
        /// The floor.
        floor: Period,
    },
    /// A category key is neither a priority category nor a national one
    /// the category map declares.
    #[error("categories key {code:?} is neither a priority category nor a declared national one")]
    UnknownCategory {
        /// The code.
        code: String,
    },
    /// An origin key is empty or holds a control character.
    #[error("an origins key is empty or holds a control character")]
    OriginKey,
}

/// The periods a deployment keeps its records for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionPolicy {
    bounds: Bounds,
    default: Period,
    categories: BTreeMap<Category, Period>,
    origins: BTreeMap<String, Period>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            bounds: Bounds::union(),
            default: Period::Years(FLOOR_YEARS),
            categories: BTreeMap::new(),
            origins: BTreeMap::new(),
        }
    }
}

impl RetentionPolicy {
    /// The policy, inside the Union, that keeps every record `years`, the
    /// data of each category of `categories` and from each origin of
    /// `origins` as long as they say, in whole years, each category read as
    /// [`CategoryMap::category`] reads it.
    ///
    /// # Errors
    ///
    /// A [`RetentionError`] for a period under [`FLOOR_YEARS`], a code that
    /// names no category, and an empty or malformed origin key.
    pub fn declare(
        years: u16,
        categories: &BTreeMap<String, u16>,
        origins: &BTreeMap<String, u16>,
        map: &CategoryMap,
    ) -> Result<Self, RetentionError> {
        let in_years = |table: &BTreeMap<String, u16>| {
            table
                .iter()
                .map(|(key, years)| (key.clone(), Period::Years(*years)))
                .collect::<BTreeMap<_, _>>()
        };
        Self::declare_within(
            &Bounds::union(),
            Period::Years(years),
            &in_years(categories),
            &in_years(origins),
            map,
        )
    }

    /// The policy within `bounds` that keeps every record `default`, the
    /// data of each category of `categories` and from each origin of
    /// `origins` (by its key: its endpoint, or the `system_id` of an origin
    /// that names no endpoint) as long as they say.
    ///
    /// # Errors
    ///
    /// A [`RetentionError`] for a period outside the bounds, a code that
    /// names no category, and an empty or malformed origin key.
    pub fn declare_within(
        bounds: &Bounds,
        default: Period,
        categories: &BTreeMap<String, Period>,
        origins: &BTreeMap<String, Period>,
        map: &CategoryMap,
    ) -> Result<Self, RetentionError> {
        let default = bounds.admit("years".to_owned(), default)?;
        let categories = categories
            .iter()
            .map(|(code, period)| {
                let category = map
                    .category(code)
                    .ok_or_else(|| RetentionError::UnknownCategory { code: code.clone() })?;
                Ok((
                    category,
                    bounds.admit(format!("categories.{code}"), *period)?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, RetentionError>>()?;
        let origins = origins
            .iter()
            .map(|(key, period)| {
                if key.is_empty() || key.chars().any(char::is_control) {
                    return Err(RetentionError::OriginKey);
                }
                Ok((
                    key.clone(),
                    bounds.admit(format!("origins.{key}"), *period)?,
                ))
            })
            .collect::<Result<BTreeMap<_, _>, RetentionError>>()?;
        Ok(Self {
            bounds: bounds.clone(),
            default,
            categories,
            origins,
        })
    }

    /// The bounds the policy was declared within.
    #[must_use]
    pub fn bounds(&self) -> &Bounds {
        &self.bounds
    }

    /// The origin keys the policy names.
    pub fn origins(&self) -> impl Iterator<Item = &str> {
        self.origins.keys().map(String::as_str)
    }

    /// The longest period the policy declares anywhere, by the fewest days
    /// each spans; [`RetentionPolicy::retain`] compares by the end date each
    /// gives a record instead.
    #[must_use]
    pub fn longest(&self) -> Period {
        self.categories
            .values()
            .chain(self.origins.values())
            .copied()
            .fold(self.default, |kept, candidate| {
                let longer = match (kept.fewest_days(), candidate.fewest_days()) {
                    (None, _) => false,
                    (Some(_), None) => true,
                    (Some(ours), Some(theirs)) => theirs > ours,
                };
                if longer { candidate } else { kept }
            })
    }

    /// How long the record of an access made at `recorded`, classified as
    /// `classified`, from the origins keyed `origins`, is kept.
    ///
    /// The period is the longest of the default, each category's and each
    /// origin's, and the longest declared anywhere for an access that is
    /// unclassified in whole or in part, each compared by the end date it
    /// gives this record. The ground is the first of these, in that order,
    /// that calls for the period.
    #[must_use]
    pub fn retain<'a>(
        &self,
        recorded: Timestamp,
        classified: &Classification,
        origins: impl IntoIterator<Item = &'a str>,
    ) -> Retention {
        let accessed = Offset::UTC.to_datetime(recorded).date();
        let longest = |candidates: Vec<(Period, Ground)>| {
            candidates
                .into_iter()
                .fold((self.default, Ground::Default), |kept, candidate| {
                    if kept.0.outlasted_by(candidate.0, accessed) {
                        candidate
                    } else {
                        kept
                    }
                })
        };
        let (period, ground) = if classified.unclassified().is_some() {
            let (period, _) = longest(
                self.categories
                    .values()
                    .chain(self.origins.values())
                    .map(|period| (*period, Ground::Unclassified))
                    .collect(),
            );
            (period, Ground::Unclassified)
        } else {
            let categories = classified.categories().keys().filter_map(|category| {
                self.categories
                    .get(category)
                    .map(|period| (*period, Ground::Category(category.clone())))
            });
            let origins = origins.into_iter().filter_map(|key| {
                self.origins
                    .get(key)
                    .map(|period| (*period, Ground::Origin(key.to_owned())))
            });
            longest(categories.chain(origins).collect())
        };
        Retention {
            period,
            ground,
            floor: self.bounds.floor_ground.clone(),
            ends: period.ends(accessed),
        }
    }
}
