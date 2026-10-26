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

- **A release bumps the workspace version, the changelog section and the
  lock together** (`cargo update -w`), in the release pull request.
- A **product** pins the crate exactly (`=X.Y.Z`) and moves its pin
  deliberately, in lockstep with the other products. A **shared crate** that
  builds on it takes a caret requirement on the newest release it needs
  (`ferrohealth-smart = "0.7.4"`), which Cargo reads as compatible across the patches of that
  `0.x` line (<https://doc.rust-lang.org/cargo/reference/semver.html>), so a
  patch reaches every product through its lockfile without a release of
  every shared crate in between (owner decision, 2026-10-26). That is safe
  only because a patch release never breaks the public API.
- A pull request that breaks a public API says so in its body and its
  CHANGELOG entry, naming what a product changes; while the line is `0.x`,
  such a release moves the minor version.
- `ehds-logging` keeps its exact requirement on `ihe-atna`: both release
  from this one workspace, together.

## Every release is checked for compatibility

A caret on these crates, which other family shared crates take, is only
safe while a patch never breaks the API. `cargo-semver-checks`
(<https://github.com/obi1kenobi/cargo-semver-checks>) holds that by machine:
it builds the rustdoc of each crate and of its newest version on crates.io
and refuses a break the manifest version does not allow (Cargo's SemVer
rules count `0.x.y` to `0.x.(y+1)` as compatible, so a break on a `0.x`
line moves the minor version). An unchanged version is checked as a patch.
`scripts/checks/semver.sh` runs it over the workspace at the version pinned
in `docs/VERSIONS.md`:

- on every pull request (the `cargo-semver-checks` job of `ci.yml`), so a
  breaking change carries its minor bump in the pull request that breaks;
- in the release plan (`scripts/release/plan.sh`, which
  `scripts/release/preflight.sh` and the `plan` job of `release.yml` both
  run) at every tag.

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
- cargo-semver-checks: <https://github.com/obi1kenobi/cargo-semver-checks>
