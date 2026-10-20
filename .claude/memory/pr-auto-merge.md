---
name: pr-auto-merge
description: "Every pull request gets auto-merge armed the moment it is opened (gh pr merge <n> --auto, which adds it to the merge queue, no --squash and no --delete-branch); family rule"
metadata:
  type: feedback
---

<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

After opening a pull request, arm auto-merge at once:

```sh
gh pr create ... && gh pr merge <n> --auto
```

`main` merges through GitHub's merge queue, which sets the merge method
itself and refuses `--squash` and `--delete-branch`. The repository deletes
a merged head branch on its own.

**Why:** the owner, on FerroBRIDGE, 2026-09-04: "for PR's do not forget to
trigger auto merge okay!! so when the CI is green it will be merged". The
`main` ruleset requires the `conclusion` check, so auto-merge is the
hand-off.

**How to apply:**
- Never leave a pull request waiting for a manual merge, and never use
  `--admin` without the owner asking.
- Confirm a merge with `gh pr view <n> --json state,mergedAt` after the
  queue reports ([[worker-wait-loops]]).
- After every merge clean up ([[remove-worktrees-after-merge]]).
- Branch from `origin/main` in a worktree, never from a local `main`, and
  stage named paths, never `git add -A` from the root.
