// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The receiving side of ITI-20 over syslog: RFC 5425 octet-counted frames
//! over TLS, each an RFC 5424 message carrying a DICOM audit message (ITI
//! TF-2 §3.20.4.1).
//!
//! RFC 5425 has no acknowledgement, so a sender counts a frame as delivered
//! once it is written, and a frame the repository read and lost is lost for
//! good. [`receive`] therefore stores every frame in the repository's spool,
//! through [`Forwarder::submit`], before it reads the next. While the spool
//! is full it stops reading and tries again, so the sender's writes wait and
//! its own spool keeps its records. The forwarder built by
//! [`Forwarder::into_store`] delivers each stored frame to the
//! [`AuditStore`](super::AuditStore) as the `AuditEvent` [`super::dicom`]
//! reads, and quarantines a frame the mapping cannot carry.

use std::sync::Arc;
use std::time::Duration;

use secrecy::SecretSlice;
use tokio::io::{AsyncRead, AsyncReadExt as _};
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::atna::forwarder::Forwarder;
use crate::atna::spool::SpoolError;

/// The bounds of what [`receive`] reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The longest `SYSLOG-MSG` accepted, in octets. RFC 5425 §4.3.1 has a
    /// receiver take at least 2048 and should take 8192; a DICOM audit
    /// message is often longer.
    pub max_message: usize,
    /// How long a full spool is waited on before the frame is tried again.
    pub full_wait: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            // NOTE: no specification governs the ceiling above RFC 5425 §4.3.1's floor: our own
            // design.
            max_message: 1 << 20,
            full_wait: Duration::from_millis(200),
        }
    }
}

/// Why a frame or a connection was refused; no variant carries a value.
#[derive(Debug, thiserror::Error)]
pub enum ReceiveError {
    /// The connection could not be read.
    #[error("the syslog connection could not be read")]
    Io(#[source] std::io::Error),
    /// The TLS handshake failed.
    #[error("the TLS handshake failed")]
    Tls(#[source] std::io::Error),
    /// The bytes are no RFC 5425 frame, or its `MSG-LEN` is over the limit.
    #[error("the connection sent no RFC 5425 frame")]
    Frame(#[source] FrameError),
    /// The spool could not store the frame.
    #[error("the frame could not be stored")]
    Spool(#[source] SpoolError),
}

/// Why bytes are no ITI-20 syslog message; no variant carries a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FrameError {
    /// `MSG-LEN` is not `NONZERO-DIGIT *DIGIT` followed by a space (RFC 5425
    /// §4.3).
    #[error("the frame's MSG-LEN is malformed")]
    Length,
    /// `MSG-LEN` is over [`Limits::max_message`].
    #[error("the frame's MSG-LEN is over the limit")]
    TooLong,
    /// The frame ends before `MSG-LEN` octets.
    #[error("the frame ends early")]
    Truncated,
    /// The message is no RFC 5424 `SYSLOG-MSG` with a `MSG` (ITI TF-2
    /// §3.20.4.1.2).
    #[error("the frame holds no RFC 5424 message with a MSG")]
    Message,
}

/// Accepts connections on `listener`, completes each TLS handshake with
/// `acceptor`, and [`receive`]s on each in a task of its own, until the
/// task running this is dropped.
///
/// A failed accept or handshake ends that connection alone.
pub async fn serve(
    listener: TcpListener,
    acceptor: TlsAcceptor,
    forwarder: Arc<Forwarder>,
    limits: Limits,
) {
    loop {
        let (tcp, _) = match listener.accept().await {
            Ok(accepted) => accepted,
            Err(error) => {
                tracing::warn!(target: crate::atna::targets::REPOSITORY, error = %error, "a syslog connection could not be accepted");
                tokio::time::sleep(limits.full_wait).await;
                continue;
            }
        };
        let acceptor = acceptor.clone();
        let forwarder = Arc::clone(&forwarder);
        tokio::spawn(async move {
            let outcome = match acceptor.accept(tcp).await {
                Ok(mut stream) => receive(&mut stream, &forwarder, limits).await,
                Err(error) => Err(ReceiveError::Tls(error)),
            };
            if let Err(error) = outcome {
                tracing::warn!(target: crate::atna::targets::REPOSITORY, error = %crate::atna::chain(&error), "a syslog connection ended with an error");
            }
        });
    }
}

/// Reads RFC 5425 frames from `stream` until it ends, storing each through
/// `forwarder` before reading the next, and returns how many it stored.
///
/// # Errors
///
/// A [`ReceiveError`] when the stream cannot be read, sends bytes that are
/// no ITI-20 frame, or the spool cannot store a frame for a reason other
/// than a want of room; the frames stored before it stay stored.
pub async fn receive<S: AsyncRead + Unpin>(
    stream: &mut S,
    forwarder: &Forwarder,
    limits: Limits,
) -> Result<u64, ReceiveError> {
    let mut stored = 0_u64;
    while let Some(frame) = read_frame(stream, limits.max_message).await? {
        message(&frame).map_err(ReceiveError::Frame)?;
        loop {
            match forwarder.submit(SecretSlice::from(frame.clone())).await {
                // NOTE: a late write stores the frame once it ends (Forwarder::submit), so it
                // counts as stored.
                Ok(()) | Err(SpoolError::Late { .. }) => break,
                Err(SpoolError::Full { .. }) => tokio::time::sleep(limits.full_wait).await,
                Err(error) => return Err(ReceiveError::Spool(error)),
            }
        }
        stored = stored.saturating_add(1);
    }
    Ok(stored)
}

/// Reads one frame, `MSG-LEN SP SYSLOG-MSG`, or `None` at the end of the
/// stream between two frames.
async fn read_frame<S: AsyncRead + Unpin>(
    stream: &mut S,
    max_message: usize,
) -> Result<Option<Vec<u8>>, ReceiveError> {
    let mut frame = Vec::with_capacity(16);
    let mut length = 0_usize;
    loop {
        let mut byte = [0_u8; 1];
        let read = stream.read(&mut byte).await.map_err(ReceiveError::Io)?;
        if read == 0 {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Err(ReceiveError::Frame(FrameError::Truncated))
            };
        }
        let [byte] = byte;
        frame.push(byte);
        match byte {
            b' ' if length > 0 => break,
            b'0' if length == 0 => return Err(ReceiveError::Frame(FrameError::Length)),
            b'0'..=b'9' => {
                length = length
                    .checked_mul(10)
                    .and_then(|length| length.checked_add(usize::from(byte - b'0')))
                    .filter(|length| *length <= max_message)
                    .ok_or(ReceiveError::Frame(FrameError::TooLong))?;
            }
            _ => return Err(ReceiveError::Frame(FrameError::Length)),
        }
    }
    let start = frame.len();
    frame.resize(start.saturating_add(length), 0);
    let body = frame
        .get_mut(start..)
        .ok_or(ReceiveError::Frame(FrameError::Truncated))?;
    stream.read_exact(body).await.map_err(|error| {
        if error.kind() == std::io::ErrorKind::UnexpectedEof {
            ReceiveError::Frame(FrameError::Truncated)
        } else {
            ReceiveError::Io(error)
        }
    })?;
    Ok(Some(frame))
}

/// The `MSG` of the RFC 5424 message in the RFC 5425 `frame`: the DICOM
/// audit message's XML (ITI TF-2 §3.20.4.1.2).
///
/// # Errors
///
/// A [`FrameError`] when `frame` is no whole frame, or its message has no
/// RFC 5424 header, structured data and `MSG`.
pub fn message(frame: &[u8]) -> Result<&[u8], FrameError> {
    if !crate::atna::syslog::is_frame(frame) {
        return Err(FrameError::Length);
    }
    let space = frame
        .iter()
        .position(|byte| *byte == b' ')
        .ok_or(FrameError::Length)?;
    let rest = frame
        .get(space.saturating_add(1)..)
        .ok_or(FrameError::Length)?;
    // NOTE: RFC 5424 §6: HEADER is PRI VERSION and five SP-separated fields.
    let header = rest
        .iter()
        .position(|byte| *byte == b'>')
        .filter(|end| rest.first() == Some(&b'<') && (2..=4).contains(end))
        .ok_or(FrameError::Message)?;
    let mut at = header.saturating_add(1);
    for _ in 0..6 {
        let field = rest
            .get(at..)
            .and_then(|tail| tail.iter().position(|byte| *byte == b' '))
            .filter(|length| *length > 0)
            .ok_or(FrameError::Message)?;
        at = at.saturating_add(field).saturating_add(1);
    }
    let structured = rest.get(at..).ok_or(FrameError::Message)?;
    let after = structured_data(structured).ok_or(FrameError::Message)?;
    let msg = structured
        .get(after..)
        .and_then(|tail| tail.strip_prefix(b" "))
        .filter(|msg| !msg.is_empty())
        .ok_or(FrameError::Message)?;
    Ok(msg.strip_prefix(b"\xef\xbb\xbf").unwrap_or(msg))
}

/// The length of the RFC 5424 §6.3 `STRUCTURED-DATA` at the start of
/// `bytes`: `-`, or one or more `[...]` elements whose quoted values escape
/// `"`, `\` and `]` with a backslash.
fn structured_data(bytes: &[u8]) -> Option<usize> {
    if bytes.first() == Some(&b'-') {
        return Some(1);
    }
    let mut at = 0_usize;
    while bytes.get(at) == Some(&b'[') {
        let mut quoted = false;
        let mut escaped = false;
        loop {
            at = at.checked_add(1)?;
            let byte = *bytes.get(at)?;
            match (quoted, escaped, byte) {
                (true, true, _) => escaped = false,
                (true, false, b'\\') => escaped = true,
                (_, false, b'"') => quoted = !quoted,
                (false, _, b']') => break,
                _ => {}
            }
        }
        at = at.checked_add(1)?;
    }
    (at > 0).then_some(at)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn framed(message: &str) -> Vec<u8> {
        format!("{} {message}", message.len()).into_bytes()
    }

    #[test]
    fn the_msg_follows_the_header_and_the_structured_data() {
        let frame = framed("<85>1 2026-05-19T08:30:00Z host app 42 IHE+RFC-3881 - <AuditMessage/>");
        assert_eq!(message(&frame), Ok(&b"<AuditMessage/>"[..]));
        let frame = framed(
            "<85>1 2026-05-19T08:30:00Z host app 42 IHE+RFC-3881 [a b=\"x\\]y\"][c] \u{feff}<AuditMessage/>",
        );
        assert_eq!(message(&frame), Ok(&b"<AuditMessage/>"[..]));
    }

    #[test]
    fn a_message_with_no_msg_or_header_is_refused() {
        for text in [
            "<85>1 2026-05-19T08:30:00Z host app 42 IHE+RFC-3881 -",
            "<85>1 2026-05-19T08:30:00Z host app 42 IHE+RFC-3881 - ",
            "85>1 2026-05-19T08:30:00Z host app 42 IHE+RFC-3881 - <x/>",
            "<85>1 2026-05-19T08:30:00Z host app 42 IHE+RFC-3881 [a b=\"] <x/>",
        ] {
            assert_eq!(message(&framed(text)), Err(FrameError::Message), "{text}");
        }
        assert_eq!(message(b"3 ab"), Err(FrameError::Length));
    }

    #[tokio::test]
    async fn a_frame_over_the_limit_or_malformed_ends_the_connection() {
        let mut stream: &[u8] = b"99999 <85>";
        let read = read_frame(&mut stream, 100).await;
        assert!(
            matches!(read, Err(ReceiveError::Frame(FrameError::TooLong))),
            "{read:?}"
        );
        let mut stream: &[u8] = b"012 x";
        let read = read_frame(&mut stream, 100).await;
        assert!(
            matches!(read, Err(ReceiveError::Frame(FrameError::Length))),
            "{read:?}"
        );
        let mut stream: &[u8] = b"10 short";
        let read = read_frame(&mut stream, 100).await;
        assert!(
            matches!(read, Err(ReceiveError::Frame(FrameError::Truncated))),
            "{read:?}"
        );
        let mut stream: &[u8] = b"";
        assert!(matches!(read_frame(&mut stream, 100).await, Ok(None)));
    }
}
