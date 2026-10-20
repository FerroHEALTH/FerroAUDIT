// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The log targets of the spool and the forwarder are a stable interface an
//! operator's log filter names, never the module layout (no specification
//! governs them: our own design).

use std::io::Write;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use ihe_atna::atna::forwarder::Forwarder;
use ihe_atna::atna::repository::{Repository, Timeouts};
use ihe_atna::atna::spool::{Bounds, Spool};
use ihe_atna::atna::targets;
use secrecy::SecretSlice;
use url::Url;

/// A log sink the test reads back.
#[derive(Clone, Default)]
struct Lines(Arc<Mutex<Vec<u8>>>);

impl Write for Lines {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Lines {
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.0.lock().unwrap_or_else(PoisonError::into_inner)).into_owned()
    }
}

#[test]
fn the_targets_are_the_documented_names() {
    assert_eq!(targets::SPOOL, "ihe_atna::atna::spool");
    assert_eq!(targets::FORWARDER, "ihe_atna::atna::forwarder");
}

/// A forwarder that cannot reach its repository logs under the forwarder's
/// target, and a spooled file of another content is quarantined under the
/// spool's.
#[tokio::test]
async fn each_event_names_its_target() {
    let lines = Lines::default();
    let writer = lines.clone();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(move || writer.clone())
        .with_target(true)
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .finish();
    // NOTE: the spool logs from its blocking file tasks, on threads a scoped default never reaches.
    tracing::subscriber::set_global_default(subscriber).expect("the one global subscriber");

    let closed = std::net::TcpListener::bind("127.0.0.1:0").expect("a port");
    let address = closed.local_addr().expect("an address");
    drop(closed);
    let url = Url::parse(&format!("tcp://{address}")).expect("a URL");
    let timeouts = Timeouts {
        connect: Duration::from_millis(200),
        send: Duration::from_millis(200),
    };
    let repository = Repository::unencrypted_for_development(&url, timeouts).expect("a repository");
    let directory = tempfile::tempdir().expect("a directory");
    let bounds = Bounds {
        max_messages: 8,
        max_bytes: 1 << 20,
        write_timeout: Duration::from_secs(5),
    };
    let spool = Spool::open(&directory.path().join("spool"), bounds).expect("it opens");
    spool
        .push(SecretSlice::from(b"not a frame".to_vec()))
        .await
        .expect("stored");
    spool
        .push(SecretSlice::from(b"5 frame".to_vec()))
        .await
        .expect("stored");
    let forwarder = Forwarder::new(spool, repository, Duration::from_millis(250));
    let running = tokio::spawn(Arc::clone(&forwarder).run());
    let deadline = Instant::now() + Duration::from_secs(5);
    while !(lines.text().contains(targets::FORWARDER) && lines.text().contains(targets::SPOOL))
        && Instant::now() < deadline
    {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    running.abort();
    let text = lines.text();
    assert!(text.contains(targets::FORWARDER), "{text}");
    assert!(text.contains(targets::SPOOL), "{text}");
    assert!(
        !text.contains("ihe_atna::atna::spool::"),
        "no submodule target: {text}"
    );
}
