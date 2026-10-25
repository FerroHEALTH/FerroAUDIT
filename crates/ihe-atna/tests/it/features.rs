// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The feature boundaries, read from the crate graph one feature at a time
//! (no specification governs them: our own design).
//!
//! `atna` compiles no FHIR model and no HTTP client: the syslog sender needs
//! neither, so a caller that audits over syslog alone links none of them.
//! `balp` adds both for the `AuditEvent` and the FHIR Feed, and compiles
//! neither the RFC 5425 TLS stack nor, of its own, the DICOM message's XML
//! writer (owner decision of 2026-10-20): the Feed is an HTTP create whose
//! TLS is the caller's client's. Neither feature compiles the message ids of an XCPD
//! exchange, which the ITI-55 audit message's caller supplies.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The crates only `balp` compiles.
const BALP_ONLY: &[&str] = &["fhir-types", "reqwest"];

/// The crates of the RFC 5425 TLS connection, which only `atna` compiles.
const ATNA_ONLY: &[&str] = &["tokio-rustls", "rustls-platform-verifier"];

/// The DICOM message's XML writer, which only `atna` depends on itself;
/// `fhir-types` takes it in for its own XML form under `balp`.
const ATNA_DIRECT: &[&str] = &["quick-xml"];

/// The crates no feature compiles.
const NEVER: &[&str] = &["uuid"];

/// The workspace root, two levels above this crate's manifest.
const ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../..");

/// A cargo home of its own for a child cargo, in a temporary directory whose
/// `registry` and `git` link to the real home's.
///
/// The child resolves offline from the lockfile and the registry already on
/// disk (`--frozen`), which only reads those caches, yet cargo still takes
/// the package cache lock of its home, and a cargo building elsewhere on the
/// machine holds that lock for its whole build. No other cargo takes the lock
/// of this home (the Cargo book, "Cargo Home").
fn private_cargo_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("a temporary cargo home");
    let shared = std::env::var_os("CARGO_HOME").map_or_else(
        || {
            std::env::home_dir()
                .expect("CARGO_HOME or a home directory names the cargo home")
                .join(".cargo")
        },
        PathBuf::from,
    );
    for cache in ["registry", "git"] {
        let source = shared.join(cache);
        if source.exists() {
            std::os::unix::fs::symlink(&source, home.path().join(cache))
                .expect("the shared cache links into the private home");
        }
    }
    home
}

/// The package names `cargo tree` lists for this crate with only `feature`
/// on, through normal dependencies, down to `depth` when given.
fn tree(feature: &str, depth: Option<&str>) -> BTreeSet<String> {
    let home = private_cargo_home();
    let mut command = Command::new(env!("CARGO"));
    command
        .env("CARGO_HOME", home.path())
        .env("CARGO_CACHE_AUTO_CLEAN_FREQUENCY", "never")
        .arg("tree")
        .arg("--manifest-path")
        .arg(Path::new(ROOT).join("Cargo.toml"))
        .args(["--package", "ihe-atna"])
        .args(["--edges", "normal"])
        .args(["--prefix", "none", "--format", "{p}"])
        .args(["--no-default-features", "--features", feature, "--frozen"]);
    if let Some(depth) = depth {
        command.args(["--depth", depth]);
    }
    let output = command.output().expect("cargo tree runs");
    assert!(
        output.status.success(),
        "cargo tree failed for {feature}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let names: BTreeSet<String> = String::from_utf8(output.stdout)
        .expect("UTF-8 output")
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect();
    assert!(
        names.contains("ihe-atna"),
        "cargo tree for {feature} did not list the crate itself, so its output was not read"
    );
    names
}

#[test]
fn only_balp_compiles_the_fhir_model_and_the_http_client() {
    let atna = tree("atna", None);
    let balp = tree("balp", None);
    for name in BALP_ONLY {
        assert!(balp.contains(*name), "balp lost {name}: {balp:?}");
        assert!(!atna.contains(*name), "atna compiles {name}: {atna:?}");
    }
    let balp_direct = tree("balp", Some("1"));
    for name in BALP_ONLY {
        assert!(balp_direct.contains(*name), "balp names {name}");
    }
}

#[test]
fn no_feature_compiles_the_xcpd_message_ids() {
    for feature in ["atna", "balp"] {
        let closure = tree(feature, None);
        for name in NEVER {
            assert!(
                !closure.contains(*name),
                "{feature} compiles {name}: {closure:?}"
            );
        }
    }
}

#[test]
fn only_atna_compiles_the_syslog_sender_and_its_tls_stack() {
    let atna = tree("atna", None);
    let balp = tree("balp", None);
    for name in ATNA_ONLY {
        assert!(atna.contains(*name), "atna lost {name}: {atna:?}");
        assert!(!balp.contains(*name), "balp compiles {name}: {balp:?}");
    }
    let atna_direct = tree("atna", Some("1"));
    let balp_direct = tree("balp", Some("1"));
    for name in ATNA_DIRECT {
        assert!(atna_direct.contains(*name), "atna names {name}");
        assert!(
            !balp_direct.contains(*name),
            "balp names {name}: {balp_direct:?}"
        );
    }
}

/// `recording` compiles the record bound alone and `redact` the URL helpers
/// alone, so a profile crate takes them without the audit stack: neither the
/// FHIR model and the HTTP client of `balp` nor the syslog sender of `atna`.
#[test]
fn recording_and_redact_take_no_audit_stack() {
    for (feature, takes) in [("recording", "tokio"), ("redact", "url")] {
        let closure = tree(feature, None);
        assert!(
            closure.contains(takes),
            "{feature} lost {takes}: {closure:?}"
        );
        for name in BALP_ONLY.iter().chain(ATNA_ONLY).chain(ATNA_DIRECT) {
            assert!(
                !closure.contains(*name),
                "{feature} compiles {name}: {closure:?}"
            );
        }
    }
    assert!(
        !tree("redact", None).contains("tokio"),
        "redact takes no runtime"
    );
}

/// `retrieve` compiles the ITI-81 query and response over the FHIR model
/// alone: no HTTP client, no runtime and no syslog sender, since the
/// repository serves the request with its own stack.
#[test]
fn retrieve_takes_the_fhir_model_and_no_transport() {
    let closure = tree("retrieve", None);
    assert!(
        closure.contains("fhir-types"),
        "retrieve lost fhir-types: {closure:?}"
    );
    for name in ["reqwest", "tokio"].iter().chain(ATNA_ONLY) {
        assert!(
            !closure.contains(*name),
            "retrieve compiles {name}: {closure:?}"
        );
    }
}
