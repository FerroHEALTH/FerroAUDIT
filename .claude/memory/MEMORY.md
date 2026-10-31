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
  only signed commits through pull requests and the merge queue
- [Auto-merge](pr-auto-merge.md): arm `gh pr merge <n> --auto` the moment a
  pull request opens; no `--squash`, no `--delete-branch`
- [Release tag](release-tag.md): the tag is signed on the merge commit and the
  crates-io environment waits for the owner's approval
- [Release closes milestone](release-closes-milestone.md): release.yml refuses
  a tag whose milestone is missing or not empty and closes it after every
  leg; check it after a manual or recovery release
- [Worker scratch subfolders](worker-scratch-subfolders.md): scratch files
  under `<scratchpad>/<task>/`, never the root; re-read every PR body
- [Remove worktrees after merge](remove-worktrees-after-merge.md): `cargo
  clean`, remove the worktree and its branch at every merge
- [Wait loops](worker-wait-loops.md): wait on notifications, poll at most
  every 60 seconds, never a self-matching `pgrep`
