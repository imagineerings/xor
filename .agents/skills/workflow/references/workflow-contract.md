# Workflow Contract

`WORKFLOW.md` contains optional YAML front matter followed by a Markdown prompt
template. This skill supports the local-task workflow: tasks come from
`.agents/specs/**/tasks.md`, and Linear is populated when a local task is
picked.

## Automatic Next Task

On normal skill invocation, execute:

```bash
node .agents/skills/workflow/scripts/workflow.js next
```

It:

1. Loads active local task packets in deterministic source order.
2. Searches Linear for machine-readable markers in existing issue descriptions:
   - `workflow.local_task_id:<task-id>`
   - `workflow.local_task_source:<task-file>:<line>`
3. Skips tasks that already have a non-terminal Linear issue.
4. Creates Linear for the first unclaimed task and renders the prompt.

If all active local tasks already have non-terminal Linear issues,
the script returns no task and includes `skipped_claimed_tasks` in JSON output
for visibility. Use `workflow.js pick <task-id>` only when a user names a
specific task; it may resume that exact task's existing Linear issue instead of
creating a new one.

Use `workflow.js next --count <n> --json` to reserve multiple unclaimed tasks
for parallel work. The agent decides whether this is appropriate by comparing
task text, requirements, write manifests, and likely code ownership. Do not run
tasks in parallel when they touch the same files, have explicit dependency
ordering, or require a shared migration/schema step.

## Linear Stage Updates

Use the executable script to move Linear issues between stages:

```bash
node .agents/skills/workflow/scripts/workflow.js move <task-id-or-linear-id> --state-name "In Progress"
```

`move` accepts a local task ID, local task source (`.agents/specs/...:line`),
Linear issue identifier, Linear issue ID, or Linear issue URL. Pass
`--state-id` when the caller already knows the target Linear state ID; otherwise
pass `--state-name` and the script resolves the state in the configured Linear
team.

## Local UI

`assets/ui/index.html` is a task board for visualizing script output. The page
does not execute repository commands and does not call Linear directly. The
launcher supplies data through `?data=/workflow-data.json`, and the UI fetches
that payload on load. The board shows local task state, Linear state, reserved
batch membership, requirements, writes manifests, and task packets.

Launch it with:

```bash
node .agents/skills/workflow/scripts/workflow.js ui
```

Launch modes:

- Ordinary `workflow.js next` and ordinary `$workflow` invocations do not launch
  the UI.
- `workflow.js ui` starts a localhost server, serves the UI with active local
  tasks, and opens the browser without claiming work by default.
- `workflow.js begin-ui` is the explicit begin-and-visualize mode. It is
  equivalent to `workflow.js ui --data next`, so it claims the next task and
  launches the populated board.
- `workflow.js ui --no-open --json` starts the server and returns the UI URL,
  data URL, server host, port, and process ID for agent runtimes that open UIs
  themselves.
- `workflow.js ui --data next --count <n>` reserves tasks through Linear before
  rendering the reserved batch. Use this only when the agent intends to claim
  work.
- `workflow.js ui --data pick --task-id <task-id>` opens the UI for one
  explicit task after creating or resuming its Linear issue.
- `workflow.js ui --static --json` resolves the bundled HTML file without
  serving workflow data.

The UI still supports manually loaded JSON created by `workflow.js list --json`
or `workflow.js next --count <n> --json`.

## Front Matter

Supported front matter is intentionally small and compatible with the Symphony
spec shape:

```yaml
---
tracker:
  kind: linear
  endpoint: https://api.linear.app/graphql
  api_key: $LINEAR_API_KEY
  team_key: ENG
  project_slug: baymax
  active_states: [Todo, In Progress]
  terminal_states: [Done, Closed, Canceled, Cancelled, Duplicate]
tasks:
  glob: .agents/specs/**/tasks.md
agent:
  max_turns: 20
---
```

The script accepts nested maps, scalar values, simple
`- value` lists, inline lists, and literal `|` block scalars. Avoid anchors,
aliases, folded block scalars, and object-valued list items.

`tracker.kind` must be `linear` when a task is picked. `tracker.api_key` may be
a literal token or `$VAR_NAME`; `$LINEAR_API_KEY` is the canonical environment
variable. Provide either `tracker.team_id` or `tracker.team_key` so the script
can create a Linear issue. `tracker.project_id` or `tracker.project_slug` is
optional.

## Local Task Packets

Each top-level checkbox in a matched `tasks.md` file is a packet:

```markdown
- [ ] 3. Wire interactive mode to shared slash command behavior
  - Add autocomplete backed by the shared command catalog
  - _Requirements: 6_
  - _writes: crates/cli/src/interactive/slash_commands.rs_
```

The packet body includes the title line and all following indented lines until
the next top-level checkbox. The parser derives:

- `issue.id`: stable hash of relative path, source line, and title
- `issue.identifier`: local source key such as
  `.agents/specs/foo/tasks.md:12`
- `issue.title`
- `issue.description`: packet body without the title line
- `issue.state`: `Todo`, `In Progress`, or `Done`
- `issue.labels`: `local-task`, `workflow`, and path-derived labels
- `issue.task_file`
- `issue.task_line`
- `issue.requirements`
- `issue.writes`
- `issue.linear`: populated after `workflow.js next` or `workflow.js pick`

## Template Body

Use strict double-brace interpolation:

```markdown
You are working on a local Baymax spec task.

Task: {{ issue.title }}
Source: {{ issue.task_file }}:{{ issue.task_line }}
Requirements: {{ issue.requirements }}
Writes: {{ issue.writes }}

Task body:
{{ issue.description }}

Linear:
{{ issue.linear.url }}
```

Supported values include:

- `issue.id`
- `issue.identifier`
- `issue.title`
- `issue.description`
- `issue.priority`
- `issue.state`
- `issue.branch_name`
- `issue.url`
- `issue.labels`
- `issue.blocked_by`
- `issue.created_at`
- `issue.updated_at`
- `issue.task_file`
- `issue.task_line`
- `issue.task_body`
- `issue.requirements`
- `issue.writes`
- `issue.linear`
- `attempt`

Unknown variables are errors. Filters such as
`{{ issue.title | default: "Untitled" }}` are not supported.

## Linear Population

`workflow.js pick <task-id>` creates or resumes a Linear issue for the selected packet. The issue
description must include:

- local task ID and source path
- task body
- requirements and writes metadata
- rendered prompt

The Linear response is attached to `issue.linear` before the prompt is returned,
so prompts can include `{{ issue.linear.identifier }}` or
`{{ issue.linear.url }}`. If Linear creation fails, the task is not considered
picked.
