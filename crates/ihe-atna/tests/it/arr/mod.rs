// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The Audit Record Repository (feature `arr`): ITI-20 received over syslog
//! and the FHIR Feed from this crate's own forwarders, and ITI-81 answered
//! over the store with every search parameter of `RESTful` ATNA Rev. 3.6
//! §3.81.4.1.2. The store is the test's own, in memory, as a product's
//! would be its database.

#[cfg(test)]
mod feed;
#[cfg(test)]
mod search;
#[cfg(test)]
mod syslog;

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use fhir_types::r4::audit_event::AuditEvent;
use ihe_atna::arr::search::matches;
use ihe_atna::arr::{AuditStore, Found, Requester, StoreError};
use ihe_atna::retrieve::Query;
use ihe_atna::retrieve::bundle::Match;

/// A store in memory, which a test can make unavailable or refusing.
#[derive(Debug, Default)]
pub(crate) struct Memory {
    pub(crate) kept: Mutex<Vec<AuditEvent>>,
    pub(crate) unavailable: AtomicBool,
    pub(crate) refusing: AtomicBool,
}

/// Why the test store did not keep a record.
#[derive(Debug, thiserror::Error)]
#[error("the test store said no")]
struct Said;

impl Memory {
    pub(crate) fn kept(&self) -> Vec<AuditEvent> {
        self.kept.lock().expect("the store").clone()
    }
}

#[async_trait::async_trait]
impl AuditStore for Memory {
    async fn keep(&self, record: AuditEvent) -> Result<Option<String>, StoreError> {
        if self.unavailable.load(Ordering::SeqCst) {
            return Err(StoreError::Unavailable(Box::new(Said)));
        }
        if self.refusing.load(Ordering::SeqCst) {
            return Err(StoreError::Refused(Box::new(Said)));
        }
        let mut kept = self.kept.lock().expect("the store");
        kept.push(record);
        Ok(Some(kept.len().to_string()))
    }

    async fn search(&self, query: &Query, _requester: &Requester) -> Result<Found, StoreError> {
        if self.unavailable.load(Ordering::SeqCst) {
            return Err(StoreError::Unavailable(Box::new(Said)));
        }
        let kept = self.kept.lock().expect("the store");
        let found: Vec<(usize, &AuditEvent)> = kept
            .iter()
            .enumerate()
            .filter(|(_, event)| matches(query, event, &jiff::tz::TimeZone::UTC))
            .collect();
        let offset = usize::try_from(query.offset().unwrap_or(0)).expect("an offset");
        let count = query
            .count()
            .map_or(usize::MAX, |count| usize::try_from(count).expect("a count"));
        Ok(Found {
            total: u32::try_from(found.len()).expect("a total"),
            matches: found
                .into_iter()
                .skip(offset)
                .take(count)
                .map(|(index, event)| Match {
                    full_url: Some(format!(
                        "https://arr.example.org/fhir/AuditEvent/{}",
                        index + 1
                    )),
                    resource: event.clone(),
                })
                .collect(),
        })
    }
}
