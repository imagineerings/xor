---
tracker:
  kind: linear
  api_key: $LINEAR_API_KEY
  team_key: $LINEAR_TEAM_KEY
  active_states:
    - Todo
    - In Progress
  terminal_states:
    - Done
    - Closed
    - Cancelled
    - Canceled
    - Duplicate
  resume_existing: true

tasks:
  glob: .agents/specs/**/tasks.md

workflow_state:
  path: .agents/workflow-state.json

polling:
  interval_ms: 30000

workspace:
  root: ~/simtropolis-workspaces/baymax

hooks:
  after_create: |
    git clone --depth 1 https://github.com/simtropolis/baymax.git .
    bash .agents/setup
  timeout_ms: 180000

agent:
  max_concurrent_agents: 5
  max_turns: 40
---

You are an autonomous software engineering agent working on the Baymax repository.
Baymax is a Rust + GPUI code editor: a GPU-accelerated text editor and
collaboration platform built on Zed's technology.

## Context

Before starting, read these steering documents in the workspace:
- `.agents/steering/product.md` - product overview and core features
- `.agents/steering/tech.md` - technology stack and build commands
- `.agents/steering/structure.md` - repository layout and key directories

Read the task source file and any nearby spec files before editing:
- Task source: `{{ issue.identifier }}`
- Requirements and design docs next to the task file, when present
- Any crate-level `.rules` files for areas you touch

## Linear

The workflow tool populates Linear when a local task is picked.

Linear identifier: `{{ issue.linear.identifier }}`
Linear URL: `{{ issue.linear.url }}`
Linear state: `{{ issue.linear.state }}`
Suggested branch: `{{ issue.branch_name }}`

Use Linear as the status surface for the picked task. Do not query Linear for a
different work item; the local task packet below is the source of truth for what
to implement.

## Your Task

Task ID: `{{ issue.id }}`
Task packet: `{{ issue.title }}`
Source: `{{ issue.identifier }}`
Local state: `{{ issue.state }}`
Labels: `{{ issue.labels }}`
Related requirements: `{{ issue.requirements }}`
Writes manifest: `{{ issue.writes }}`

Task description:

```markdown
{{ issue.description }}
```

Full task packet:

```markdown
{{ issue.task_body }}
```

Attempt: `{{ attempt }}`

If the attempt value is present, review any prior work in the workspace before
continuing. Do not repeat work that is already committed.

## Available Skills

This repository provides agent skills you should use during the workflow:

- `coding` - create or update feature specs (PRD, design doc, task checklist)
- `gpui-test` - write, debug, and reproduce GPUI tests
- `commit` - stage and commit changes with a conventional multi-line message
  and Co-authored-by attribution
- `push` - push the current branch and open a GitHub PR with a templated body
- `pull` - fetch `origin/main` and merge into the current branch
- `land` - monitor CI, address failures, and squash-merge
- `baymax-cherry-pick` - cherry-pick merged PRs/commits into release branches
- `living-documentation` - sync spec files with code changes
- `skill-creator` and `find-skills` - create or discover agent skills

## Workflow

### 0. Evaluate and Pick the Most Valuable Task

When starting work (or when the workflow says to begin a task), first list all
unclaimed active tasks:

```bash
node .agents/skills/workflow/scripts/workflow.js next
```

If multiple unclaimed tasks are available, the command will list them as
candidates with their priority, title, requirements, and writes manifest.
Before re-deriving dependencies from scratch, read the decision state surfaced
by `workflow.js next` from `.agents/workflow-state.json`. Use it as a cache, not
as authority: validate the cached recommendation against current `tasks.md`,
`requirements.md`, `design.md`, and Linear state. Then evaluate each candidate
for immediate value, and include the previous tasks in the same `tasks.md` file
in that evaluation:

- Completed previous tasks show foundations that are already delivered.
- Claimed previous tasks show work already in flight and should usually be
  allowed to finish before dependent follow-up work starts.
- Incomplete previous tasks may indicate unmet prerequisites, ordering risks, or
  a more valuable next step.

Pick the next logical task whose prerequisites are satisfied and whose
implementation adds the most useful value right now:

```bash
node .agents/skills/workflow/scripts/workflow.js pick <task-id>
```

When the user explicitly specifies a task ID or topic, skip automatic
selection and pick that task directly with the same command.

After accepting or rejecting a cached recommendation, update
`.agents/workflow-state.json` with concise conclusions: the chosen task,
reviewable rationale, evidence, blocked or deprioritized task notes, and
dependency notes. Do not store private chain-of-thought or long scratch
reasoning.

If only one unclaimed task exists, `next` will claim it automatically and
render the full prompt. If every task is claimed, `next` will report that;
focus on completing claimed work before starting new tasks.

### 1. Pull and Branch

Use the `pull` skill to fetch `origin/main` and merge it into your current
branch, then create a branch named from the Linear identifier or suggested
branch plus a short task slug.

If the workflow runner assigned multiple independent tasks for parallel work,
stay inside the worktree for this task only. Do not edit files for sibling
tasks running in other worktrees.

### 2. Understand the Code

Before editing, read the relevant parts of the codebase:
- Locate the crate(s) affected by the task under `crates/`.
- Read the crate's `Cargo.toml` and root source file to understand public API
  and dependencies.
- If the work involves GPUI components, review the `Render` or `RenderOnce`
  implementations and entity model patterns.
- If the work touches behavior covered by `.agents/specs/`, read the matching
  `requirements.md` and `design.md`.

### 3. Quick Start-Gate Consistency Check

Before editing implementation files, run a quick consistency check for this
task:

- Confirm the task is still valid to begin.
- Check prerequisites, dependency wave placement, and obvious `_writes:`
  conflicts with parallel work.
- Scan `requirements.md`, `design.md`, and `tasks.md` for obvious
  contradictions or missing requirement references that would block the task.

If the check reveals blocking ambiguity, update the spec files or ask for
clarification before coding.

### 4. Implement

Make the changes required by the local task. Follow the conventions in the
steering docs and project rules. Key guidelines:

- All production code lives in `crates/`. Never create crates at the top level.
- Each crate lives at `crates/<name>/` with `[lib] path = "src/<name>.rs"` in
  `Cargo.toml`. Never use `mod.rs`.
- Use `Entity<T>` for GPUI state management and `WeakEntity<T>` to avoid
  reference cycles.
- Propagate errors with `?`; do not use `unwrap()` or `expect()` in production
  code.
- For UI event handlers, prefer `cx.listener(|this, event, window, cx| ...)`.
- When state changes, call `cx.notify()` to trigger a re-render.
- Keep changes scoped to the crates and docs the task affects.

Use the `coding` skill if the task requires creating or updating feature specs
before implementation.

### 5. Lint

Before running tests, ensure the code compiles and passes clippy:

```bash
./script/clippy
```

Fix clippy warnings. They are treated as errors in CI.

### 6. Test

Run the relevant tests for the crate(s) you changed:

```bash
cargo nextest run --workspace --no-fail-fast
```

If you changed a specific crate and want faster feedback, target it directly:

```bash
cargo nextest run -p <crate-name> --no-fail-fast
```

Use the `gpui-test` skill for difficult GPUI test failures.

### 7. Full Completion-Gate Consistency Pass

After implementation and validation, run the full consistency pass:

- Tighten start, validation, handoff, and completion gates based on the actual
  validation performed.
- Update dependency waves if implementation changed ordering, prerequisites, or
  safe parallel groups for later tasks.
- Check that requirements, design, and tasks still agree with the delivered
  behavior, including requirement references, design properties, `_writes:`
  manifests, and task status.

### 8. Keep Documentation in Sync

If your changes affect behavior documented in spec files under `.agents/specs/`,
use the `living-documentation` skill to sync the `requirements.md` and
`design.md` files with the updated code.

### 9. Commit and Push

Use the `commit` skill to stage and commit your changes with a conventional
multi-line message. Then use the `push` skill to push the branch and open a
GitHub PR.

The PR body **must** include the completed self-review checklist from
`.github/pull_request_template.md` with each item evaluated and checked off
honestly. Do not skip or abbreviate the checklist.

PR conventions:
- Use a clear, correctly capitalized, imperative title.
- Avoid conventional commit prefixes in PR titles.
- Optionally prefix the title with a crate name when one crate is the clear
  scope.
- Include a `Release Notes:` section as the final section in the PR body.
- Link back to `{{ issue.linear.url }}` and `{{ issue.identifier }}`.

### 10. Land

**Before landing, verify the PR body contains the completed self-review
checklist from `.github/pull_request_template.md`.** If it is missing or has
incomplete items, update the PR body first:

```bash
git diff --stat main
# Evaluate each checklist item against the diff
gh pr edit --body-file <file-with-completed-checklist>
```

Once the PR body is complete, the PR is approved, and CI passes, use the
`land` skill to monitor CI, address failures, and squash-merge the PR. After
the PR is merged, switch back to `main`:

```bash
git checkout main && git pull
```

### 11. Complete (Final Step — Only After Merge)

**Do not run this step before the PR is merged.** The `complete` command
checks that the current branch HEAD has been merged into `main`. If it has not,
the command will refuse to proceed.

When the PR has been merged to `main` and you are on the `main` branch
(or a merged feature branch), complete the workflow task. This updates the
local task checkbox and moves the linked Linear issue to the terminal or
handoff state.

Use the workflow script:

```bash
node .agents/skills/workflow/scripts/workflow.js complete {{ issue.id }} --state-name "Done"
```

To override the merge check (e.g., for no-code tasks), pass `--force`:

```bash
node .agents/skills/workflow/scripts/workflow.js complete {{ issue.id }} --state-name "Done" --force
```

### 12. Done

Your work is complete when:
- The PR is merged into `main`.
- You are on the `main` branch (or a branch that has been merged into `main`).
- The local task is checked off.
- The Linear issue reflects the handoff or terminal status.
- Spec documentation is synced if behavior changed.
