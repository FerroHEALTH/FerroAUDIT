// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The drain's side of a spool: finding a stored message, reading it,
//! settling it as delivered or quarantined, and removing a segment once all
//! of its messages are settled. No specification governs the bookkeeping:
//! our own design.

use std::path::Path;

use secrecy::{ExposeSecret, SecretSlice};

use super::disk::{Frame, read_frame, remove_segment, write_quarantined, write_sent};
use super::{Inner, QUARANTINE, SpoolError, Store, Stored};

impl Inner {
    /// The segment that holds message `sequence`, whether it is sealed, and
    /// the message's frame.
    fn locate(&self, sequence: u64) -> Option<(u64, bool, Frame)> {
        let Store::Disk { segments, .. } = &self.store else {
            return None;
        };
        let (first, segment) = segments.range(..=sequence).next_back()?;
        let index = usize::try_from(sequence.checked_sub(*first)?).ok()?;
        segment
            .frames
            .get(index)
            .map(|frame| (*first, segment.sealed, *frame))
    }

    /// Marks the segment whose first message is `first` sealed, once it is
    /// renamed, and removes it when all of its messages are settled.
    pub(super) fn sealed(&mut self, first: u64) -> Result<(), SpoolError> {
        let Store::Disk {
            directory,
            segments,
            ..
        } = &mut self.store
        else {
            return Ok(());
        };
        let Some(segment) = segments.get_mut(&first) else {
            return Ok(());
        };
        segment.sealed = true;
        if segment.prefix >= segment.len() {
            segments.remove(&first);
            return remove_segment(directory, first, true);
        }
        Ok(())
    }

    pub(super) fn oldest(&mut self) -> Result<Option<Stored>, SpoolError> {
        self.oldest_after(None)
    }

    /// The oldest message after `after`, or the oldest of all, that can be
    /// delivered; every one on the way that cannot is quarantined.
    fn oldest_after(&mut self, after: Option<u64>) -> Result<Option<Stored>, SpoolError> {
        let later = |sequence: &u64| after.is_none_or(|after| *sequence > after);
        loop {
            let (directory, content, sequence) = match &self.store {
                Store::Memory { queue, .. } => {
                    return Ok(queue.iter().find(|(sequence, _)| later(sequence)).map(
                        |(sequence, message)| Stored {
                            sequence: *sequence,
                            message: SecretSlice::from(message.expose_secret().to_vec()),
                        },
                    ));
                }
                Store::Disk {
                    directory,
                    content,
                    sizes,
                    ..
                } => {
                    let from = after.map_or(0, |after| after.saturating_add(1));
                    match sizes.range(from..).next() {
                        None => return Ok(None),
                        Some((sequence, _)) => (directory.clone(), *content, *sequence),
                    }
                }
            };
            let Some((first, sealed, frame)) = self.locate(sequence) else {
                tracing::error!(target: crate::atna::targets::SPOOL, sequence, "a spooled audit message is gone from the spool");
                self.forget(sequence)?;
                continue;
            };
            match read_frame(&directory, first, sealed, frame) {
                Ok(message) if content.holds(&message) => {
                    return Ok(Some(Stored {
                        sequence,
                        message: SecretSlice::from(message),
                    }));
                }
                Ok(message) => {
                    self.quarantine(&directory, sequence, &message, content.mismatch())?;
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    tracing::error!(target: crate::atna::targets::SPOOL, sequence, "a spooled audit message is gone from the spool");
                    self.forget(sequence)?;
                }
                Err(error) => {
                    tracing::error!(target: crate::atna::targets::SPOOL, sequence, error = %error, "a spooled audit message cannot be read");
                    self.quarantine(&directory, sequence, &[], "it cannot be read")?;
                }
            }
        }
    }

    pub(super) fn oldest_batch(&mut self, max: usize) -> Result<Vec<Stored>, SpoolError> {
        let mut batch = Vec::new();
        let mut after = None;
        while batch.len() < max {
            let Some(stored) = self.oldest_after(after)? else {
                break;
            };
            after = Some(stored.sequence);
            batch.push(stored);
        }
        Ok(batch)
    }

    /// Drops message `sequence`, which can no longer be read, from the
    /// count.
    fn forget(&mut self, sequence: u64) -> Result<(), SpoolError> {
        if let Store::Disk { sizes, .. } = &mut self.store
            && let Some(size) = sizes.remove(&sequence)
        {
            self.depth.messages = self.depth.messages.saturating_sub(1);
            self.depth.bytes = self.depth.bytes.saturating_sub(size);
        }
        self.settle(sequence)
    }

    /// Records message `sequence` as delivered or quarantined in its
    /// segment, and removes the segment once all of its messages are.
    fn settle(&mut self, sequence: u64) -> Result<(), SpoolError> {
        let active = self.active;
        let Store::Disk {
            directory,
            segments,
            ..
        } = &mut self.store
        else {
            return Ok(());
        };
        let Some((first, segment)) = segments.range_mut(..=sequence).next_back() else {
            return Ok(());
        };
        let first = *first;
        let Some(index) = sequence.checked_sub(first) else {
            return Ok(());
        };
        if index >= segment.len() || index < segment.prefix {
            return Ok(());
        }
        segment.settled.insert(index);
        let before = segment.prefix;
        while segment.settled.remove(&segment.prefix) {
            segment.prefix = segment.prefix.saturating_add(1);
        }
        if segment.prefix >= segment.len() && active != Some(first) {
            let sealed = segment.sealed;
            segments.remove(&first);
            return remove_segment(directory, first, sealed);
        }
        if segment.prefix > before {
            write_sent(directory, first, segment.prefix)?;
        }
        Ok(())
    }

    /// Writes message `sequence`, `message`, to the quarantine, where it
    /// stays counted, and settles it in its segment.
    fn quarantine(
        &mut self,
        directory: &Path,
        sequence: u64,
        message: &[u8],
        why: &'static str,
    ) -> Result<(), SpoolError> {
        let held = directory.join(QUARANTINE);
        write_quarantined(&held, sequence, message)?;
        if let Store::Disk {
            sizes, quarantined, ..
        } = &mut self.store
            && let Some(size) = sizes.remove(&sequence)
        {
            quarantined.insert(sequence, size);
            self.depth.quarantined = self.depth.quarantined.saturating_add(1);
        }
        self.settle(sequence)?;
        tracing::error!(target: crate::atna::targets::SPOOL,
            sequence,
            reason = why,
            "a spooled audit message was moved to the quarantine; the drain goes on"
        );
        Ok(())
    }

    pub(super) fn reject(&mut self, sequence: u64) -> Result<(), SpoolError> {
        match &mut self.store {
            Store::Disk {
                directory, sizes, ..
            } => {
                if !sizes.contains_key(&sequence) {
                    return Ok(());
                }
                let directory = directory.clone();
                let message = match self.locate(sequence) {
                    Some((first, sealed, frame)) => {
                        read_frame(&directory, first, sealed, frame).unwrap_or_default()
                    }
                    None => Vec::new(),
                };
                self.quarantine(
                    &directory,
                    sequence,
                    &message,
                    "the audit repository refused it",
                )
            }
            Store::Memory { queue, held } => {
                let Some(position) = queue.iter().position(|(kept, _)| *kept == sequence) else {
                    return Ok(());
                };
                if let Some(message) = queue.remove(position) {
                    held.push(message);
                    self.depth.quarantined = self.depth.quarantined.saturating_add(1);
                    tracing::error!(target: crate::atna::targets::SPOOL,
                        sequence,
                        reason = "the audit repository refused it",
                        "an audit message held in memory was set aside; the drain goes on"
                    );
                }
                Ok(())
            }
        }
    }

    pub(super) fn remove(&mut self, sequence: u64) -> Result<(), SpoolError> {
        let size = match &mut self.store {
            Store::Disk { sizes, .. } => {
                let Some(size) = sizes.remove(&sequence) else {
                    return Ok(());
                };
                size
            }
            Store::Memory { queue, .. } => {
                let Some(position) = queue.iter().position(|(held, _)| *held == sequence) else {
                    return Ok(());
                };
                queue.remove(position).map_or(0, |(_, message)| {
                    u64::try_from(message.expose_secret().len()).unwrap_or(u64::MAX)
                })
            }
        };
        self.depth.messages = self.depth.messages.saturating_sub(1);
        self.depth.bytes = self.depth.bytes.saturating_sub(size);
        self.settle(sequence)
    }
}
