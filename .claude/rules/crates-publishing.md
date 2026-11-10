<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Published crate discipline (crates.io)

`crates/ihe-atna` and `crates/ehds-logging` are published on crates.io under
their directory names so the FerroHEALTH products can depend on them. Both
are BUSL-1.1, and `ehds-logging` pins `ihe-atna` exactly at the workspace
version.
Published versions are immutable, so version hygiene is a hard rule.

## One version line

The workspace `version` in the root `Cargo.toml` is both crates' version and
the release tag `vX.Y.Z`. Each crate inherits it (`version.workspace =
true`), so the tag, the crate and the changelog section always agree.

## The bump rule

- **The crates follow the Cargo SemVer rules and no rule of their own**
  (owner decision 2026-11-10, #204). The Cargo Book, "SemVer
  Compatibility" (<https://doc.rust-lang.org/cargo/reference/semver.html>),
  is the one authority: a 0.y.z release "can treat changes in "y" as a
  major release, and "z" as a minor release", so a breaking change moves
  the minor and a compatible change the patch.
- **A breaking change moves the minor in the pull request that breaks.**
  That pull request bumps the workspace version, the changelog section and
  the lock together (`cargo update -w`), so it is the release change of the
  next minor. A compatible change leaves the version alone, and the release
  pull request moves the patch with the section and the lock.
- **A public dependency moved to another compatibility line is a breaking
  change.** The Cargo Book chapter has no entry for it. Its intro counts as
  breaking the "changes that will cause cargo and rustc to fail to build
  something that previously worked", and a type of the dependency in the
  crate's API becomes another type on the new line. The
  `cargo-semver-checks` README (v0.51.0) lists "breaking type changes, for
  example in the type of a field or function parameter" among what the
  judge does not catch. For a 0.x dependency the incompatible step is the
  left-most non-zero component (`ferrohealth-smart` 0.16 to 0.17,
  `openehr-base` 0.0.109 to 0.0.110). A public dependency is any normal
  dependency of the `openehr-*`, `fhir-types`, `fhir-search` and
  `ferrohealth-*` families or a sibling of this workspace
  (`scripts/checks/lib/public-deps.sh`), optional ones included. The
  reference is FerroHEALTH/FerroBRIDGE's rule of the same name.
- A **product** pins the crate exactly (`=X.Y.Z`) and moves its pin
  deliberately, in lockstep with the other products. A **shared crate** that
  builds on it takes a caret requirement on the newest release it needs
  (`ferrohealth-smart = "0.7.4"`), which Cargo reads as compatible across the patches of that
  `0.x` line (<https://doc.rust-lang.org/cargo/reference/semver.html>), so a
  patch reaches every product through its lockfile without a release of
  every shared crate in between (owner decision, 2026-10-26). That is safe
  only because a patch release never breaks the public API.
- **The FerroBRIDGE crates** (`fhir-types`, `fhir-search` and the rest of its
  published line) are required by caret ranges on the newest release this
  workspace needs, never with `=`. FerroBRIDGE's crates follow Cargo SemVer
  (FerroHEALTH/FerroBRIDGE#1745), and an exact requirement kept two family
  crates pinned to different patches of one of them from building together
  (FerroHEALTH/FerroAUDIT#185).
- A pull request that breaks a public API says so in its body and its
  CHANGELOG entry, naming what a product changes; while the line is `0.x`,
  such a release moves the minor version.
- `ehds-logging` keeps its exact requirement on `ihe-atna`: both release
  from this one workspace, together.

## Every change is checked for compatibility

A caret on these crates, which other family shared crates take, is only
safe while a patch never breaks the API. `cargo-semver-checks`
(<https://github.com/obi1kenobi/cargo-semver-checks>) holds that by machine:
it builds the rustdoc of each crate and of its newest version on crates.io
and refuses a break the manifest version does not allow. An unchanged
version is judged as a patch. Each crate is judged on the judge's default
feature set and on `--only-explicit-features`, what a product built with
`default-features = false` compiles against, since the README says the
judge does not catch "breaking changes that exist when only a subset of all
crate features are activated". `scripts/checks/semver.sh` runs it over the
workspace at the version pinned in `docs/VERSIONS.md`, compares each
crate's public dependency lines with its newest release's, and names a
judge that could not build apart from a judged break:

- on every pull request and in the merge queue (the `cargo-semver-checks`
  job of `ci.yml`);
- in the release plan (`scripts/release/plan.sh`, which
  `scripts/release/preflight.sh` and the `plan` job of `release.yml` both
  run) at every tag.

`scripts/checks/crate-version-guard.sh` refuses, on every pull request, a
change that moves a public dependency to another line without moving the
version to a later minor at patch 0.

A failure is answered with the minor bump and a changelog entry naming what
a product changes, or by keeping the API; never by skipping the check.

## The publish lane is resumable and verified

`scripts/release/publish-crates.sh` (`publish` / `verify` / `version`)
uploads the crate, counts "already exists" as done, and reads the registry
back before reporting success. Two lanes call it: the `crates` leg of
`release.yml` on a `v*` tag (the primary path, paused by the `crates-io`
environment's required reviewer) and `publish-crates.yml` on a manual
dispatch (a dry run by default; `publish = true` is the recovery path). Both
authenticate with crates.io Trusted Publishing (OIDC through
`rust-lang/crates-io-auth-action`); no long-lived crates.io token exists in
the repository. Neither lane restores a build cache (`ci-cd.md`).

`cargo publish --workspace --dry-run --locked` runs on every pull request as
the `publish-dry-run` job, `ehds-logging` packaged against the `ihe-atna`
beside it.

## Owner steps (done once)

1. `ihe-atna` was reserved at `0.0.0` on 2026-10-20, since a first version
   cannot use Trusted Publishing; `ehds-logging` was reserved at `0.0.0` on
   2026-10-06 from FerroFED, whose 0.0.x line never published past it.
2. On crates.io, each crate's Trusted Publishing carries two GitHub entries:
   repository owner `FerroHEALTH`, repository `FerroAUDIT`, workflows
   `release.yml` and `publish-crates.yml`, environment `crates-io`.
3. The `crates-io` GitHub environment carries the owner as required reviewer
   and admits `main` and `v*` tags.

## Official documentation (durable citations)

- Trusted Publishing on crates.io: <https://crates.io/docs/trusted-publishing>
- `cargo publish`: <https://doc.rust-lang.org/cargo/commands/cargo-publish.html>
- Caret requirements: <https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#caret-requirements>
- SemVer compatibility: <https://doc.rust-lang.org/cargo/reference/semver.html>
- cargo-semver-checks README, v0.51.0: <https://github.com/obi1kenobi/cargo-semver-checks/blob/v0.51.0/README.md>
