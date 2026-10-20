---
name: remove-worktrees-after-merge
description: Remove every worktree, its branches and its build folder as soon as its PR is merged, in every family repo; never leave one hanging
metadata:
  type: feedback
---

<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

Remove a worktree under `.claude/worktrees/` and its local branches as soon
as the pull request carrying its work is merged, and run `git fetch
--prune`. A worktree whose work was discarded or superseded goes too. Check
`git worktree list` at every merge notice.

**Why:** the owner found leftover worktrees and asked (2026-10-15) that they
"never hang there", and found the disk at 1.1 TB used on 2026-10-19 and asked
to "keep cleaning the cargo and things".

**How to apply:** on each merge: run `cargo clean` in the worktree so its
`target/` goes, `git worktree remove .claude/worktrees/<name>`, `git branch
-D <branch>`, `git fetch --prune`, and `git pull --ff-only` in the main
checkout. Before removing, check the worktree has no uncommitted or unpushed
work and that its PR is merged. Every checkout goes under
`.claude/worktrees/<name>` (`git worktree add .claude/worktrees/<name> -b
<branch> origin/main`), never a sibling folder beside the project. Run
`cargo clean` in the main checkout too once no build uses it.
