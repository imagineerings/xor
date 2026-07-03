---
name: workflow
description: Automatically begin the next unclaimed repository workflow task from local `.agents/specs/**/tasks.md` files, render it through a repository-owned `WORKFLOW.md`, and populate Linear without duplicating existing work. Use when an agent needs to start the next logical local spec task; list, pick, render, or complete local tasks; validate or author WORKFLOW.md files for the local task workflow; or create the Linear issue that represents picked task work.
---

# Workflow

## Overview

Use this skill to turn local spec tasks into actionable agent work packets. The
only supported work source is `.agents/specs/**/tasks.md`; do not source work
from GitHub Projects or other issue trackers. On ordinary invocation, begin the
next unclaimed local task automatically by creating a Linear issue for it. If a
local task already has a non-terminal Linear issue, skip it so parallel agents
do not duplicate effort.

This skill follows the Symphony service spec's core shape where it applies:
`WORKFLOW.md` is the repository-owned contract, the prompt body is rendered with
strict template variables, Linear is the tracker, and tracker writes are part of
the workflow/tooling layer. Baymax's local workflow mode differs from the full
service spec by using checked-in task files as the dispatch source instead of
polling Linear for candidates.

## Workflow

1. Locate the repository root.
2. Read and validate `WORKFLOW.md`; if no explicit path is configured, use
   `WORKFLOW.md` in the repository root or current working directory.
3. Load task packets from `.agents/specs/**/tasks.md`. Treat each top-level
   Markdown checkbox task as one packet; include its indented bullet details,
   `_Requirements:` lines, and `_writes:` lines in the packet body.
4. For automatic starts, execute `node .agents/skills/workflow/scripts/workflow.js next`.
   It scans active local tasks in source order, skips tasks already represented
   by non-terminal Linear issues, creates Linear for the first unclaimed task
   when only one unclaimed task is active, and returns the rendered prompt. When
   multiple unclaimed tasks are active, evaluate the candidates before picking:
   account for priority, immediate value, dependencies, and the previous tasks
   in the same `tasks.md` file. Treat previous tasks as context for what has
   already been completed, claimed, or left incomplete; prefer the next task
   whose prerequisites are satisfied and whose implementation adds the most
   useful value now. If every active task is claimed, report that instead of
   starting work.
5. Before editing implementation files, run a quick start-gate consistency
   check for the picked task's spec directory:
   - Confirm the task is still valid to begin.
   - Check prerequisites, dependency wave placement, obvious `_writes:`
     conflicts, and obvious contradictions between `requirements.md`,
     `design.md`, and `tasks.md`.
   - If the check reveals blocking ambiguity, update the spec files or ask for
     clarification before implementation. Do not proceed on a known inconsistent
     task packet.
6. Do not launch the UI during ordinary `$workflow` invocation. Launch the UI
   only when the user explicitly asks for a visual/task board mode, such as
   `$workflow ui`, `$workflow begin-ui`, "workflow with UI", or "show workflow
   UI".
7. When independent tasks can safely run in parallel, execute
   `node .agents/skills/workflow/scripts/workflow.js next --count <n> --json`,
   create one git worktree per returned task, and run one agent per worktree
   from that task's rendered prompt. Use parallel work only when the task write
   manifests, likely files, and dependencies do not overlap in a way that would
   cause conflicting edits.
8. For explicit task IDs, execute `node .agents/skills/workflow/scripts/workflow.js pick <task-id>`;
   it resumes the existing Linear issue for that exact task when present
   instead of creating a duplicate.
9. Work from the rendered prompt in the repository checkout or task-specific
   worktree. Do not treat Linear
   as the source of truth for dispatch eligibility in this local workflow.
10. When work changes phase, update Linear with
   `node .agents/skills/workflow/scripts/workflow.js move <task-or-linear-id> --state-name <state>`.
11. When the agent determines the implementation is complete and validation is
    passing, run the full completion-gate consistency pass before marking the
    task complete:
    - Tighten start, validation, handoff, and completion gates based on the
      actual validation performed.
    - Update dependency waves for remaining work when ordering, prerequisites,
      or parallel safety changed.
    - Reconcile `requirements.md`, `design.md`, and `tasks.md` so requirement
      references, design properties, task reads/writes, and done conditions
      match the delivered behavior.
    Then execute
    `node .agents/skills/workflow/scripts/workflow.js complete <task-id>`.
    This moves the linked Linear issue to `Done` by default and updates the
    local task checkbox to `[x]`. Pass `--state-name <state>` when the
    repository uses a non-`Done` terminal or review state.

## Executable Script

Use `scripts/workflow.js` directly; no MCP server is required.

```bash
node .agents/skills/workflow/scripts/workflow.js next
```

Commands:

- `next` — automatically pick the next unclaimed active task when only one is
  active; when multiple tasks are active, return candidates for value-based
  selection that considers previous tasks and dependencies. Use `--count <n>`
  to reserve multiple independent tasks for parallel worktrees.
- `list` — list task packets from local task files.
- `render <task-id>` — render a selected task through `WORKFLOW.md`.
- `pick <task-id>` — render a selected task and create or resume Linear.
- `move <task-or-linear-id> --state-name <state>` — move a Linear issue from
  one workflow stage to another.
- `complete <task-id> --state-name <state>` — after the agent verifies the task
  works correctly, move the linked Linear issue and mark the local task
  checkbox complete. Defaults to `--state-name Done`.
- `ui` — opt-in visual mode; launch the local workflow task board with active
  task data, without claiming work by default.
- `begin-ui` — opt-in begin-and-visualize mode; begin the next task and launch
  the UI with the claimed task payload.
- `validate` — load and validate the `WORKFLOW.md` contract.

The script reads JSON settings from `WORKFLOW_SETTINGS`. Values in
`WORKFLOW_SETTINGS` override defaults and `WORKFLOW.md` front matter. Use these
fields:

- `repository_path`: repository root for relative paths.
- `workflow_path`: workflow file path; defaults to `WORKFLOW.md`.
- `tasks_glob`: local task source glob; defaults to
  `.agents/specs/**/tasks.md`.
- `linear_api_key`: Linear token literal or `$LINEAR_API_KEY` reference.
- `linear_endpoint`: Linear GraphQL endpoint; defaults to
  `https://api.linear.app/graphql`.
- `linear_team_id` or `linear_team_key`: required to create Linear issues.
- `linear_project_id` or `linear_project_slug`: optional Linear project routing.
- `linear_label_ids`: optional list of Linear label IDs to attach.
- `linear_state_id`: optional state ID for the created issue.
- `resume_existing`: optional boolean for explicit picks; defaults to `true`.

## Parallel Worktrees

The agent may decide to execute work in parallel when tasks are independent.
Prefer parallel work when returned tasks write different files or crates, have
no dependency ordering in their task text, and can be validated separately. Use
sequential work when tasks touch the same files, have explicit ordering, share a
migration/schema boundary, or require the output of an earlier task.

For parallel execution:

1. Reserve tasks with `workflow.js next --count <n> --json`.
2. Create one git worktree and branch for each returned task.
3. Start one agent per worktree using that task's rendered prompt.
4. Move each Linear issue to the appropriate in-progress/review/done stage as
   work begins, is handed off, and completes.

## Local UI

Use `node .agents/skills/workflow/scripts/workflow.js ui` to launch the local
task board with a populated task payload. The command starts a localhost server,
serves `assets/ui/index.html`, passes `?data=/workflow-data.json`, and opens the
browser unless `--no-open` is passed.

```bash
node .agents/skills/workflow/scripts/workflow.js ui
node .agents/skills/workflow/scripts/workflow.js begin-ui
node .agents/skills/workflow/scripts/workflow.js ui --no-open --json
```

UI launch is opt-in. Use these clear modes:

- `ui` or `ui --data list` loads active local tasks without Linear
  writes.
- `begin-ui` is equivalent to `ui --data next`; it reserves the next task and
  creates or resumes its Linear issue before showing the claimed task.
- `ui --data next --count <n>` reserves tasks and creates or resumes Linear
  issues before showing the reserved batch.
- `ui --data pick --task-id <task-id>` opens the UI for one explicit task after
  creating or resuming its Linear issue.
- `--host <host>` and `--port <port>` override the default localhost server
  address when an agent runtime requires a specific endpoint.
- `--static` resolves the bundled HTML file without serving workflow data.

The page still accepts pasted or loaded JSON from these commands when a local
server is not appropriate:

```bash
node .agents/skills/workflow/scripts/workflow.js list --json > /tmp/workflow-tasks.json
node .agents/skills/workflow/scripts/workflow.js next --count 4 --json > /tmp/workflow-batch.json
```

The UI groups tasks by local task state, Linear-linked state, reserved batch
state, and done state. It also shows requirements, writes manifests, task
packets, Linear URLs, and copyable stage-move and completion commands.

`WORKFLOW.md` front matter may provide equivalent values under `tracker`:

```yaml
---
tracker:
  kind: linear
  api_key: $LINEAR_API_KEY
  team_key: ENG
  project_slug: baymax
tasks:
  glob: .agents/specs/**/tasks.md
---
```

## Prompt Rendering Rules

- Treat unknown variables and unknown filters as errors.
- Support dot paths such as `{{ issue.title }}`,
  `{{ issue.description }}`, `{{ issue.labels }}`, `{{ issue.task_file }}`,
  `{{ issue.task_line }}`, `{{ issue.requirements }}`, `{{ issue.writes }}`,
  `{{ issue.linear.url }}`, and `{{ attempt }}`.
- Render arrays and objects as pretty JSON.
- If the workflow prompt body is empty, use a minimal local-task prompt. Do not
  silently fall back when `WORKFLOW.md` is missing or has invalid front matter.

Read `references/workflow-contract.md` before editing `WORKFLOW.md`, changing
the script, or explaining the supported task and Linear fields.
