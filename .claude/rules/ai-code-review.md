<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Machine review (SonarQube Cloud): advisory, never authority

Every push to `main`, and a weekly schedule, is analyzed by SonarQube Cloud
(`.github/workflows/sonar.yml`; scope in `sonar-project.properties`; project `FerroHEALTH_FerroAUDIT`, organization
`ferrohealth`, the built-in "Sonar way" quality gate). It is a second opinion
beside the local gates and CodeQL, and it also reads the shell and workflow
YAML the Rust gates never see. The scan imports the lcov report of the
instrumented nextest run.

Because it gates no merge, it runs on no pull request and no merge group
(owner decision, 2026-11-25): the organisation shares 20 concurrent jobs,
and an advisory analysis must not hold a runner a gate needs. A newer push
to `main` cancels a running analysis. CodeQL, the security scanner, keeps
its pull-request run, limited to changes under Rust sources, Cargo files or
`.github/`, and runs on no merge group.

## Precedence: a finding never outranks the sources

1. The specifications vendored under `docs/specs/` (`spec-adherence.md`).
2. The hard rules: `CLAUDE.md` and the `.claude/rules/*.md` files.
3. The local gates: `cargo fmt`, `clippy`, `cargo nextest`, `cargo deny`,
   `shellcheck`, `actionlint`, `zizmor` and the guards under
   `scripts/checks/`.
4. The analyzer.

A finding that contradicts a specification citation, or asks for something
the rules forbid, is wrong by construction. Nothing it reports relaxes
`testing.md`: never weaken a test because a finding suggested it.

## Rules

- **It gates no merge.** The quality gate is informational, and the
  `conclusion` job of `ci.yml` does not wait on it.
- **Findings are fixed by hand, in a normal change**, never applied through a
  UI that would attribute a commit to a bot.
- **Automatic Analysis stays off.** CI-based analysis and Automatic Analysis
  cannot both run, and Automatic Analysis does not analyze Rust.
- **A wrong finding is data, never a silent suppression:** mark it false
  positive or accepted with its reason recorded on a tracker issue, or adjust
  `sonar-project.properties` when the scope is wrong (a vendored or generated
  tree), never to make a number go down.

## Official documentation (durable citations)

- <https://docs.sonarsource.com/sonarqube-cloud/>
- <https://docs.sonarsource.com/sonarqube-cloud/analyzing-source-code/languages/rust>
