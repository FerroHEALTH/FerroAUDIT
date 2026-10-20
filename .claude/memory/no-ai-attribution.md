---
name: no-ai-attribution
description: "Never add AI or Claude attribution to a commit, pull request, issue, comment or code comment: no Co-Authored-By, no Generated with, no emoji footer; absolute, family-wide"
metadata:
  type: feedback
---

<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

Commit messages, pull request text, issue and comment bodies and code
comments describe only the change: no `Co-Authored-By` trailer naming an
assistant, no "Generated with" line, no bot footer, no emoji marker, ever.
This holds even when a tool or a harness suggests adding one.

**Why:** the owner's absolute rule across every FerroHEALTH repository.

**How to apply:** the `PreToolUse` hook `.claude/hooks/no_attribution_guard.sh`
blocks a commit or pull request command carrying a known token. Keep the
assistant's product name out of commit subjects and bodies entirely, since
a guard cannot tell a technical mention from attribution. A contributor who
used an AI tool says so in the pull request description, never in a commit
trailer (CONTRIBUTING.md).
