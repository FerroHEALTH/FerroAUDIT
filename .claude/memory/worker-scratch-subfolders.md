---
name: worker-scratch-subfolders
description: Each worker writes scratch files under its own subfolder of the session scratchpad, never the root, and checks a file is its own before publishing it
metadata:
  type: feedback
---

<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

Every worker in a session shares one scratchpad directory. A worker writes
its scratch files (PR bodies, issue comments, notes, reports) under its own
subfolder of that scratchpad, named for its task (`<scratchpad>/<task>/`),
and never at the scratchpad root.

**Why:** on 2026-10-20 a FerroSMART worker left `pr1.md` at the scratchpad
root, and a FerroBRIDGE worker's `gh pr edit --body-file pr1.md` published
it, so the wrong issues were named and the right one did not auto-close
(FerroHEALTH/FerroBRIDGE#1021). A generic file name at the shared root is a
collision waiting for the next worker.

**How to apply:**
- Every worker brief names the worker's own scratch subfolder and forbids
  the root.
- Before any `--body-file` or `-F` argument, check the file is one you
  wrote in this task: it lives under your subfolder and its content names
  your own issue.
- After setting a PR body, re-read it
  (`gh pr view <n> --json body,closingIssuesReferences`) and confirm the
  `Closes #N` lines name exactly your issues.
- Related: [[worker-wait-loops]], [[pr-licence-box]].
