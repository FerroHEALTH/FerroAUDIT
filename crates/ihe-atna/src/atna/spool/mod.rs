// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The local store of audit messages awaiting delivery: ITI TF-2
//! §3.20.4.1.1 has a sender that cannot reach its Audit Record Repository
//! store the record locally and send it when it is able.
//!
//! A [`Spool`] is bounded by a number of messages and a number of bytes,
//! the messages queued for a write counted with those stored, so a stalled
//! disk holds no more in memory than the bounds admit; a message past either
//! bound is refused at once, never dropped silently, so its caller can fail
//! the exchange it audits. Storing one message is bounded in
//! time too: one not stored within [`Bounds::write_timeout`], the wait for a
//! write before it included, is refused with [`SpoolError::Late`] and is not
//! counted. That write goes on, or still waits its turn, and once it ends
//! the message is stored and delivered like any other, or its failure is
//! logged, so a slow disk neither loses nor repeats a message. A caller
//! that refuses its exchange when its record is late uses
//! [`reserve::Reservation::push_or_withdraw`] instead: a message still
//! queued at the bound is withdrawn and the record of the refusal queued in
//! its place. Messages leave in the order they were stored.
//!
//! On disk ([`Spool::open`]), one writer thread owns the active segment, a
//! file named by the sequence number of its first message, each message
//! framed by its length and its CRC-32. The messages that queue while one
//! write is in flight are appended together as one batch and flushed to the
//! device once, so a batch costs one flush however many messages it holds,
//! and each message counts as stored once its batch is durable and survives
//! a crash or a restart. A segment that reaches 4,096 messages or 8 MiB is
//! sealed with an index of its frames, so an open reads indexes instead of
//! every message. A sidecar beside a segment counts its messages delivered
//! or quarantined from the first, written without a flush, so a crash sends
//! again what it had not recorded: each message is delivered at least once.
//! A sealed segment whose messages are all settled is removed. An active
//! segment a crash left is sealed by the next open, an append it cut short,
//! never acknowledged, cut off. A segment that ends in a complete frame
//! whose CRC fails keeps the frames before it, and the bytes from that frame
//! on go to the quarantine. On Unix the directory must give its owner alone access, and
//! every file is created readable by its owner alone, because a message
//! names a patient. In memory ([`Spool::in_memory`]), the messages live as
//! long as the process, for a development deployment.
//!
//! A spool holds one [`Content`]: RFC 5425 syslog frames, or, with the
//! `balp` feature, FHIR `AuditEvent` records for the FHIR Feed. A stored
//! message that cannot be read, or that is no message of the spool's content,
//! is moved to the `quarantine` subdirectory when the drain reaches it,
//! logged with its sequence number and never its content, and the drain goes
//! on with the next one, so one bad file never holds back the rest. So is a
//! message the repository refused ([`Spool::reject`]). A quarantined message
//! stays counted under the bounds until an operator removes it; in memory,
//! it stays held until the process ends. A file the spool did not write refuses [`Spool::open`]
//! instead, naming the file to move: in a directory only the sender's user
//! may enter, it was put there by hand.
//!
//! The bounds and the file layout are our own design: no specification
//! governs them.

mod disk;
mod drain;
pub(super) mod in_flight;
mod inspect;
pub mod reserve;
pub mod withdraw;
mod writer;

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use secrecy::{ExposeSecret, SecretSlice};
use tokio::sync::{Notify, oneshot};
use tokio::task::JoinError;

use disk::{
    Frame, check_private, create_private, probe, remove_segment, scan_quarantine, scan_segments,
    seal, sync_directory, torn, truncate_segment, write_index, write_quarantined,
};
use writer::{Shared, Writer};

/// The extension of a message held in the quarantine.
const STORED: &str = "msg";

/// The extension of a sealed segment.
const SEGMENT: &str = "seg";

/// The extension of the active segment, which batches are appended to.
const OPEN: &str = "open";

/// The extension of a sealed segment's index of its frames.
const INDEX: &str = "idx";

/// The extension of a segment's sidecar, the count of its messages settled.
const SENT: &str = "sent";

/// The extension of a message being written.
const PARTIAL: &str = "partial";

/// The subdirectory a message that cannot be delivered is moved to.
pub const QUARANTINE: &str = "quarantine";

/// What a spool on disk holds, which a stored file is read back as.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum Content {
    /// RFC 5425 syslog frames, each a DICOM audit message (ITI TF-2
    /// §3.20.4.1).
    #[default]
    SyslogFrames,
    /// FHIR `AuditEvent` resources as JSON, for the FHIR Feed (ITI TF-2
    /// §3.20.4.2).
    #[cfg(feature = "balp")]
    AuditEvents,
}

impl Content {
    /// Whether `bytes` are one message of this content.
    fn holds(self, bytes: &[u8]) -> bool {
        match self {
            Self::SyslogFrames => super::syslog::is_frame(bytes),
            #[cfg(feature = "balp")]
            Self::AuditEvents => {
                serde_json::from_slice::<fhir_types::r4::audit_event::AuditEvent>(bytes).is_ok()
            }
        }
    }

    /// What a stored file of another content is, as the quarantine logs it.
    fn mismatch(self) -> &'static str {
        match self {
            Self::SyslogFrames => "it is no whole RFC 5425 frame",
            #[cfg(feature = "balp")]
            Self::AuditEvents => "it is no FHIR AuditEvent",
        }
    }
}

/// The bounds of a spool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// The most messages held at once.
    pub max_messages: usize,
    /// The most bytes held at once, over every message.
    pub max_bytes: u64,
    /// The longest storing one message may take, the wait for a write
    /// before it included.
    pub write_timeout: Duration,
}

/// Why a spool could not be opened or could not store a message.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SpoolError {
    /// The spool already holds its most messages or bytes, the messages
    /// queued for a write counted with those stored.
    #[error("the audit spool is full: {messages} messages, {bytes} bytes")]
    Full {
        /// The messages it holds or has queued.
        messages: usize,
        /// The bytes it holds or has queued.
        bytes: u64,
    },
    /// The directory gives a group or other users access to it.
    #[error("the audit spool directory {0} is open to its group or to other users")]
    Exposed(PathBuf),
    /// The directory holds a file the spool did not write.
    #[error(
        "the audit spool directory {} holds {}, which the sender did not write: move that file out of the spool directory, keeping it if it may be an audit record, and start again",
        directory.display(),
        path.display()
    )]
    Foreign {
        /// The spool directory, or its quarantine.
        directory: PathBuf,
        /// The file to move.
        path: PathBuf,
    },
    /// The path of the spool directory, of its quarantine, or of the nearest
    /// existing directory above a missing one, is a file.
    #[error("{} is no directory, so no audit spool can be opened there", .0.display())]
    NotADirectory(PathBuf),
    /// The mode of the directory admits no writer.
    #[error(
        "the audit spool directory {} gives no user write access; grant the sender's user write access to it",
        .0.display()
    )]
    Unwritable(PathBuf),
    /// A file system operation failed.
    #[error("the audit spool at {path} could not be {action}")]
    Io {
        /// What was being done: `read`, `written`, `created`, `removed`.
        action: &'static str,
        /// The path it was done to.
        path: PathBuf,
        /// The cause.
        #[source]
        source: std::io::Error,
    },
    /// The message was not stored within [`Bounds::write_timeout`]. Its write
    /// goes on, and a message it stores is delivered like any other.
    #[error("the audit spool did not store the message within {bound:?}")]
    Late {
        /// The bound it missed.
        bound: Duration,
    },
    /// The message was not stored within [`Bounds::write_timeout`] and was
    /// still queued, so it was withdrawn and the record of its refusal
    /// queued in its place ([`reserve::Reservation::push_or_withdraw`]).
    #[error(
        "the audit spool did not store the message within {bound:?}; it was withdrawn and the record of its refusal queued in its place"
    )]
    Withdrawn {
        /// The bound it missed.
        bound: Duration,
    },
    /// The task that touched the disk did not finish.
    #[error("the audit spool task did not finish")]
    Task(#[source] JoinError),
    /// The write of the batch that held the message did not finish, so the
    /// message is not counted as stored.
    #[error("the audit spool's write of the batch holding the message did not finish")]
    Unfinished,
}

/// What a spool holds right now.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Depth {
    /// The messages held, quarantined ones included.
    pub messages: usize,
    /// The bytes held, quarantined ones included.
    pub bytes: u64,
    /// The messages held in quarantine.
    pub quarantined: usize,
}

impl Depth {
    /// The messages awaiting delivery: those held, less the quarantined.
    #[must_use]
    pub fn waiting(&self) -> usize {
        self.messages.saturating_sub(self.quarantined)
    }
}

/// One stored message, with the sequence number it is removed by.
pub struct Stored {
    /// The sequence number.
    pub sequence: u64,
    /// The message.
    pub message: SecretSlice<u8>,
}

impl std::fmt::Debug for Stored {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stored")
            .field("sequence", &self.sequence)
            .finish_non_exhaustive()
    }
}

/// A bounded store of messages awaiting delivery.
///
/// Reading its state never waits on the disk, so a write that stalls holds
/// up no reader of [`Spool::depth`].
#[derive(Clone)]
pub struct Spool {
    inner: Arc<Mutex<Inner>>,
    shown: Arc<Mutex<Ledger>>,
    waiting: Arc<Mutex<VecDeque<Waiting>>>,
    tickets: Arc<AtomicU64>,
    wake: Arc<Condvar>,
    closed: Arc<AtomicBool>,
    retire: Arc<AtomicBool>,
    #[cfg(test)]
    flush: Arc<Mutex<Option<writer::Flush>>>,
    #[cfg(test)]
    gate: Arc<Mutex<()>>,
    writer: Option<Arc<Writer>>,
    arrived: Arc<Notify>,
    bounds: Bounds,
    durable: bool,
}

impl std::fmt::Debug for Spool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Spool")
            .field("on_disk", &self.durable)
            .field("depth", &self.depth())
            .field("bounds", &self.bounds)
            .finish_non_exhaustive()
    }
}

struct Inner {
    store: Store,
    bounds: Bounds,
    depth: Depth,
    shown: Arc<Mutex<Ledger>>,
    next: u64,
    /// The first message of the segment the writer appends to.
    active: Option<u64>,
    /// Set when every message of the active segment is settled, so the
    /// writer removes it.
    retire: Arc<AtomicBool>,
    /// The directory flushes the writes have made.
    #[cfg(test)]
    syncs: u64,
    /// The segment flushes the writes have made.
    #[cfg(test)]
    segment_syncs: u64,
}

/// A message waiting for the batch that writes it, with its place under the
/// bounds and the caller waiting for its sequence number.
struct Waiting {
    /// What a caller finds it by while it is queued.
    ticket: u64,
    message: SecretSlice<u8>,
    place: Place,
    done: oneshot::Sender<Result<u64, SpoolError>>,
}

/// What readers of the spool see without taking its lock: the depth it
/// holds, the messages queued for a write, and the refusals.
#[derive(Debug, Default)]
struct Ledger {
    depth: Depth,
    queued: usize,
    queued_bytes: u64,
    refused: u64,
}

impl Ledger {
    /// Counts a message refused for want of room.
    fn refuse(&mut self) -> u64 {
        self.refused = self.refused.saturating_add(1);
        self.refused
    }
}

/// The place a message holds under the bounds while it waits for its
/// write, given back once the write has ended and the depth shows it.
struct Place {
    ledger: Arc<Mutex<Ledger>>,
    size: u64,
    held: bool,
}

impl Place {
    /// Gives the place back in `ledger`, which the caller holds locked.
    fn give_back(mut self, ledger: &mut Ledger) {
        ledger.queued = ledger.queued.saturating_sub(1);
        ledger.queued_bytes = ledger.queued_bytes.saturating_sub(self.size);
        self.held = false;
    }
}

impl Drop for Place {
    fn drop(&mut self) {
        if self.held {
            let mut ledger = lock(&self.ledger);
            ledger.queued = ledger.queued.saturating_sub(1);
            ledger.queued_bytes = ledger.queued_bytes.saturating_sub(self.size);
        }
    }
}

/// One segment on disk: where its messages lie, and which are settled.
struct Segment {
    /// Its frames, in order; message `first + i` is frame `i`.
    frames: Vec<Frame>,
    /// How many frames, from the first, are delivered or quarantined, as the
    /// sidecar records.
    prefix: u64,
    /// The frames past the prefix that are settled.
    settled: BTreeSet<u64>,
    /// Whether it is sealed: renamed `.seg`, taking no more messages.
    sealed: bool,
}

impl Segment {
    /// The number of its frames.
    fn len(&self) -> u64 {
        u64::try_from(self.frames.len()).unwrap_or(u64::MAX)
    }
}

/// Locks `mutex`, taking over the state a holder that panicked left.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

enum Store {
    Disk {
        directory: PathBuf,
        content: Content,
        sizes: BTreeMap<u64, u64>,
        quarantined: BTreeMap<u64, u64>,
        segments: BTreeMap<u64, Segment>,
    },
    Memory {
        queue: VecDeque<(u64, SecretSlice<u8>)>,
        held: Vec<(u64, SecretSlice<u8>)>,
    },
}

impl Spool {
    /// The spool of syslog frames in `directory`, created when missing,
    /// holding the messages a previous process left in it.
    ///
    /// # Errors
    ///
    /// The errors of [`Spool::open_for`].
    pub fn open(directory: &Path, bounds: Bounds) -> Result<Self, SpoolError> {
        Self::open_for(directory, bounds, Content::SyslogFrames)
    }

    /// The spool of `content` in `directory`, created when missing, holding
    /// the messages a previous process left in it.
    ///
    /// # Errors
    ///
    /// [`SpoolError::Exposed`] when the directory or its quarantine gives a
    /// group or other users access to it (Unix), [`SpoolError::Foreign`]
    /// when either holds a file the spool did not write, and
    /// [`SpoolError::Io`] when they cannot be created, read or written.
    pub fn open_for(
        directory: &Path,
        bounds: Bounds,
        content: Content,
    ) -> Result<Self, SpoolError> {
        let held = directory.join(QUARANTINE);
        create_private(directory)?;
        check_private(directory)?;
        create_private(&held)?;
        check_private(&held)?;
        probe(directory)?;
        let mut quarantined = scan_quarantine(&held)?;
        let mut sizes = BTreeMap::new();
        let mut segments = BTreeMap::new();
        let mut sealed_any = false;
        for (first, loaded) in scan_segments(directory, &held)? {
            let mut segment = Segment {
                frames: loaded.frames,
                prefix: loaded.settled,
                settled: BTreeSet::new(),
                sealed: loaded.sealed,
            };
            if let Some(remainder) = loaded.remainder {
                let sequence = first.saturating_add(segment.len());
                if !segment.sealed && torn(&remainder) {
                    tracing::warn!(target: crate::atna::targets::SPOOL,
                        sequence,
                        "the active spool segment ends in an append a crash cut short, never acknowledged; it was cut off"
                    );
                } else {
                    if let std::collections::btree_map::Entry::Vacant(kept) =
                        quarantined.entry(sequence)
                    {
                        write_quarantined(&held, sequence, &remainder)?;
                        kept.insert(u64::try_from(remainder.len()).unwrap_or(u64::MAX));
                    }
                    tracing::error!(target: crate::atna::targets::SPOOL,
                        sequence,
                        "a spool segment ends in a frame that cannot be read; the bytes from it on were moved to the quarantine"
                    );
                }
                truncate_segment(directory, first, segment.sealed, &segment.frames)?;
            }
            if !segment.sealed {
                if !segment.frames.is_empty() {
                    write_index(directory, first, &segment.frames)?;
                }
                seal(directory, first)?;
                segment.sealed = true;
                sealed_any = true;
            }
            if segment.prefix >= segment.len() {
                remove_segment(directory, first, true)?;
                continue;
            }
            for index in segment.prefix..segment.len() {
                let frame = usize::try_from(index)
                    .ok()
                    .and_then(|index| segment.frames.get(index));
                if let Some(frame) = frame {
                    sizes.insert(first.saturating_add(index), u64::from(frame.len));
                }
            }
            segments.insert(first, segment);
        }
        if sealed_any {
            sync_directory(directory)?;
        }
        let depth = Depth {
            messages: sizes.len().saturating_add(quarantined.len()),
            bytes: sizes
                .values()
                .chain(quarantined.values())
                .fold(0, |sum, size| sum.saturating_add(*size)),
            quarantined: quarantined.len(),
        };
        let next = segments
            .iter()
            .map(|(first, segment)| first.saturating_add(segment.len()))
            .chain(quarantined.keys().map(|last| last.saturating_add(1)))
            .max()
            .unwrap_or(0);
        let mut spool = Self::with(
            Store::Disk {
                directory: directory.to_owned(),
                content,
                sizes,
                quarantined,
                segments,
            },
            bounds,
            depth,
            next,
        );
        spool.writer = Some(Arc::new(Writer::start(spool.shared(), directory)?));
        Ok(spool)
    }

    /// A spool in memory, lost when the process ends: for a development
    /// deployment.
    #[must_use]
    pub fn in_memory(bounds: Bounds) -> Self {
        Self::with(
            Store::Memory {
                queue: VecDeque::new(),
                held: Vec::new(),
            },
            bounds,
            Depth::default(),
            0,
        )
    }

    fn with(store: Store, bounds: Bounds, depth: Depth, next: u64) -> Self {
        let durable = matches!(store, Store::Disk { .. });
        let retire = Arc::new(AtomicBool::new(false));
        let shown = Arc::new(Mutex::new(Ledger {
            depth,
            ..Ledger::default()
        }));
        Self {
            inner: Arc::new(Mutex::new(Inner {
                store,
                bounds,
                depth,
                shown: Arc::clone(&shown),
                next,
                active: None,
                retire: Arc::clone(&retire),
                #[cfg(test)]
                syncs: 0,
                #[cfg(test)]
                segment_syncs: 0,
            })),
            shown,
            waiting: Arc::new(Mutex::new(VecDeque::new())),
            tickets: Arc::new(AtomicU64::new(0)),
            wake: Arc::new(Condvar::new()),
            closed: Arc::new(AtomicBool::new(false)),
            retire,
            #[cfg(test)]
            flush: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            gate: Arc::new(Mutex::new(())),
            writer: None,
            arrived: Arc::new(Notify::new()),
            bounds,
            durable,
        }
    }

    /// Whether the messages are stored on disk.
    #[must_use]
    pub fn is_durable(&self) -> bool {
        self.durable
    }

    /// What the spool holds: a message counts once it is stored.
    #[must_use]
    pub fn depth(&self) -> Depth {
        lock(&self.shown).depth
    }

    /// The messages waiting for their write, which hold their place under
    /// the bounds until it ends.
    #[must_use]
    pub fn queued(&self) -> usize {
        lock(&self.shown).queued
    }

    /// The messages refused with [`SpoolError::Full`] since the spool was
    /// opened.
    #[must_use]
    pub fn refused(&self) -> u64 {
        lock(&self.shown).refused
    }

    /// Stores `message` after every message stored before it. On disk, it
    /// returns once the message is on the device.
    ///
    /// The bounds count the messages queued for a write with those stored,
    /// so a message they have no room for is refused at once, and a stalled
    /// disk holds at most the bounds in memory too. It waits at most
    /// [`Bounds::write_timeout`], the wait for a write before it included. A
    /// message not stored by then is refused with [`SpoolError::Late`] and
    /// is not counted as stored. Its write goes on, or waits on for its
    /// turn: once it ends, the message is stored and delivered like any
    /// other, or the failure is logged with no part of the message.
    ///
    /// # Errors
    ///
    /// [`SpoolError::Full`] past a bound, [`SpoolError::Late`] past
    /// [`Bounds::write_timeout`], and [`SpoolError::Io`] when the message cannot
    /// be written.
    pub async fn push(&self, message: SecretSlice<u8>) -> Result<(), SpoolError> {
        let size = u64::try_from(message.expose_secret().len()).unwrap_or(u64::MAX);
        let place = self.admit(size)?;
        self.push_in(message, place).await
    }

    /// Stores `message` in the `place` it holds under the bounds, waiting at
    /// most [`Bounds::write_timeout`].
    async fn push_in(&self, message: SecretSlice<u8>, place: Place) -> Result<(), SpoolError> {
        let bound = self.bounds.write_timeout;
        let store = self.store(message, place);
        match tokio::time::Instant::now().checked_add(bound) {
            Some(deadline) => tokio::time::timeout_at(deadline, store)
                .await
                .map_err(|_elapsed| SpoolError::Late { bound })?,
            None => store.await,
        }
    }

    /// Waits until a message is stored, returning at once when one was
    /// stored since the last wait ended.
    pub async fn stored(&self) {
        self.arrived.notified().await;
    }

    /// Stores `message` once every write before it has ended.
    ///
    /// The message joins the queue and wakes the writer, which takes every
    /// message queued by the time it reaches the queue as one batch. A caller
    /// that stops waiting while the message is still queued leaves it
    /// queued, and it is written once the writes before it end. A spool in
    /// memory has no writer: the caller writes the queue itself.
    // NOTE: ITI TF-2 §3.20.4.1.1 has a record stored locally and sent when the sender
    // is able, so a queued record its exchange gave up on is still written, never dropped.
    async fn store(&self, message: SecretSlice<u8>, place: Place) -> Result<(), SpoolError> {
        let outcome = self.enqueue(message, place);
        self.write();
        outcome
            .await
            .map_err(|_unfinished| SpoolError::Unfinished)?
            .map(|_sequence| ())
    }

    /// Queues `message` in its `place` for the next batch, returning the
    /// receiver of its outcome.
    fn enqueue(
        &self,
        message: SecretSlice<u8>,
        place: Place,
    ) -> oneshot::Receiver<Result<u64, SpoolError>> {
        let (done, outcome) = oneshot::channel();
        let mut waiting = lock(&self.waiting);
        let ticket = self.tickets.fetch_add(1, Ordering::Relaxed);
        waiting.push_back(Waiting {
            ticket,
            message,
            place,
            done,
        });
        outcome
    }

    /// Has the queue written: wakes the writer of a spool on disk, or writes
    /// it on this thread for a spool in memory.
    fn write(&self) {
        if self.writer.is_some() {
            self.wake.notify_one();
        } else {
            writer::write_queued(&self.shared());
        }
    }

    /// What the writer shares with the callers.
    fn shared(&self) -> Shared {
        Shared {
            waiting: Arc::clone(&self.waiting),
            wake: Arc::clone(&self.wake),
            inner: Arc::clone(&self.inner),
            arrived: Arc::clone(&self.arrived),
            closed: Arc::clone(&self.closed),
            retire: Arc::clone(&self.retire),
            #[cfg(test)]
            flush: Arc::clone(&self.flush),
            #[cfg(test)]
            gate: Arc::clone(&self.gate),
        }
    }

    /// Gives a message of `size` bytes its place under the bounds, counting
    /// the messages queued for a write with those stored, or refuses it.
    fn admit(&self, size: u64) -> Result<Place, SpoolError> {
        let mut ledger = lock(&self.shown);
        let messages = ledger.depth.messages.saturating_add(ledger.queued);
        let bytes = ledger.depth.bytes.saturating_add(ledger.queued_bytes);
        if messages >= self.bounds.max_messages
            || bytes.saturating_add(size) > self.bounds.max_bytes
        {
            let refused = ledger.refuse();
            drop(ledger);
            tracing::warn!(target: crate::atna::targets::SPOOL,
                refused,
                messages,
                bytes,
                "an audit message was refused: the spool and the writes queued for it are at their bounds"
            );
            return Err(SpoolError::Full { messages, bytes });
        }
        ledger.queued = ledger.queued.saturating_add(1);
        ledger.queued_bytes = ledger.queued_bytes.saturating_add(size);
        Ok(Place {
            ledger: Arc::clone(&self.shown),
            size,
            held: true,
        })
    }

    /// Runs `step` on the spool on a blocking thread, and shows the depth it
    /// leaves.
    async fn blocking<T: Send + 'static>(
        &self,
        step: impl FnOnce(&mut Inner) -> Result<T, SpoolError> + Send + 'static,
    ) -> Result<T, SpoolError> {
        let inner = Arc::clone(&self.inner);
        let done = tokio::task::spawn_blocking(move || {
            let mut inner = lock(&inner);
            let done = step(&mut inner);
            inner.publish();
            done
        })
        .await
        .map_err(SpoolError::Task)?;
        if self.retire.load(Ordering::Acquire) {
            self.wake.notify_one();
        }
        done
    }

    /// The oldest message that can be delivered, still stored.
    ///
    /// Every older message that cannot be read, or is no message of the
    /// spool's [`Content`], is moved to the quarantine on the way.
    ///
    /// # Errors
    ///
    /// [`SpoolError::Io`] when a message can neither be read nor moved to
    /// the quarantine.
    pub async fn oldest(&self) -> Result<Option<Stored>, SpoolError> {
        self.blocking(Inner::oldest).await
    }

    /// Up to `max` of the oldest messages that can be delivered, in order,
    /// still stored: the oldest, and those stored after it that read, each
    /// a message of the spool's [`Content`].
    ///
    /// A message on the way that cannot be read, or is no message of the
    /// spool's content, is moved to the quarantine, as [`Spool::oldest`]
    /// does.
    ///
    /// # Errors
    ///
    /// [`SpoolError::Io`] when a message can neither be read nor moved to
    /// the quarantine.
    pub async fn oldest_batch(&self, max: usize) -> Result<Vec<Stored>, SpoolError> {
        self.blocking(move |inner| inner.oldest_batch(max)).await
    }

    /// Moves the message `sequence`, which the repository refused, to the
    /// quarantine, where it stays counted, so the drain goes on.
    ///
    /// # Errors
    ///
    /// [`SpoolError::Io`] when it cannot be moved.
    pub async fn reject(&self, sequence: u64) -> Result<(), SpoolError> {
        self.blocking(move |inner| inner.reject(sequence)).await
    }

    /// Removes the message `sequence`, once it is delivered.
    ///
    /// # Errors
    ///
    /// [`SpoolError::Io`] when it cannot be removed.
    pub async fn remove(&self, sequence: u64) -> Result<(), SpoolError> {
        self.blocking(move |inner| inner.remove(sequence)).await
    }
}

impl Inner {
    /// Shows the depth to readers that do not take the spool's lock.
    fn publish(&self) {
        lock(&self.shown).depth = self.depth;
    }

    /// Shows the depth and gives back the `places` of the messages a batch
    /// wrote in one step, so no reader sees a message both queued and
    /// stored, or neither.
    fn publish_giving_back(&self, places: Vec<Place>) {
        let mut ledger = lock(&self.shown);
        ledger.depth = self.depth;
        for place in places {
            place.give_back(&mut ledger);
        }
    }

    /// Refuses a message of `size` bytes when the spool already holds its
    /// most messages or bytes, `ahead` of them written in its batch and not
    /// yet counted.
    fn check_room(&self, ahead: Depth, size: u64) -> Result<(), SpoolError> {
        let messages = self.depth.messages.saturating_add(ahead.messages);
        let bytes = self.depth.bytes.saturating_add(ahead.bytes);
        if messages >= self.bounds.max_messages
            || bytes.saturating_add(size) > self.bounds.max_bytes
        {
            let refused = lock(&self.shown).refuse();
            tracing::warn!(target: crate::atna::targets::SPOOL,
                refused,
                messages,
                bytes,
                "an audit message was refused: the spool is at its bounds"
            );
            return Err(SpoolError::Full { messages, bytes });
        }
        Ok(())
    }

    /// Counts a message of `size` bytes as stored under `sequence`.
    fn count(&mut self, sequence: u64, size: u64) {
        if let Store::Disk { sizes, .. } = &mut self.store {
            sizes.insert(sequence, size);
        }
        self.depth.messages = self.depth.messages.saturating_add(1);
        self.depth.bytes = self.depth.bytes.saturating_add(size);
    }
}
