// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The Audit Record Repository FerroAUDIT's performance gate loads (#331).
//!
//! One process mounts `ihe-atna` as a product does: the ATX: FHIR Feed and
//! ITI-81 on an HTTP server, and the ITI-20 syslog receiver on a TLS
//! listener whose spool delivers into the store. A `POST /syslog` writes a
//! DICOM audit message and hands it to an ITI-20 sender, whose spool and
//! forwarder deliver it over RFC 5425 TLS to that listener, so the gate
//! loads the sender and the receiver through HTTP. Both spools are on disk
//! under the work directory. The store keeps a fixed corpus that ITI-81
//! searches and a bounded ring of what it receives.
//!
//! `perf-arr LISTEN WORK_DIR` serves until it is killed. No specification
//! governs this harness: our own design.

mod routes;
mod store;
mod tls;

use std::error::Error;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use ihe_atna::arr::AuditStore;
use ihe_atna::arr::syslog::{Limits, serve};
use ihe_atna::atna::forwarder::Forwarder;
use ihe_atna::atna::repository::{Repository, Timeouts};
use ihe_atna::atna::retry::Retry;
use ihe_atna::atna::spool::{Bounds, Spool};
use ihe_atna::atna::syslog::Sender;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use url::Url;

use crate::routes::State;
use crate::store::Store;

/// Each spool's bounds: room for every message a gate run sends, so a
/// slow delivery shows as latency and memory, never as a refusal.
const BOUNDS: Bounds = Bounds {
    max_messages: 1_000_000,
    max_bytes: 1 << 30,
    write_timeout: Duration::from_secs(5),
};

/// The sender's connection timeouts.
const TIMEOUTS: Timeouts = Timeouts {
    connect: Duration::from_secs(5),
    send: Duration::from_secs(5),
};

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let (Some(listen), Some(work), None) = (args.next(), args.next(), args.next()) else {
        return usage();
    };
    let Some(listen) = listen.to_str().and_then(|s| s.parse::<SocketAddr>().ok()) else {
        return usage();
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => return failed(&error),
    };
    match runtime.block_on(run(listen, PathBuf::from(work))) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => failed(error.as_ref()),
    }
}

#[expect(
    clippy::print_stderr,
    reason = "the harness binary reports its usage on stderr"
)]
fn usage() -> ExitCode {
    eprintln!("usage: perf-arr LISTEN WORK_DIR");
    ExitCode::from(2)
}

#[expect(
    clippy::print_stderr,
    reason = "the harness binary reports why it stopped on stderr"
)]
fn failed(error: &dyn Error) -> ExitCode {
    eprintln!("perf-arr: {error}");
    let mut source = error.source();
    while let Some(cause) = source {
        eprintln!("  caused by: {cause}");
        source = cause.source();
    }
    ExitCode::FAILURE
}

async fn run(listen: SocketAddr, work: PathBuf) -> Result<(), Box<dyn Error>> {
    let store = Arc::new(Store::seeded()?);
    let pki = tls::Pki::mint()?;
    let retry = Retry::new(5, Duration::from_millis(50), Duration::from_secs(1))?;

    let syslog = TcpListener::bind("127.0.0.1:0").await?;
    let syslog_port = syslog.local_addr()?.port();
    let kept: Arc<dyn AuditStore> = store.clone();
    let receiving = Forwarder::into_store(Spool::open(&work.join("arr"), BOUNDS)?, kept, retry);
    tokio::spawn(Arc::clone(&receiving).run());
    tokio::spawn(serve(
        syslog,
        TlsAcceptor::from(pki.server()?),
        receiving,
        Limits::default(),
    ));

    let repository = Repository::tls(
        &Url::parse(&format!("tls://localhost:{syslog_port}"))?,
        pki.client()?,
        TIMEOUTS,
    )?;
    let sending = Forwarder::new(
        Spool::open(&work.join("sender"), BOUNDS)?,
        repository,
        Duration::from_secs(1),
    );
    tokio::spawn(Arc::clone(&sending).run());

    let state = Arc::new(State {
        store,
        sending,
        sender: Sender::new("perf.example.org", "perf-arr", "1")?,
        endpoint: format!("http://{listen}/fhir/AuditEvent"),
    });
    let http = TcpListener::bind(listen).await?;
    axum::serve(http, routes::router(state)).await?;
    Ok(())
}
