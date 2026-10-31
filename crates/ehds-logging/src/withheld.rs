// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The records a person does not see yet: access records about a resource
//! held from the person under Regulation (EU) 2025/327 Art 3(3).
//!
//! Art 3(3) lets a Member State delay a person's access to their data "for a
//! limited period of time until a health professional is able to properly
//! communicate and explain" it, and Art 9(1) and (2) give the person the
//! record of every access to their data: who, when and "which personal
//! electronic health data were accessed". While a result is held, its access
//! records in the person's own trail would tell them it exists. So the
//! product that holds a resource names the hold here, and the person-facing
//! read leaves out the records about that resource until the hold is released
//! or expires; then they appear, unchanged, with their original times.
//! Professional and supervisory reads see every record at all times.
//!
//! A [`Holds`] register is keyed by the resource as the records name it
//! ([`Request::resource`](crate::record::Request::resource), a written
//! version, a version the access reached): the product sets a hold with
//! [`Holds::hold`] and lifts it with [`Holds::release`], and the read asks
//! [`Holds::shows`] or filters with [`Holds::trail`]. No record is altered,
//! so the trail keeps its integrity and its Art 9(2) retention.
//!
//! # Examples
//!
//! ```
//! use ehds_logging::withheld::{Holds, Reader};
//! use jiff::{SignedDuration, Timestamp};
//!
//! let registered: Timestamp = "2026-10-31T09:00:00Z".parse()?;
//! let mut holds = Holds::default();
//! holds.hold("8fa1c0de", registered + SignedDuration::from_hours(72));
//!
//! // A record about version 1 of the held object.
//! assert!(holds.withholds("8fa1c0de::cdr.example.org::1", registered));
//! assert_eq!(Reader::Person.is_person(), true);
//!
//! // The professional explains the result and releases it.
//! holds.release("8fa1c0de", registered + SignedDuration::from_hours(2))?;
//! assert!(!holds.withholds("8fa1c0de", registered + SignedDuration::from_hours(2)));
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

use std::collections::BTreeMap;

use jiff::Timestamp;

use crate::record::AccessRecord;

/// One hold on a resource: until when it is withheld from the person, and
/// when a professional released it, if they did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hold {
    until: Timestamp,
    released: Option<Timestamp>,
}

impl Hold {
    /// A hold that withholds the resource from the person until `until`.
    ///
    /// Every hold ends by itself, since Art 3(3) admits a delay "for a
    /// limited period of time" only.
    #[must_use]
    pub const fn new(until: Timestamp) -> Self {
        Self {
            until,
            released: None,
        }
    }

    /// When the hold expires unless released earlier.
    #[must_use]
    pub const fn expires(&self) -> Timestamp {
        self.until
    }

    /// When a professional released the hold, if they did.
    #[must_use]
    pub const fn released(&self) -> Option<Timestamp> {
        self.released
    }

    /// When the person sees the resource's records: the release, or the
    /// expiry, whichever comes first.
    #[must_use]
    pub fn ends(&self) -> Timestamp {
        self.released
            .map_or(self.until, |released| released.min(self.until))
    }

    /// Whether the hold withholds the resource from the person at `now`.
    #[must_use]
    pub fn withholds(&self, now: Timestamp) -> bool {
        now < self.ends()
    }

    /// Releases the hold at `at`; a hold released twice keeps its earlier
    /// release.
    pub fn release(&mut self, at: Timestamp) {
        self.released = Some(self.released.map_or(at, |released| released.min(at)));
    }
}

/// Who reads the access trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reader {
    /// The data subject, or a representative acting for them (Art 4(2)),
    /// reading their own trail under Art 9.
    Person,
    /// A health professional or the controller's staff.
    Professional,
    /// A supervisory authority, or the controller reviewing its log (Annex
    /// II 3.3).
    Supervisory,
}

impl Reader {
    /// Whether the reader is the person a hold withholds from.
    #[must_use]
    pub const fn is_person(self) -> bool {
        matches!(self, Self::Person)
    }
}

/// Why a hold could not be lifted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum HoldError {
    /// The register holds no hold on the resource.
    #[error("the resource is not held from the person")]
    NotHeld,
}

/// The holds a product keeps on resources, keyed by the resource as its
/// access records name it.
///
/// A product passes, for one person's read, a register that holds that
/// person's holds; the register is a value it builds from its own store.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Holds {
    by_resource: BTreeMap<String, Hold>,
}

impl Holds {
    /// Holds `resource` from the person until `until`.
    ///
    /// A hold in force on the resource is extended to the later of the two
    /// ends; a hold already released is replaced, so a professional can hold
    /// an amended resource again.
    pub fn hold(&mut self, resource: impl Into<String>, until: Timestamp) {
        let fresh = Hold::new(until);
        self.by_resource
            .entry(resource.into())
            .and_modify(|hold| {
                *hold = match hold.released {
                    None => Hold::new(hold.until.max(until)),
                    Some(_) => fresh,
                };
            })
            .or_insert(fresh);
    }

    /// Releases the hold on `resource` at `at`: from then on the person sees
    /// its records, with their original times.
    ///
    /// # Errors
    ///
    /// [`HoldError::NotHeld`] when the register holds no hold on `resource`.
    pub fn release(&mut self, resource: &str, at: Timestamp) -> Result<(), HoldError> {
        let hold = self
            .by_resource
            .get_mut(resource)
            .ok_or(HoldError::NotHeld)?;
        hold.release(at);
        Ok(())
    }

    /// The hold on `resource`, if the register holds one.
    #[must_use]
    pub fn get(&self, resource: &str) -> Option<&Hold> {
        self.by_resource.get(resource)
    }

    /// Whether `resource` is withheld from the person at `now`.
    ///
    /// An openEHR `OBJECT_VERSION_ID` is withheld when its `object_id` is, so
    /// a hold on a versioned object covers each of its versions.
    #[must_use]
    pub fn withholds(&self, resource: &str, now: Timestamp) -> bool {
        // NOTE: openEHR BASE 1.3.0 identification package, an OBJECT_VERSION_ID is
        // `object_id::creating_system_id::version_tree_id`, every version sharing the object_id.
        let object = resource.split_once("::").map(|(object, _)| object);
        [Some(resource), object]
            .into_iter()
            .flatten()
            .filter_map(|id| self.by_resource.get(id))
            .any(|hold| hold.withholds(now))
    }

    /// Whether `reader` sees `record` in the access trail at `now`.
    ///
    /// A professional or supervisory reader sees every record. The person
    /// does not see a record that names a resource withheld at `now`, by its
    /// request, a version a write left or a version the access reached. A
    /// record whose versions the map's evidence limit cut cannot be cleared,
    /// so the person does not see it while any hold of the register is in
    /// force.
    #[must_use]
    pub fn shows(&self, reader: Reader, record: &AccessRecord, now: Timestamp) -> bool {
        // NOTE: Art 9(1) gives the person the record of every access, and Art 3(3) delays
        // their access to held data; the delay reaches only the person's own read.
        if !reader.is_person() || self.by_resource.is_empty() {
            return true;
        }
        let cut = record.categories.evidence_counts().versions > record.categories.versions().len()
            || record.origins.iter().any(|origin| {
                origin.categories.as_ref().is_some_and(|categories| {
                    categories.evidence_counts().versions > categories.versions().len()
                })
            });
        if cut && self.by_resource.values().any(|hold| hold.withholds(now)) {
            return false;
        }
        !named(record).any(|resource| self.withholds(resource, now))
    }

    /// The records of `records` that `reader` sees at `now`, in their order
    /// and unchanged.
    pub fn trail<'a>(
        &'a self,
        reader: Reader,
        records: impl IntoIterator<Item = &'a AccessRecord> + 'a,
        now: Timestamp,
    ) -> impl Iterator<Item = &'a AccessRecord> + 'a {
        records
            .into_iter()
            .filter(move |record| self.shows(reader, record, now))
    }
}

/// Every resource `record` names: the one its request addressed, each
/// version a write left, and each version the access reached.
fn named(record: &AccessRecord) -> impl Iterator<Item = &str> {
    let origins = record.origins.iter().flat_map(|origin| {
        let versions = origin
            .categories
            .iter()
            .flat_map(|categories| categories.versions().iter().map(String::as_str));
        origin
            .version
            .as_ref()
            .and_then(|version| version.named())
            .into_iter()
            .chain(versions)
    });
    record
        .request
        .resource
        .as_deref()
        .into_iter()
        .chain(record.categories.versions().iter().map(String::as_str))
        .chain(origins)
}
