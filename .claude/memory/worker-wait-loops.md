---
name: worker-wait-loops
description: Wait on a long command through its completion notification, poll a pull request at most every 60 seconds, and never write a pgrep loop that matches its own command line
metadata:
  type: feedback
---

<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

Workers in FerroBRIDGE left shells polling for hours with `until ! pgrep -f
"<command>"`: the pattern matches the loop's own command line, so it never
ends. One loop from a finished worker ran 4.5 hours, and the owner found
"13 shells running ... holding up things".

**Why:** each stuck loop holds a shell slot and burns CPU, and duplicate
waits stack on the same output.

**How to apply:**
- Run a long command in the background and wait for its completion
  notification; at most one wait per output.
- Watch a pull request with `gh pr view <n> --json state`, polled at most
  every 60 seconds.
- Never poll with `pgrep -f` on a string the loop itself contains. Stop a
  process by its PID, never with a broad `pkill -f <script>`.
- A finished branch waits for nobody: push once its own tests pass and let
  CI run the rest.
