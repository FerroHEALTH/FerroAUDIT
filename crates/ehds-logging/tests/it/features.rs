// SPDX-FileCopyrightText: Cadasto B.V.
// SPDX-License-Identifier: BUSL-1.1

//! The boundaries of the logging component, read from its own crate graph
//! with each feature on: it depends on no interoperability component and on
//! no product, so it stays independent of the interoperability component
//! (Regulation (EU) 2025/327 Art 2(2)(n), (o)). The `balp` feature adds
//! `ihe-atna`, the audit transport, and nothing of an exchange format.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The interoperability components of the family the logging component never
/// depends on.
const INTEROPERABILITY: &[&str] = &["eehrxf", "fhirconnect", "openehr-mapping-core"];

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
        .args(["--package", "ehds-logging"])
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
        names.contains("ehds-logging"),
        "cargo tree for {feature} did not list the crate itself, so its output was not read"
    );
    names
}

#[test]
fn no_feature_depends_on_an_interoperability_component_or_a_product() {
    for feature in ["", "balp"] {
        let closure = tree(feature, None);
        for name in INTEROPERABILITY {
            assert!(
                !closure.contains(*name),
                "{feature:?} compiles {name}: {closure:?}"
            );
        }
        let products: Vec<&String> = closure
            .iter()
            .filter(|name| name.starts_with("ferro"))
            .collect();
        assert!(
            products.is_empty(),
            "{feature:?} compiles a product: {products:?}"
        );
    }
}

#[test]
fn only_balp_adds_the_audit_transport() {
    assert!(
        !tree("", None).contains("ihe-atna"),
        "no feature takes ihe-atna"
    );
    assert!(
        tree("balp", Some("1")).contains("ihe-atna"),
        "balp names ihe-atna"
    );
}
