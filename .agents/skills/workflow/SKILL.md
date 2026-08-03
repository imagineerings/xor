---
name: workflow
description: Plan, claim, execute, hand off, and complete repository tasks from `.agents/specs/**/tasks.md` using a repository-owned `WORKFLOW.md` and Linear synchronization. Use when Codex needs to choose the next local spec task, inspect task readiness or conflicts, reserve independent work, manage task leases, render work prompts, reconcile interrupted updates, operate the workflow UI, or maintain the local workflow contract.
---

# Workflow

Turn checked-in spec tasks into safe, traceable work packets. Treat local
`tasks.md` files as dispatch authority and Linear as the shared claim and status
tracker.

Use the runtime-discovering wrapper:

```bash
.agents/skills/workflow/scripts/workflow <command>
```

Read [references/workflow-contract.md](references/workflow-contract.md) before
changing task metadata, `WORKFLOW.md`, the script, or Linear fields.

## Route by intent

Skill invocation alone does not authorize a claim or tracker write.

- For inspection, status, recommendations, or “what is next?”, run `workflow
  plan --json`, `workflow list`, or `workflow check`. Stop after reporting.
- When the user explicitly asks to start, claim, or execute work, inspect
  `workflow plan --json`, then run `workflow next` or `workflow pick <task-id>`.
- For completion, use the two-phase `finish` then `close` lifecycle below.

Run `workflow doctor` when setup may be incomplete. It reports separate
planning, claiming, and completion capabilities. Planning is read-only and
explains rank, blockers, dependency readiness, and manifest conflicts.

`next` and `pick` run the start gate, check Linear-backed conflicts, create or
resume the Linear issue, write an expiring lease, and return a self-contained
prompt. Move the issue to the appropriate Linear state as implementation changes
phase.

Do not launch the UI unless the user requests visual mode. Use `workflow ui`
for read-only visualization or `workflow begin-ui` to claim and visualize.

## Task selection

Prefer explicit, durable task metadata:

```markdown
- [ ] Implement cursor animation
  - _id: editor-smooth-cursor_
  - _priority: P1_
  - _value: high_
  - _wave: 2_
  - _blocked_by: editor-cursor-settings_
  - _reads: crates/editor/src/blink.rs_
  - _writes: crates/editor/src/editor.rs_
  - _validation: cargo test -p editor cursor_blink_
  - _Requirements: 7.2, 7.4_
```

Tasks without `_id` remain supported through a semantic fallback ID and a
legacy line-based alias. Use explicit IDs for new or edited task packets.

When reserving multiple tasks, first inspect `workflow plan --count <n>`. Then
use `workflow next --count <n> --json`; the script excludes blocked tasks and
tasks whose dependencies or read/write manifests conflict. Claim only when one
ready executor is available per task. Hand off every prompt immediately and
release any batch item that does not start.

Earlier unfinished packets add an ordering penalty but are not hard blockers.
Use `_blocked_by` whenever ordering is required.

## Claims and handoff

Claims are leases, not permanent locks. The default lease is 120 minutes.

- `workflow renew <id> --lease-minutes <n>` renews ownership.
- `workflow release <id> --summary <text>` makes work resumable.
- `workflow takeover <id> --owner <name> --override-reason <reason>` explicitly
  replaces an active claim without bypassing consistency gates.

Use takeover only with clear coordination. Linear activity markers are the
cross-machine source of truth; `.agents/workflow-state.json` is a local cache.
Renew before the printed expiry. If implementation stops or cannot continue,
release the task with a concise summary instead of abandoning the lease.

## Gates and completion

The script runs the start gate before a claim. Run it directly when diagnosing:

```bash
.agents/skills/workflow/scripts/workflow check <id> --phase start
```

Before merge:

1. Validate the implementation.
2. Reconcile `requirements.md`, `design.md`, and `tasks.md` with delivered
   behavior.
3. Run `workflow check <id> --phase complete --validation-evidence <summary>`.
4. Run `workflow finish <id> --validation-evidence <summary>`. This marks the
   checked-in task packet complete so the change lands in the implementation PR.

After merge, fetch and switch to a clean checkout whose `HEAD` exactly matches
`origin/main`, then run `workflow close <id>`. Close moves Linear to Done and
releases the lease without editing repository files. `workflow complete` is a
compatibility alias for `finish`.

`_validation` is the expected plan, not proof. `finish` requires concise
validation evidence. Narrow overrides require both the relevant override flag
and `--override-reason`; takeover never bypasses a gate. Close operations are
journaled so interrupted Linear updates can be inspected or repaired with
`workflow reconcile --dry-run` before repair.

## Setup and maintenance

- `workflow init` creates a minimal `WORKFLOW.md` when missing.
- `workflow doctor` checks Node, contract parsing, tasks, IDs, Linear settings,
  and Git availability without calling Linear.
- `workflow doctor --online` additionally verifies Linear credentials and
  routing without writing tracker state.
- `workflow lint --strict` audits task metadata and reports coverage.
- `workflow migrate-ids --json` suggests durable IDs without editing task files;
  apply them before first claim or coordinate preservation of existing markers.
- `workflow validate` validates `WORKFLOW.md`.
- `workflow --self-test` runs the bundled test suite with runtime discovery.
- `workflow reconcile --dry-run` reports interrupted operations before repair.

Do not manually edit `.agents/workflow-state.json` or
`.agents/workflow-operations.json` during ordinary work. Store concise,
reviewable evidence only; never store secrets or private reasoning.
