// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The harness's store: a fixed corpus ITI-81 searches, so a search costs
//! the same in every phase, and a bounded ring of the records it keeps, so
//! memory stays flat however long the load runs.

use std::collections::VecDeque;
use std::fmt;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use fhir_types::r4::audit_event::AuditEvent;
use ihe_atna::arr::search::matches;
use ihe_atna::arr::{AuditStore, Found, Requester, StoreError};
use ihe_atna::retrieve::Query;
use ihe_atna::retrieve::bundle::Match;
use jiff::tz::TimeZone;

/// The records of the corpus.
const RECORDS: u32 = 10_000;

/// The patients the corpus spreads its records over; the plan draws its
/// searched patient from `0` to one below this.
const PATIENTS: u32 = 1_000;

/// The most received records the ring holds.
const RING: usize = 4_096;

/// The harness's store.
pub(crate) struct Store {
    corpus: Vec<AuditEvent>,
    ring: Mutex<VecDeque<AuditEvent>>,
    kept: AtomicU64,
}

impl fmt::Debug for Store {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Store")
            .field("corpus", &self.corpus.len())
            .field("kept", &self.kept.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

/// The ring's lock was poisoned by a panic while it was held.
#[derive(Debug, thiserror::Error)]
#[error("the store's ring was poisoned")]
struct Poisoned;

impl Store {
    /// The store holding the corpus: record `n` is a read of patient
    /// `Qz7-p-<n mod PATIENTS>` on day `n mod 365` of 2026. Every value is
    /// synthetic.
    ///
    /// # Errors
    ///
    /// The error of a day past jiff's range or of a record that is no FHIR
    /// R4 `AuditEvent`.
    pub(crate) fn seeded() -> Result<Self, Box<dyn std::error::Error>> {
        let start = jiff::civil::date(2026, 1, 1);
        let mut corpus = Vec::new();
        for n in 0..RECORDS {
            let day = start.checked_add(jiff::Span::new().days(i64::from(n % 365)))?;
            corpus.push(serde_json::from_str(&record(
                n % PATIENTS,
                &day.to_string(),
            ))?);
        }
        Ok(Self {
            corpus,
            ring: Mutex::new(VecDeque::with_capacity(RING)),
            kept: AtomicU64::new(0),
        })
    }
}

/// The JSON of a BALP patient read of `Qz7-p-<patient>` on `day`.
fn record(patient: u32, day: &str) -> String {
    format!(
        r#"{{"resourceType":"AuditEvent","type":{{"system":"http://terminology.hl7.org/CodeSystem/audit-event-type","code":"rest"}},"subtype":[{{"system":"http://hl7.org/fhir/restful-interaction","code":"read"}}],"action":"R","recorded":"{day}T10:00:00Z","outcome":"0","agent":[{{"type":{{"coding":[{{"system":"http://dicom.nema.org/resources/ontology/DCM","code":"110153"}}]}},"who":{{"identifier":{{"value":"Qz7-client"}}}},"requestor":false,"network":{{"address":"192.0.2.10","type":"2"}}}},{{"type":{{"coding":[{{"system":"http://dicom.nema.org/resources/ontology/DCM","code":"110152"}}]}},"who":{{"identifier":{{"value":"perf.example.org"}}}},"requestor":false}},{{"who":{{"identifier":{{"system":"urn:oid:2.999.1","value":"Qz7-user"}}}},"requestor":true}}],"source":{{"observer":{{"identifier":{{"value":"perf.example.org"}}}}}},"entity":[{{"what":{{"reference":"Patient/Qz7-p-{patient}","identifier":{{"system":"urn:oid:2.999.2","value":"Qz7-p-{patient}"}}}},"type":{{"system":"http://terminology.hl7.org/CodeSystem/audit-entity-type","code":"1"}},"role":{{"system":"http://terminology.hl7.org/CodeSystem/object-role","code":"1"}}}}]}}"#
    )
}

#[async_trait::async_trait]
impl AuditStore for Store {
    async fn keep(&self, record: AuditEvent) -> Result<Option<String>, StoreError> {
        let mut ring = self
            .ring
            .lock()
            .map_err(|_poisoned| StoreError::Unavailable(Box::new(Poisoned)))?;
        if ring.len() == RING {
            ring.pop_front();
        }
        ring.push_back(record);
        drop(ring);
        let id = self.kept.fetch_add(1, Ordering::Relaxed).saturating_add(1);
        Ok(Some(id.to_string()))
    }

    async fn search(&self, query: &Query, _requester: &Requester) -> Result<Found, StoreError> {
        let zone = TimeZone::UTC;
        let found: Vec<(usize, &AuditEvent)> = self
            .corpus
            .iter()
            .enumerate()
            .filter(|(_, event)| matches(query, event, &zone))
            .collect();
        let offset = usize::try_from(query.offset().unwrap_or(0)).unwrap_or(usize::MAX);
        let count = query.count().map_or(usize::MAX, |count| {
            usize::try_from(count).unwrap_or(usize::MAX)
        });
        Ok(Found {
            total: u32::try_from(found.len()).unwrap_or(u32::MAX),
            matches: found
                .into_iter()
                .skip(offset)
                .take(count)
                .map(|(index, event)| Match {
                    full_url: Some(format!("http://perf.example.org/fhir/AuditEvent/c{index}")),
                    resource: event.clone(),
                })
                .collect(),
        })
    }
}
