// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The waits a test of a timeout or a deadline makes, sized so that how busy
//! the host is cannot change which side of a deadline an event falls on.
//!
//! The forwarder talks to its repository over real sockets, so
//! the tokio clock cannot be paused under them: a paused clock advances to
//! the next timer whenever the runtime is idle, even while bytes are still
//! in flight. A deadline is taken after the forwarder and its repository are
//! set up, and sits the waits their own timeouts allow plus [`SLACK`] past
//! the moment it is taken. No specification governs this: our own design.

use std::time::Duration;
use std::time::Instant;

/// The time a loaded host may add to any step a test waits on.
pub(crate) const SLACK: Duration = Duration::from_secs(3);

/// The instant `waits` plus [`SLACK`] from now.
pub(crate) fn deadline(waits: Duration) -> Instant {
    Instant::now()
        .checked_add(waits.saturating_add(SLACK))
        .expect("the deadline should be within the platform clock")
}
