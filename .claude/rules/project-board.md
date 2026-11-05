<!-- SPDX-FileCopyrightText: Cadasto B.V. -->
<!-- SPDX-License-Identifier: BUSL-1.1 -->

# No roadmap board

FerroAUDIT has no GitHub Project (v2) board, and this repository creates
none: a board is the owner's to create. The FerroHEALTH products each keep a
public roadmap board; this library repository keeps its plan on the tracker
alone, as it has since the repository was set up (#3).

- **The tracker is the issue list** (`issue-workflow.md`): the native issue
  type and the organisation's `Priority` and `Effort` fields carry type,
  priority and effort; labels carry work kind and domain; native edges carry
  decomposition and sequencing (`issue-relationships.md`).
- **Milestones are the release spine and the public view.** The milestone
  page shows what a release holds and what is done; `release.yml` refuses a
  tag whose milestone holds an open item and closes it after the release.
- **No status to move.** There is no `Todo` / `In Progress` / `Done` column
  to keep true: an issue is open or closed, and a pull request that names it
  with `Closes #N` closes it on merge.

No step of the issue loop calls `scripts/gh/project.sh`: pickup and close
move nothing (`issue-workflow.md`, the `next-task` and `phase-done`
skills). The helper stays as a stub for the shared habit of the products,
whose loop runs `project.sh status <n> in-progress` at pickup: every
command prints that FerroAUDIT has no roadmap board, so nothing was moved,
and exits 0 without calling GitHub. That answer is the expected one, never
a defect to report.

Never create a "FerroAUDIT Roadmap" project, never add this repository's
issues to another product's board, and never extend the stub into a board
client. Should the owner ever want a board, that decision rewrites this
file and the helper first.
