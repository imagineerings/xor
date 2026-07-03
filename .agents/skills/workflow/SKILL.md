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
   by non-terminal Linear issues, creates Linear for the first unclaimed task,
   and returns the rendered prompt. If every active task is claimed, report that
   instead of starting work.
5. When independent tasks can safely run in parallel, execute
   `node .agents/skills/workflow/scripts/workflow.js next --count <n> --json`,
   create one git worktree per returned task, and run one agent per worktree
   from that task's rendered prompt. Use parallel work only when the task write
   manifests, likely files, and dependencies do not overlap in a way that would
   cause conflicting edits.
6. For explicit task IDs, execute `node .agents/skills/workflow/scripts/workflow.js pick <task-id>`;
   it resumes the existing Linear issue for that exact task when present
   instead of creating a duplicate.
7. Work from the rendered prompt in the repository checkout or task-specific
   worktree. Do not treat Linear
   as the source of truth for dispatch eligibility in this local workflow.
8. When work changes phase, update Linear with
   `node .agents/skills/workflow/scripts/workflow.js move <task-or-linear-id> --state-name <state>`.
   Also update the local task checkbox when the implementation is complete and
   the workflow asks for that status change.

## Executable Script

Use `scripts/workflow.js` directly; no MCP server is required.

```bash
node .agents/skills/workflow/scripts/workflow.js next
```

Commands:

- `next` — automatically pick the next unclaimed active task, skipping tasks
  that already have non-terminal Linear issues. Use `--count <n>` to reserve
  multiple independent tasks for parallel worktrees.
- `list` — list task packets from local task files.
- `render <task-id>` — render a selected task through `WORKFLOW.md`.
- `pick <task-id>` — render a selected task and create or resume Linear.
- `move <task-or-linear-id> --state-name <state>` — move a Linear issue from
  one workflow stage to another.
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

Use `assets/ui/index.html` to visualize task state locally. The page is static
and does not call Linear or read the repository by itself. Export workflow JSON,
open the page, and load or paste the JSON:

```bash
node .agents/skills/workflow/scripts/workflow.js list --json > /tmp/workflow-tasks.json
node .agents/skills/workflow/scripts/workflow.js next --count 4 --json > /tmp/workflow-batch.json
```

The UI groups tasks by local task state, Linear-linked state, reserved batch
state, and done state. It also shows requirements, writes manifests, task
packets, Linear URLs, and copyable stage-move commands.

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
