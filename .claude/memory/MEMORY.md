<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# Memory index

- [Family architecture](family-architecture.md): the FerroHEALTH family
  is five separate servers that reach each other only over published
  protocols, plus shared crates for cross-cutting work; nothing is
  hand-written that a shared crate or another product provides; owner
  2026-10-25
- [Memory lives in the repo](memory-lives-in-repo.md): every learning is a
  tracked file here, never a per-user note
- [What FerroAUDIT is](ferroaudit.md): the shared audit library, `ihe-atna`
  (the whole audit trail: the ATNA ITI-20 sender, BALP, forwarding and
  retrieving) and `ehds-logging` (the EHDS access record); every product
  audits through it; owner 2026-10-20 and 2026-10-25
- [Crate name and licence](crate-name.md): the crate is `ihe-atna`, named
  for its specification, under BUSL-1.1; Trusted Publishing from 0.1.0
- [Licence: BUSL 1.1](license-busl.md): the FerroHEALTH terms, Cadasto B.V.
  the Licensor, Apache 2.0 four years after each version
- [Products pin exactly](lockstep.md): every product pins `ihe-atna`
  exactly; a release rolls out as one pull request per product
- [Support policy](support-newest-release-only.md): only the newest release
  is supported; the CRA five years is a commitment to keep publishing fixes
- [No AI attribution](no-ai-attribution.md): never, anywhere, in any form
- [Signed commits and the merge queue](merge-queue-signed.md): `main` takes
  only signed commits through pull requests and the merge queue; no
  commit-message line starts with "-" or a git header word (2026-11-09)
- [Auto-merge](pr-auto-merge.md): arm `gh pr merge <n> --auto` the moment a
  pull request opens; no `--squash`, no `--delete-branch`
- [Release tag](release-tag.md): the tag is signed on the merge commit and pushed,
  and the session approves the crates-io gate itself (owner 2026-11-03)
- [Release closes milestone](release-closes-milestone.md): release.yml refuses
  a tag whose milestone is missing or not empty and closes it after every
  leg; check it after a manual or recovery release
- [Worker scratch subfolders](worker-scratch-subfolders.md): scratch files
  under `<scratchpad>/<task>/`, never the root; re-read every PR body
- [Remove worktrees after merge](remove-worktrees-after-merge.md): `cargo
  clean`, remove the worktree and its branch at every merge
- [Wait loops](worker-wait-loops.md): wait on notifications, poll at most
  every 60 seconds, never a self-matching `pgrep`

## Family rules (carried in every FerroHEALTH repository, 2026-11-09)

- [No compatibility shims](no-compat-shims.md): refactors break APIs freely and leave no deprecated item, alias, re-export or stub; a removal refusal lives ONE release, then is deleted fully; no legacy support ever; owner 2026-11-01, 2026-11-09
- [No half solutions](no-half-solutions.md): build what the issue asks the proper way and validate the need against its consumer first; a breaking change moves the minor version, never shape a design to dodge semver; owner 2026-11-08
- [EHDS above openEHR](ehds-above-openehr.md): where Regulation (EU) 2025/327 conflicts with a standard's text the Regulation wins; never revert EHDS behaviour to pass a conformance row; owner 2026-11-09
- [Workers on Opus only](workers-on-opus-only.md): every subagent is model opus; never Fable, Sonnet or Haiku as a worker; owner 2026-10-30
- [Ten worker slots](worker-cap-ten.md): up to 10 implementation workers at once across the family since 2026-11-03, replacing the cap of 5
- [No workflows for workers](no-workflows-for-workers.md): start workers with the Agent tool only, since a workflow agent cannot be messaged; owner 2026-11-01
- [Workers never idle on the merge queue](workers-never-idle-on-merge-queue.md): start the next independent issue in a fresh worktree while a PR waits in the queue; owner 2026-11-03
- [Workers own every open PR](workers-own-every-open-pr.md): drive every PR you opened to merge; the orchestrator sweeps every open PR after each merge, red main first; owner 2026-11-01, 2026-11-03
- [PR backlog discipline](pr-backlog-discipline.md): several open PRs are fine if each merges cleanly; disjoint files, no stacked PRs, every PR tracked to its merge; owner 2026-10-17
- [Worktrees in the repo](worktrees-in-repo.md): every worktree at <repo>/.claude/worktrees/<name>, never a sibling folder or the scratchpad; owner 2026-10-21
- [Admin bypass stays](admin-bypass-stays.md): the org-admin bypass on main stays in every family repo; nobody merges through it; owner 2026-10-21
- [Decide, do not ask](decide-dont-ask.md): make tracker, design and release calls yourself and act; ask the owner only when a tool is refused; owner 2026-11-03
- [Autonomous loop](autonomous-loop.md): never stop and never wait idle; keep working the issues while PRs drain; owner 2026-10-17
- [Always a next milestone](next-milestone-always.md): every repo keeps a milestone with a due date after the active one; owner 2026-11-03
- [One milestone at a time](one-milestone-at-a-time.md): every worker on the current milestone until it is cut; en-route issues join it and are fixed first; owner 2026-10-19
- [Shared-crate blockers](shared-crate-blockers.md): every filed issue gets a milestone; a shared-crate defect goes in the owner repo's current milestone with a worker at once; no consumer workaround; owner 2026-11-07
- [Family toolchain](family-toolchain.md): newest stable Rust as toolchain, the toolchain minus one as MSRV; never lower an MSRV; owner 2026-10-21
- [Subagent reports go to a file](subagent-reports-to-file.md): a long agent report is written to the agent's scratch subfolder and read from there; 2026-09-12
- [Sibling projects](sibling-projects.md): the other family repositories are read-only prior art from here; a change there is made in its own checkout and PR
