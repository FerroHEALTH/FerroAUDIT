// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The one writer of a spool on disk: a thread that owns the active segment.
//!
//! A caller queues its message and wakes the thread. The thread takes every
//! message queued by the time it reaches the queue as one batch, appends the
//! batch to the active segment with one flush, and answers each caller once
//! the flush has returned, so the messages that queue while one flush runs
//! form the next batch and nothing waits for a batch to fill. A full segment
//! is sealed and the next batch starts a new one. When the spool is dropped
//! the thread writes what is still queued, answers it, and ends. A caller
//! that stopped waiting has its outcome logged by the thread, never its
//! message. A spool in memory has no thread: each caller writes the queue
//! itself. No specification governs the writer: our own design.

use std::collections::VecDeque;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::JoinHandle;

use secrecy::ExposeSecret as _;
use tokio::sync::Notify;

use super::disk::{
    Frame, append, create_active, file, remove_segment, seal, segment_extension, sync_directory,
    write_index,
};
use super::{Depth, Inner, Segment, SpoolError, Store, Waiting, lock};

/// The most messages an active segment takes before it is sealed.
pub(super) const SEGMENT_MESSAGES: usize = 4096;

/// The most bytes an active segment takes before it is sealed.
pub(super) const SEGMENT_BYTES: u64 = 8 << 20;

/// The most bytes one batch takes from the queue; the rest wait for the
/// next.
const BATCH_BYTES: u64 = 4 << 20;

/// What the writer and the callers share.
#[derive(Clone)]
pub(super) struct Shared {
    /// The messages waiting for their batch.
    pub waiting: Arc<Mutex<VecDeque<Waiting>>>,
    /// Wakes the writer when a message is queued or the spool closes.
    pub wake: Arc<Condvar>,
    /// The spool's state.
    pub inner: Arc<Mutex<Inner>>,
    /// Woken once a batch stores a message.
    pub arrived: Arc<Notify>,
    /// Set when the spool is dropped.
    pub closed: Arc<AtomicBool>,
    /// Set when every message of the active segment is settled.
    pub retire: Arc<AtomicBool>,
    /// The flush of an append, which a test replaces to fail or delay it.
    #[cfg(test)]
    pub flush: Arc<Mutex<Option<Flush>>>,
    /// Held by a test to keep the messages queued, as a write before them
    /// would.
    #[cfg(test)]
    pub gate: Arc<Mutex<()>>,
}

/// A replacement for the flush of an append, under test.
#[cfg(test)]
pub(super) type Flush = Box<dyn FnMut(&File) -> std::io::Result<()> + Send>;

/// The writer thread of a spool on disk, ended when the last handle of the
/// spool is dropped.
pub(super) struct Writer {
    shared: Shared,
    thread: Option<JoinHandle<()>>,
}

impl Writer {
    /// Starts the writer of the spool `shared` holds.
    pub(super) fn start(shared: Shared, directory: &Path) -> Result<Self, SpoolError> {
        let running = shared.clone();
        // NOTE: no specification governs this: our own design; the writer logs to the
        // subscriber of the code that opened the spool, as a task on its runtime would.
        let logging = tracing::dispatcher::get_default(Clone::clone);
        let thread = std::thread::Builder::new()
            .name(String::from("ihe-atna-spool"))
            .spawn(move || tracing::dispatcher::with_default(&logging, || run(&running)))
            .map_err(|source| SpoolError::Io {
                action: "started",
                path: directory.to_owned(),
                source,
            })?;
        Ok(Self {
            shared,
            thread: Some(thread),
        })
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        self.shared.closed.store(true, Ordering::Release);
        {
            let _queue = lock(&self.shared.waiting);
            self.shared.wake.notify_all();
        }
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            tracing::error!(target: crate::atna::targets::SPOOL,
                "the audit spool's writer ended in a panic"
            );
        }
    }
}

/// The segment the writer appends to.
struct Active {
    first: u64,
    file: File,
    path: PathBuf,
    len: u64,
    frames: Vec<Frame>,
}

/// The writer's loop: waits for a message, writes a batch, until the spool
/// closes with nothing queued.
fn run(shared: &Shared) {
    let mut active: Option<Active> = None;
    loop {
        let queued = {
            let mut queue = lock(&shared.waiting);
            while queue.is_empty() && !shared.retire.load(Ordering::Acquire) {
                if shared.closed.load(Ordering::Acquire) {
                    return;
                }
                queue = shared
                    .wake
                    .wait(queue)
                    .unwrap_or_else(PoisonError::into_inner);
            }
            !queue.is_empty()
        };
        if shared.retire.swap(false, Ordering::AcqRel) {
            retire(shared, &mut active);
        }
        if queued {
            write_batch(shared, &mut active);
        }
    }
}

/// Writes the messages queued now on the caller's thread, for a spool in
/// memory, which has no writer.
pub(super) fn write_queued(shared: &Shared) {
    write_batch(shared, &mut None);
}

/// Writes the messages queued now as one batch and answers each.
fn write_batch(shared: &Shared, active: &mut Option<Active>) {
    let room = active.as_ref().map_or(SEGMENT_MESSAGES, |segment| {
        SEGMENT_MESSAGES.saturating_sub(segment.frames.len())
    });
    #[cfg(test)]
    let gate = lock(&shared.gate);
    let batch = take(&shared.waiting, room.max(1));
    #[cfg(test)]
    drop(gate);
    if batch.is_empty() {
        return;
    }
    let mut inner = lock(&shared.inner);
    let mut outcomes: Vec<Result<u64, SpoolError>> = Vec::with_capacity(batch.len());
    let mut admitted = Vec::new();
    let mut ahead = Depth::default();
    for waiting in &batch {
        let size = u64::try_from(waiting.message.expose_secret().len()).unwrap_or(u64::MAX);
        match inner.check_room(ahead, size) {
            Ok(()) => {
                ahead.messages = ahead.messages.saturating_add(1);
                ahead.bytes = ahead.bytes.saturating_add(size);
                admitted.push((outcomes.len(), size));
                outcomes.push(Ok(0));
            }
            Err(full) => outcomes.push(Err(full)),
        }
    }
    let first = inner.next;
    inner.next = first.saturating_add(u64::try_from(admitted.len()).unwrap_or(u64::MAX));
    let directory = match &inner.store {
        Store::Disk { directory, .. } => Some(directory.clone()),
        Store::Memory { .. } => None,
    };
    match directory {
        None => store_in_memory(shared, inner, batch, outcomes, &admitted, first),
        Some(directory) => {
            drop(inner);
            let written = if admitted.is_empty() {
                Ok(Vec::new())
            } else {
                let bodies: Vec<&[u8]> = admitted
                    .iter()
                    .filter_map(|(at, _)| batch.get(*at))
                    .map(|waiting| waiting.message.expose_secret())
                    .collect();
                write_disk(shared, active, &directory, first, &bodies)
            };
            let mut inner = lock(&shared.inner);
            register(
                &mut inner,
                active.as_ref(),
                written,
                &mut outcomes,
                &admitted,
                first,
            );
            if active.is_none() {
                inner.active = None;
            }
            finish(shared, inner, batch, outcomes);
            let full = active.as_ref().is_some_and(|segment| {
                segment.frames.len() >= SEGMENT_MESSAGES || segment.len >= SEGMENT_BYTES
            });
            if full && let Some(segment) = active.take() {
                roll(shared, &directory, segment);
            }
        }
    }
}

/// Keeps the `admitted` messages of `batch`, numbered from `first`, in the
/// memory of a spool in memory, and answers each.
fn store_in_memory(
    shared: &Shared,
    mut inner: std::sync::MutexGuard<'_, Inner>,
    mut batch: Vec<Waiting>,
    mut outcomes: Vec<Result<u64, SpoolError>>,
    admitted: &[(usize, u64)],
    first: u64,
) {
    let mut sequence = first;
    for &(at, size) in admitted {
        if let (Some(waiting), Store::Memory { queue, .. }) = (batch.get_mut(at), &mut inner.store)
        {
            let message =
                std::mem::replace(&mut waiting.message, secrecy::SecretSlice::from(Vec::new()));
            queue.push_back((sequence, message));
        }
        inner.count(sequence, size);
        if let Some(outcome) = outcomes.get_mut(at) {
            *outcome = Ok(sequence);
        }
        sequence = sequence.saturating_add(1);
    }
    finish(shared, inner, batch, outcomes);
}

/// Counts the `admitted` messages, numbered from `first`, as stored in
/// `active` with their `written` frames, or records why they are not.
fn register(
    inner: &mut Inner,
    active: Option<&Active>,
    written: Result<Vec<Frame>, SpoolError>,
    outcomes: &mut [Result<u64, SpoolError>],
    admitted: &[(usize, u64)],
    first: u64,
) {
    match written {
        Ok(frames) => {
            if let (Some(segment), false) = (active, frames.is_empty()) {
                let first_of = segment.first;
                if let Store::Disk { segments, .. } = &mut inner.store {
                    segments
                        .entry(first_of)
                        .or_insert_with(|| Segment {
                            frames: Vec::new(),
                            prefix: 0,
                            settled: std::collections::BTreeSet::new(),
                            sealed: false,
                        })
                        .frames
                        .extend(frames);
                }
                inner.active = Some(first_of);
            }
            let mut sequence = first;
            for &(at, size) in admitted {
                inner.count(sequence, size);
                if let Some(outcome) = outcomes.get_mut(at) {
                    *outcome = Ok(sequence);
                }
                sequence = sequence.saturating_add(1);
            }
        }
        Err(error) => {
            let mut error = Some(error);
            for &(at, _) in admitted {
                let failed = error.take().unwrap_or(SpoolError::Unfinished);
                if let Some(outcome) = outcomes.get_mut(at) {
                    *outcome = Err(failed);
                }
            }
        }
    }
}

/// Takes the messages queued now, at most `room` of them and up to
/// [`BATCH_BYTES`], the first always.
fn take(waiting: &Mutex<VecDeque<Waiting>>, room: usize) -> Vec<Waiting> {
    let mut queue = lock(waiting);
    let mut batch = Vec::new();
    let mut bytes: u64 = 0;
    while let Some(next) = queue.front() {
        let size = u64::try_from(next.message.expose_secret().len()).unwrap_or(u64::MAX);
        if !batch.is_empty() && (batch.len() >= room || bytes.saturating_add(size) > BATCH_BYTES) {
            break;
        }
        bytes = bytes.saturating_add(size);
        if let Some(waiting) = queue.pop_front() {
            batch.push(waiting);
        }
    }
    batch
}

/// Appends `bodies`, messages `first` on, to the active segment, opening one
/// when none is active or the active one does not continue at `first`.
fn write_disk(
    shared: &Shared,
    active: &mut Option<Active>,
    directory: &Path,
    first: u64,
    bodies: &[&[u8]],
) -> Result<Vec<Frame>, SpoolError> {
    let continues = active.as_ref().is_some_and(|segment| {
        segment
            .first
            .saturating_add(u64::try_from(segment.frames.len()).unwrap_or(u64::MAX))
            == first
    });
    if !continues {
        if let Some(segment) = active.take() {
            roll(shared, directory, segment);
        }
        let opened = create_active(directory, first)?;
        sync(shared, directory)?;
        *active = Some(Active {
            first,
            file: opened,
            path: file(directory, first, segment_extension(false)),
            len: 0,
            frames: Vec::new(),
        });
    }
    let Some(segment) = active.as_mut() else {
        return Err(SpoolError::Unfinished);
    };
    count_flush(shared);
    #[cfg(test)]
    let flushed = {
        let flush = Arc::clone(&shared.flush);
        append(
            &mut segment.file,
            &segment.path,
            segment.len,
            bodies,
            &mut |handle| match lock(&flush).as_mut() {
                Some(replaced) => replaced(handle),
                None => handle.sync_data(),
            },
        )
    };
    #[cfg(not(test))]
    let flushed = append(
        &mut segment.file,
        &segment.path,
        segment.len,
        bodies,
        &mut |handle: &File| handle.sync_data(),
    );
    match flushed {
        Ok(frames) => {
            segment.len = frames.last().map_or(segment.len, |frame| {
                frame.offset.saturating_add(u64::from(frame.len))
            });
            segment.frames.extend_from_slice(&frames);
            Ok(frames)
        }
        Err(failed) => {
            // NOTE: no specification governs this: our own design; the failed batch took
            // sequence numbers, so the next batch starts a segment of its own.
            if let Some(segment) = active.take() {
                if failed.kept && segment.frames.is_empty() {
                    let removed = remove_segment(directory, segment.first, false);
                    if let Err(error) = removed {
                        tracing::error!(target: crate::atna::targets::SPOOL,
                            error = %super::super::chain(&error),
                            "an empty audit spool segment could not be removed"
                        );
                    }
                }
                drop(segment);
            }
            Err(failed.error)
        }
    }
}

/// Removes the active segment once every message of it is settled, which
/// only the writer does, so no append reaches a removed file.
fn retire(shared: &Shared, active: &mut Option<Active>) {
    let Some(first) = active.as_ref().map(|segment| segment.first) else {
        return;
    };
    let mut inner = lock(&shared.inner);
    let Store::Disk {
        directory,
        segments,
        ..
    } = &mut inner.store
    else {
        return;
    };
    let settled = segments
        .get(&first)
        .is_some_and(|segment| segment.prefix >= segment.len());
    if !settled {
        return;
    }
    segments.remove(&first);
    let removed = remove_segment(directory, first, false);
    inner.active = None;
    drop(inner);
    *active = None;
    if let Err(error) = removed {
        tracing::error!(target: crate::atna::targets::SPOOL,
            error = %super::super::chain(&error),
            "a settled audit spool segment could not be removed; the next open removes it"
        );
    }
}

/// Seals `segment`: its index written, then renamed `.seg` and the
/// directory flushed; a segment whose messages are all settled is removed.
fn roll(shared: &Shared, directory: &Path, segment: Active) {
    let Active { first, frames, .. } = segment;
    if let Err(error) = write_index(directory, first, &frames) {
        tracing::error!(target: crate::atna::targets::SPOOL,
            error = %super::super::chain(&error),
            "an audit spool segment's index could not be written; the next open reads the segment"
        );
    }
    let sealed = {
        let mut inner = lock(&shared.inner);
        inner.active = None;
        let renamed = seal(directory, first);
        if renamed.is_ok() {
            inner.sealed(first)
        } else {
            renamed
        }
    };
    let flushed = sealed.and_then(|()| sync(shared, directory));
    if let Err(error) = flushed {
        tracing::error!(target: crate::atna::targets::SPOOL,
            error = %super::super::chain(&error),
            "an audit spool segment could not be sealed; the next open seals it"
        );
    }
}

/// Flushes `directory`, counted under test.
#[cfg_attr(
    not(test),
    expect(unused_variables, reason = "the flushes are counted only under test")
)]
fn sync(shared: &Shared, directory: &Path) -> Result<(), SpoolError> {
    #[cfg(test)]
    {
        let mut inner = lock(&shared.inner);
        inner.syncs = inner.syncs.saturating_add(1);
    }
    sync_directory(directory)
}

/// Counts a segment flush under test.
#[cfg_attr(
    not(test),
    expect(unused_variables, reason = "the flushes are counted only under test")
)]
fn count_flush(shared: &Shared) {
    #[cfg(test)]
    {
        let mut inner = lock(&shared.inner);
        inner.segment_syncs = inner.segment_syncs.saturating_add(1);
    }
}

/// Shows the depth, gives the places back, wakes the drain and answers each
/// caller of `batch` with its outcome.
fn finish(
    shared: &Shared,
    inner: std::sync::MutexGuard<'_, Inner>,
    batch: Vec<Waiting>,
    outcomes: Vec<Result<u64, SpoolError>>,
) {
    let mut places = Vec::with_capacity(batch.len());
    let mut answers = Vec::with_capacity(batch.len());
    for waiting in batch {
        places.push(waiting.place);
        answers.push(waiting.done);
    }
    inner.publish_giving_back(places);
    drop(inner);
    if outcomes.iter().any(Result::is_ok) {
        shared.arrived.notify_one();
    }
    for (done, outcome) in answers.into_iter().zip(outcomes) {
        if let Err(unread) = done.send(outcome) {
            super::in_flight::abandoned(unread);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    use secrecy::{ExposeSecret as _, SecretSlice};

    use super::super::disk::{Frame, file, parse, write_index};
    use super::super::in_flight::tests::{BOUND, Logs, PATIENT, SLACK, Stall, naming_the_patient};
    use super::super::withdraw::Withdrawable;
    use super::super::{Bounds, Depth, INDEX, OPEN, SEGMENT, SENT, Spool, SpoolError, lock};
    use super::SEGMENT_MESSAGES;

    fn bounds(write_timeout: Duration) -> Bounds {
        Bounds {
            max_messages: 1 << 21,
            max_bytes: 1 << 34,
            write_timeout,
        }
    }

    /// A message of `index`, one RFC 5425 frame, so the drain delivers it.
    fn message(index: usize) -> SecretSlice<u8> {
        let text = format!("<AuditMessage>{index:08}</AuditMessage>");
        SecretSlice::from(format!("{} {text}", text.len()).into_bytes())
    }

    /// The files of `directory` with `extension`.
    fn files(directory: &Path, extension: &str) -> usize {
        std::fs::read_dir(directory)
            .expect("the directory reads")
            .filter_map(Result::ok)
            .filter(|entry| entry.path().extension().is_some_and(|it| it == extension))
            .count()
    }

    /// Drains `spool`, returning every message's text in order.
    async fn drained(spool: &Spool) -> Vec<String> {
        let mut texts = Vec::new();
        while let Some(stored) = spool.oldest().await.expect("it reads") {
            texts.push(String::from_utf8_lossy(stored.message.expose_secret()).into_owned());
            spool.remove(stored.sequence).await.expect("it removes");
        }
        texts
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_pushes_share_flushes_and_fill_segments_sealed_with_an_index() {
        const PUSHES: usize = SEGMENT_MESSAGES + 904;
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("spool");
        let spool = Spool::open(&path, bounds(SLACK + SLACK)).expect("it opens");
        let pushes: Vec<_> = (0..PUSHES)
            .map(|index| {
                let spool = spool.clone();
                tokio::spawn(async move { spool.push(message(index)).await })
            })
            .collect();
        for push in pushes {
            push.await.expect("the push ends").expect("stored");
        }
        assert_eq!(PUSHES, spool.depth().messages);
        let flushes = lock(&spool.inner).segment_syncs;
        assert!(
            flushes < u64::try_from(PUSHES).expect("small"),
            "the pushes that queue during a flush share the next one: {flushes} flushes"
        );
        assert_eq!(1, files(&path, SEGMENT), "the full segment is sealed");
        assert_eq!(1, files(&path, INDEX), "with its index");
        assert_eq!(1, files(&path, OPEN), "the rest go to the next segment");
        assert_eq!(
            SEGMENT_MESSAGES,
            parse(&std::fs::read(file(&path, 0, SEGMENT)).expect("it reads"))
                .0
                .len(),
            "a sealed segment holds its whole count"
        );
        let texts = drained(&spool).await;
        assert_eq!(PUSHES, texts.len(), "every message is delivered once");
        assert_eq!(
            0,
            files(&path, SEGMENT),
            "a settled sealed segment is removed"
        );
    }

    #[tokio::test]
    async fn no_caller_is_answered_before_its_batch_is_flushed() {
        let logs = Logs::default();
        let _logging = tracing::subscriber::set_default(logs.subscriber());
        let directory = tempfile::tempdir().expect("a directory");
        let spool = Spool::open(&directory.path().join("spool"), bounds(BOUND)).expect("it opens");
        let (release, released) = mpsc::channel::<()>();
        let released = std::sync::Mutex::new(released);
        *lock(&spool.flush) = Some(Box::new(move |handle| {
            // The first flush waits for the release; once the sender is gone the
            // later ones flush at once.
            let _released = lock(&released).recv().is_ok();
            handle.sync_data()
        }));

        let asked = Instant::now();
        let late = spool.push(naming_the_patient()).await;
        assert!(
            matches!(late, Err(SpoolError::Late { bound }) if bound == BOUND),
            "unanswered while its flush has not returned: {late:?}"
        );
        assert!(asked.elapsed() < BOUND + SLACK, "{:?}", asked.elapsed());
        assert_eq!(Depth::default(), spool.depth(), "nothing counts as stored");

        let withdrawn = spool
            .reserve(64)
            .expect("room")
            .push_or_withdraw(vec![Withdrawable::new(naming_the_patient(), message(1))])
            .await;
        assert!(
            matches!(withdrawn, Err(SpoolError::Withdrawn { .. })),
            "queued behind the flush, it is withdrawn at its bound: {withdrawn:?}"
        );

        release.send(()).expect("the flush waits");
        drop(release);
        let until = Instant::now() + SLACK;
        while spool.depth().messages < 2 && Instant::now() < until {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        let texts = drained(&spool).await;
        assert_eq!(
            2,
            texts.len(),
            "the late message and the refusal: {texts:?}"
        );
        assert!(texts.first().is_some_and(|text| text.contains(PATIENT)));
        assert_eq!(
            Some(String::from_utf8_lossy(message(1).expose_secret()).into_owned()),
            texts.get(1).cloned(),
            "the withdrawn message's refusal, never the message"
        );
        let log = logs
            .wait_for("stored after its exchange stopped waiting")
            .await;
        assert!(
            log.contains("stored after its exchange stopped waiting"),
            "{log}"
        );
        assert!(!log.contains(PATIENT), "no log names the patient: {log}");
    }

    #[tokio::test]
    async fn a_failed_flush_answers_its_batch_as_unstored_and_the_next_one_stands() {
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("spool");
        let spool = Spool::open(&path, bounds(SLACK)).expect("it opens");
        spool.push(message(0)).await.expect("stored");
        *lock(&spool.flush) = Some(Box::new(|_handle| {
            Err(std::io::Error::other("the device refused the flush"))
        }));
        let failed = spool.push(message(1)).await;
        assert!(matches!(failed, Err(SpoolError::Io { .. })), "{failed:?}");
        assert_eq!(
            1,
            spool.depth().messages,
            "the failed message is not counted"
        );
        *lock(&spool.flush) = None;
        spool.push(message(2)).await.expect("the next batch stands");
        let texts = drained(&spool).await;
        let expected: Vec<String> = [0, 2]
            .into_iter()
            .map(|index| String::from_utf8_lossy(message(index).expose_secret()).into_owned())
            .collect();
        assert_eq!(expected, texts, "the failed message is never delivered");
        drop(spool);
        let reopened = Spool::open(&path, bounds(SLACK)).expect("it reopens");
        assert_eq!(Depth::default(), reopened.depth(), "nothing comes back");
    }

    #[tokio::test]
    async fn a_crash_between_the_index_and_the_seal_opens_and_delivers_each_message_once() {
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("spool");
        {
            let spool = Spool::open(&path, bounds(SLACK)).expect("it opens");
            for index in 0..3 {
                spool.push(message(index)).await.expect("stored");
            }
        }
        let frames: Vec<Frame> =
            parse(&std::fs::read(file(&path, 0, OPEN)).expect("the active segment reads")).0;
        write_index(&path, 0, &frames).expect("the index is written");
        assert!(file(&path, 0, INDEX).is_file() && file(&path, 0, OPEN).is_file());

        let reopened = Spool::open(&path, bounds(SLACK)).expect("it reopens");
        assert_eq!(3, reopened.depth().messages);
        assert_eq!(3, drained(&reopened).await.len(), "each once");
        drop(reopened);
        let again = Spool::open(&path, bounds(SLACK)).expect("it reopens again");
        assert_eq!(Depth::default(), again.depth(), "none comes back");
    }

    #[tokio::test]
    async fn an_index_that_disagrees_with_its_segment_is_passed_over_for_the_segment() {
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("spool");
        {
            let spool = Spool::open(&path, bounds(SLACK)).expect("it opens");
            for index in 0..2 {
                spool.push(message(index)).await.expect("stored");
            }
        }
        drop(Spool::open(&path, bounds(SLACK)).expect("it seals"));
        write_index(&path, 0, &[]).expect("a lying index");
        let reopened = Spool::open(&path, bounds(SLACK)).expect("it reopens");
        assert_eq!(2, drained(&reopened).await.len(), "read from the segment");
    }

    #[tokio::test]
    async fn an_append_a_crash_cut_short_is_cut_off_and_never_quarantined() {
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("spool");
        {
            let spool = Spool::open(&path, bounds(SLACK)).expect("it opens");
            spool.push(message(0)).await.expect("stored");
        }
        let active = file(&path, 0, OPEN);
        let mut bytes = std::fs::read(&active).expect("it reads");
        let whole = bytes.len();
        bytes.extend_from_slice(&bytes.clone()[..whole - 3]);
        std::fs::write(&active, &bytes).expect("a torn append");
        let reopened = Spool::open(&path, bounds(SLACK)).expect("it reopens");
        assert_eq!(
            Depth {
                messages: 1,
                bytes: u64::try_from(whole - 8).expect("small"),
                quarantined: 0,
            },
            reopened.depth(),
            "the acknowledged message stays and the torn append is gone"
        );
        assert_eq!(1, drained(&reopened).await.len());
    }

    #[tokio::test]
    async fn dropping_the_spool_writes_every_message_still_queued() {
        const QUEUED: usize = 5;
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("spool");
        let spool = Spool::open(&path, bounds(BOUND)).expect("it opens");
        let held = Stall::queue(&spool);
        for index in 0..QUEUED {
            let late = spool.push(message(index)).await;
            assert!(matches!(late, Err(SpoolError::Late { .. })), "{late:?}");
        }
        assert_eq!(QUEUED, spool.queued(), "every message waits");
        let releasing = std::thread::spawn(move || {
            std::thread::sleep(BOUND);
            held.release();
        });
        drop(spool);
        releasing.join().expect("the release ends");
        let reopened = Spool::open(&path, bounds(SLACK)).expect("it reopens");
        assert_eq!(QUEUED, drained(&reopened).await.len(), "each written once");
    }

    #[tokio::test]
    async fn the_active_segment_is_removed_once_every_message_of_it_is_delivered() {
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("spool");
        let spool = Spool::open(&path, bounds(SLACK)).expect("it opens");
        for index in 0..2 {
            spool.push(message(index)).await.expect("stored");
        }
        assert_eq!(
            1,
            files(&path, OPEN),
            "the messages wait in the active segment"
        );
        assert_eq!(2, drained(&spool).await.len());
        let until = Instant::now() + SLACK;
        while files(&path, OPEN) > 0 && Instant::now() < until {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        assert_eq!(
            0,
            files(&path, OPEN),
            "no delivered message stays on the disk"
        );
        assert_eq!(0, files(&path, SENT), "nor its sidecar");
        spool
            .push(message(2))
            .await
            .expect("the next message opens a segment");
        assert_eq!(1, drained(&spool).await.len());
    }

    #[test]
    fn a_spool_of_a_million_messages_opens_in_seconds() {
        const MESSAGES: usize = 1 << 20;
        let directory = tempfile::tempdir().expect("a directory");
        let path = directory.path().join("spool");
        drop(Spool::open(&path, bounds(SLACK)).expect("it opens"));
        let body = message(0);
        let mut first = 0_u64;
        let mut left = MESSAGES;
        while left > 0 {
            let count = left.min(SEGMENT_MESSAGES);
            let bodies: Vec<&[u8]> = std::iter::repeat_n(body.expose_secret(), count).collect();
            let mut bytes = Vec::new();
            for message in &bodies {
                bytes
                    .extend_from_slice(&u32::try_from(message.len()).expect("small").to_be_bytes());
                bytes.extend_from_slice(&crc32fast::hash(message).to_be_bytes());
                bytes.extend_from_slice(message);
            }
            std::fs::write(file(&path, first, SEGMENT), &bytes).expect("a segment");
            write_index(&path, first, &parse(&bytes).0).expect("its index");
            first += u64::try_from(count).expect("small");
            left -= count;
        }
        let opening = Instant::now();
        let spool = Spool::open(&path, bounds(SLACK)).expect("it opens");
        let took = opening.elapsed();
        assert_eq!(MESSAGES, spool.depth().messages);
        assert!(took < Duration::from_secs(10), "opened in {took:?}");
    }
}
