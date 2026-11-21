// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The files of a spool on disk, readable by their owner alone.
//!
//! The messages a batch stores are one segment, named by the sequence number
//! of its first message: each message framed by its length and its CRC-32,
//! written as a partial file, flushed to the device once, then renamed. A
//! sidecar beside a segment holds how many of its messages, from the first,
//! are delivered or quarantined; it is written without a flush, so a crash
//! leaves it behind and the next run sends those messages again. A message
//! moved to the quarantine is a file of its own there. No specification
//! governs the layout: our own design.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{Read as _, Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use super::{PARTIAL, SEGMENT, SENT, STORED, SpoolError};

/// The bytes ahead of each message in a segment: its length and its CRC-32,
/// each four bytes big-endian.
const HEADER: usize = 8;

/// Where one message lies in its segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Frame {
    /// The offset of the message's first byte.
    pub offset: u64,
    /// The message's length.
    pub len: u32,
    /// The CRC-32 of the message.
    pub crc: u32,
}

/// One segment as an open reads it.
#[derive(Debug)]
pub(super) struct Loaded {
    /// Its whole frames, in order.
    pub frames: Vec<Frame>,
    /// How many of them, from the first, are delivered or quarantined.
    pub settled: u64,
    /// The bytes after the last whole frame, which no frame can be read
    /// from: a frame whose length runs past the end or whose CRC fails.
    pub remainder: Option<Vec<u8>>,
}

/// The segments in `directory`, by the sequence number of their first
/// message; a partial write a crash left is removed, a sidecar without its
/// segment is removed, and `skip`, the quarantine, is passed over.
pub(super) fn scan_segments(
    directory: &Path,
    skip: &Path,
) -> Result<BTreeMap<u64, Loaded>, SpoolError> {
    let mut segments = BTreeMap::new();
    let mut sidecars = BTreeMap::new();
    for entry in fs::read_dir(directory).map_err(io("read", directory))? {
        let entry = entry.map_err(io("read", directory))?;
        let path = entry.path();
        if path == skip {
            continue;
        }
        match numbered(&path) {
            (Some(SEGMENT), Some(first)) => {
                let bytes = fs::read(&path).map_err(io("read", &path))?;
                let (frames, remainder) = parse(&bytes);
                segments.insert(
                    first,
                    Loaded {
                        frames,
                        settled: 0,
                        remainder,
                    },
                );
            }
            (Some(SENT), Some(first)) => {
                sidecars.insert(first, path);
            }
            // NOTE: a partial file is a write a crash interrupted, which
            // never counted as stored, so it is removed.
            (Some(PARTIAL), Some(_)) => {
                fs::remove_file(&path).map_err(io("removed", &path))?;
            }
            _ => {
                return Err(SpoolError::Foreign {
                    directory: directory.to_owned(),
                    path,
                });
            }
        }
    }
    for (first, path) in sidecars {
        match segments.get_mut(&first) {
            Some(loaded) => {
                let text = fs::read_to_string(&path).map_err(io("read", &path))?;
                // NOTE: no specification governs this: our own design; a sidecar that
                // does not read counts nothing settled, so its messages are sent again.
                let settled = text.trim().parse::<u64>().unwrap_or(0);
                let whole = u64::try_from(loaded.frames.len()).unwrap_or(u64::MAX);
                loaded.settled = settled.min(whole);
            }
            None => fs::remove_file(&path).map_err(io("removed", &path))?,
        }
    }
    Ok(segments)
}

/// The messages in the quarantine `directory`, by sequence number, with
/// their sizes; a partial write a crash left is removed.
pub(super) fn scan_quarantine(directory: &Path) -> Result<BTreeMap<u64, u64>, SpoolError> {
    let mut sizes = BTreeMap::new();
    for entry in fs::read_dir(directory).map_err(io("read", directory))? {
        let entry = entry.map_err(io("read", directory))?;
        let path = entry.path();
        match numbered(&path) {
            (Some(STORED), Some(sequence)) => {
                let size = entry.metadata().map_err(io("read", &path))?.len();
                sizes.insert(sequence, size);
            }
            (Some(PARTIAL), Some(_)) => {
                fs::remove_file(&path).map_err(io("removed", &path))?;
            }
            _ => {
                return Err(SpoolError::Foreign {
                    directory: directory.to_owned(),
                    path,
                });
            }
        }
    }
    Ok(sizes)
}

/// The extension and the sequence number a file of the spool is named by.
fn numbered(path: &Path) -> (Option<&str>, Option<u64>) {
    let extension = path.extension().and_then(OsStr::to_str);
    // NOTE: no specification governs this: our own design; a name that is no
    // sequence number is a file this spool did not write.
    let sequence = path
        .file_stem()
        .and_then(OsStr::to_str)
        .and_then(|it| it.parse::<u64>().ok());
    (extension, sequence)
}

/// The whole frames of a segment's `bytes`, and the bytes after the last
/// one when a frame cannot be read.
pub(super) fn parse(bytes: &[u8]) -> (Vec<Frame>, Option<Vec<u8>>) {
    let mut frames = Vec::new();
    let mut at = 0_usize;
    while at < bytes.len() {
        let Some((frame, next)) = frame_at(bytes, at) else {
            return (frames, bytes.get(at..).map(<[u8]>::to_vec));
        };
        frames.push(frame);
        at = next;
    }
    (frames, None)
}

/// The frame whose header starts at `at` in `bytes`, when it is whole and
/// its CRC holds, with the offset of the next header.
fn frame_at(bytes: &[u8], at: usize) -> Option<(Frame, usize)> {
    let start = at.checked_add(HEADER)?;
    let header = bytes.get(at..start)?;
    let (len, crc) = header.split_at(4);
    let len = u32::from_be_bytes(len.try_into().ok()?);
    let crc = u32::from_be_bytes(crc.try_into().ok()?);
    let end = start.checked_add(usize::try_from(len).ok()?)?;
    let message = bytes.get(start..end)?;
    let frame = Frame {
        offset: u64::try_from(start).ok()?,
        len,
        crc,
    };
    (crc32fast::hash(message) == crc).then_some((frame, end))
}

/// Writes `messages` as the segment whose first message is `first`: framed
/// into a partial file, flushed to the device once, and renamed. The rename
/// is durable once the directory is flushed after it. A partial file a
/// failed write leaves is removed.
pub(super) fn write_segment(
    directory: &Path,
    first: u64,
    messages: &[&[u8]],
) -> Result<Vec<Frame>, SpoolError> {
    let partial = file(directory, first, PARTIAL);
    let mut frames = Vec::with_capacity(messages.len());
    let mut bytes = Vec::new();
    for message in messages {
        let len = u32::try_from(message.len()).map_err(|_too_large| SpoolError::Io {
            action: "written",
            path: partial.clone(),
            source: std::io::Error::from(std::io::ErrorKind::FileTooLarge),
        })?;
        let crc = crc32fast::hash(message);
        bytes.extend_from_slice(&len.to_be_bytes());
        bytes.extend_from_slice(&crc.to_be_bytes());
        frames.push(Frame {
            offset: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            len,
            crc,
        });
        bytes.extend_from_slice(message);
    }
    let mut handle = private_file(&partial)?;
    let written = handle
        .write_all(&bytes)
        .and_then(|()| handle.sync_data())
        .map_err(io("written", &partial));
    drop(handle);
    if let Err(error) = written {
        // NOTE: no specification governs this: our own design; a partial file
        // never counts as stored, and the next open removes one left here.
        let _kept: std::io::Result<()> = fs::remove_file(&partial);
        return Err(error);
    }
    let stored = file(directory, first, SEGMENT);
    fs::rename(&partial, &stored).map_err(io("written", &stored))?;
    Ok(frames)
}

/// Reads the message `frame` names in the segment whose first message is
/// `first`, and checks its CRC.
pub(super) fn read_frame(directory: &Path, first: u64, frame: Frame) -> std::io::Result<Vec<u8>> {
    let mut handle = File::open(file(directory, first, SEGMENT))?;
    handle.seek(SeekFrom::Start(frame.offset))?;
    let mut message = vec![0_u8; usize::try_from(frame.len).unwrap_or(usize::MAX)];
    handle.read_exact(&mut message)?;
    if crc32fast::hash(&message) == frame.crc {
        Ok(message)
    } else {
        Err(std::io::Error::from(std::io::ErrorKind::InvalidData))
    }
}

/// Cuts the segment whose first message is `first` after its whole
/// `frames`, once the bytes after them are kept in the quarantine.
pub(super) fn truncate_segment(
    directory: &Path,
    first: u64,
    frames: &[Frame],
) -> Result<(), SpoolError> {
    let path = file(directory, first, SEGMENT);
    let end = frames
        .last()
        .map_or(0, |frame| frame.offset.saturating_add(u64::from(frame.len)));
    OpenOptions::new()
        .write(true)
        .open(&path)
        .and_then(|handle| handle.set_len(end).and_then(|()| handle.sync_data()))
        .map_err(io("written", &path))
}

/// Records that `settled` messages of the segment whose first message is
/// `first`, from the first, are delivered or quarantined, without a flush.
pub(super) fn write_sent(directory: &Path, first: u64, settled: u64) -> Result<(), SpoolError> {
    let path = file(directory, first, SENT);
    fs::write(&path, settled.to_string()).map_err(io("written", &path))
}

/// Removes the segment whose first message is `first`, and its sidecar.
pub(super) fn remove_segment(directory: &Path, first: u64) -> Result<(), SpoolError> {
    let path = file(directory, first, SEGMENT);
    fs::remove_file(&path).map_err(io("removed", &path))?;
    let sent = file(directory, first, SENT);
    match fs::remove_file(&sent) {
        Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
            Err(io("removed", &sent)(error))
        }
        _ => Ok(()),
    }
}

/// Writes `message` as message `sequence` of the quarantine `directory`,
/// flushed to the device with the directory.
pub(super) fn write_quarantined(
    directory: &Path,
    sequence: u64,
    message: &[u8],
) -> Result<(), SpoolError> {
    let partial = file(directory, sequence, PARTIAL);
    let mut handle = private_file(&partial)?;
    let written = handle
        .write_all(message)
        .and_then(|()| handle.sync_data())
        .map_err(io("written", &partial));
    drop(handle);
    if let Err(error) = written {
        // NOTE: no specification governs this: our own design; a partial file
        // never counts as quarantined, and the next open removes one left here.
        let _kept: std::io::Result<()> = fs::remove_file(&partial);
        return Err(error);
    }
    let stored = file(directory, sequence, STORED);
    fs::rename(&partial, &stored).map_err(io("written", &stored))?;
    sync_directory(directory)
}

/// The first file in `directory` the spool did not write, `skip`, the
/// quarantine, passed over; a partial file a crash left is the spool's own.
/// In the spool a segment and its sidecar are the spool's, in the
/// quarantine (`skip` is `None`) a message. Nothing is removed or written.
pub(super) fn first_foreign(
    directory: &Path,
    skip: Option<&Path>,
) -> Result<Option<PathBuf>, SpoolError> {
    for entry in fs::read_dir(directory).map_err(io("read", directory))? {
        let path = entry.map_err(io("read", directory))?.path();
        if Some(path.as_path()) == skip {
            continue;
        }
        let own = match numbered(&path) {
            (Some(PARTIAL), Some(_)) => true,
            (Some(SEGMENT | SENT), Some(_)) => skip.is_some(),
            (Some(STORED), Some(_)) => skip.is_none(),
            _ => false,
        };
        if !own {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

/// Whether the mode of the directory at `path` lets one class of users,
/// its owner, its group or every other user, both write in it and enter it.
///
/// Only the mode is read: neither the process user nor a read-only mount is,
/// so a directory on a filesystem mounted read-only for a check still passes.
#[cfg(unix)]
pub(super) fn writable_by_mode(path: &Path) -> Result<bool, SpoolError> {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = fs::metadata(path)
        .map_err(io("read", path))?
        .permissions()
        .mode();
    Ok([0o300, 0o030, 0o003]
        .into_iter()
        .any(|bits| mode & bits == bits))
}

#[cfg(not(unix))]
pub(super) fn writable_by_mode(path: &Path) -> Result<bool, SpoolError> {
    Ok(!fs::metadata(path)
        .map_err(io("read", path))?
        .permissions()
        .readonly())
}

/// Writes and removes a partial file, so a directory that cannot take a
/// message is refused when it is opened, not when the first message
/// arrives; one a crash leaves behind is removed by the next open.
pub(super) fn probe(directory: &Path) -> Result<(), SpoolError> {
    let path = file(directory, u64::MAX, PARTIAL);
    let handle = private_file(&path)?;
    drop(handle);
    fs::remove_file(&path).map_err(io("removed", &path))
}

/// The path of the file `sequence` with `extension`, named so that the file
/// names sort in sequence order.
pub(super) fn file(directory: &Path, sequence: u64, extension: &str) -> PathBuf {
    directory.join(format!("{sequence:020}.{extension}"))
}

#[cfg(unix)]
fn private_file(path: &Path) -> Result<File, SpoolError> {
    use std::os::unix::fs::OpenOptionsExt as _;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(io("created", path))
}

#[cfg(not(unix))]
fn private_file(path: &Path) -> Result<File, SpoolError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(io("created", path))
}

#[cfg(unix)]
pub(super) fn sync_directory(directory: &Path) -> Result<(), SpoolError> {
    File::open(directory)
        .and_then(|handle| handle.sync_all())
        .map_err(io("written", directory))
}

// NOTE: no specification governs this: our own design; outside Unix a
// directory cannot be opened to flush it, and the rename is what is relied on.
#[cfg(not(unix))]
pub(super) fn sync_directory(_directory: &Path) -> Result<(), SpoolError> {
    Ok(())
}

#[cfg(unix)]
pub(super) fn create_private(directory: &Path) -> Result<(), SpoolError> {
    use std::os::unix::fs::DirBuilderExt as _;
    if directory.is_dir() {
        return Ok(());
    }
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(directory)
        .map_err(io("created", directory))
}

#[cfg(not(unix))]
pub(super) fn create_private(directory: &Path) -> Result<(), SpoolError> {
    fs::create_dir_all(directory).map_err(io("created", directory))
}

#[cfg(unix)]
pub(super) fn check_private(directory: &Path) -> Result<(), SpoolError> {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = fs::metadata(directory)
        .map_err(io("read", directory))?
        .permissions()
        .mode();
    let shared = mode & 0o077;
    if shared == 0 {
        Ok(())
    } else {
        Err(SpoolError::Exposed(directory.to_owned()))
    }
}

#[cfg(not(unix))]
pub(super) fn check_private(_directory: &Path) -> Result<(), SpoolError> {
    Ok(())
}

/// The [`SpoolError::Io`] of `action` on `path`.
pub(super) fn io<'a>(
    action: &'static str,
    path: &'a Path,
) -> impl FnOnce(std::io::Error) -> SpoolError + 'a {
    move |source| SpoolError::Io {
        action,
        path: path.to_owned(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::{Frame, parse};

    fn framed(messages: &[&[u8]]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for message in messages {
            bytes.extend_from_slice(&u32::try_from(message.len()).unwrap().to_be_bytes());
            bytes.extend_from_slice(&crc32fast::hash(message).to_be_bytes());
            bytes.extend_from_slice(message);
        }
        bytes
    }

    #[test]
    fn whole_frames_read_back_in_order() {
        let bytes = framed(&[b"one", b"", b"three"]);
        let (frames, remainder) = parse(&bytes);
        assert_eq!(None, remainder, "every frame is whole");
        let lengths: Vec<u32> = frames.iter().map(|frame: &Frame| frame.len).collect();
        assert_eq!(vec![3, 0, 5], lengths, "three frames");
    }

    #[test]
    fn a_torn_tail_keeps_the_frames_before_it() {
        let mut bytes = framed(&[b"one", b"two"]);
        let whole = bytes.len();
        bytes.extend_from_slice(&framed(&[b"three"])[..6]);
        let (frames, remainder) = parse(&bytes);
        assert_eq!(2, frames.len(), "the two whole frames are kept");
        assert_eq!(
            Some(bytes[whole..].to_vec()),
            remainder,
            "the torn tail is set apart"
        );
    }

    #[test]
    fn a_corrupt_frame_keeps_the_frames_before_it() {
        let mut bytes = framed(&[b"one", b"two", b"three"]);
        let corrupt = bytes.len() - 2;
        bytes[corrupt] ^= 0xff;
        let (frames, remainder) = parse(&bytes);
        assert_eq!(
            2,
            frames.len(),
            "the frames before the corrupt one are kept"
        );
        assert!(remainder.is_some(), "the corrupt frame is set apart");
    }
}
