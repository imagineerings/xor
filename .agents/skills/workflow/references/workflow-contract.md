# Workflow Contract

## Contents

- [Authority and lifecycle](#authority-and-lifecycle)
- [Commands](#commands)
- [WORKFLOW.md](#workflowmd)
- [Task packet schema](#task-packet-schema)
- [Ranking and compatibility](#ranking-and-compatibility)
- [Claims and leases](#claims-and-leases)
- [Decision state](#decision-state)
- [Consistency gates](#consistency-gates)
- [Completion journal](#completion-journal)
- [Linear representation](#linear-representation)
- [Prompt variables](#prompt-variables)
- [Local UI](#local-ui)

## Authority and lifecycle

`.agents/specs/**/tasks.md` is the dispatch source of truth. Linear represents
shared ownership and workflow state but does not make a local task eligible.

The lifecycle is:

1. `plan` ranks local work without tracker writes.
2. `next` or `pick` passes the start gate and acquires a Linear-backed lease.
3. `move`, `renew`, and `release` maintain shared status.
4. `check --phase complete` verifies executed-validation evidence.
5. `finish` marks the local checkbox complete before the implementation PR is
   merged.
6. `close` verifies a clean checkout at `origin/main`, journals the operation,
   moves Linear to Done, and releases activity without editing repository files.
7. `reconcile` diagnoses or repairs an interrupted close operation.

## Commands

Use `.agents/skills/workflow/scripts/workflow` so Codex can discover its bundled
Node runtime when `node` is not on `PATH`.

| Command | Tracker writes | Purpose |
|---|---:|---|
| `init` | No | Create a minimal missing `WORKFLOW.md` |
| `doctor` | No | Diagnose runtime and configuration |
| `doctor --online` | No | Also verify Linear credentials and routing |
| `lint [--strict]` | No | Audit task metadata and coverage |
| `migrate-ids` | No | Suggest durable IDs for packets that lack them |
| `validate` | No | Validate the workflow contract |
| `list [--all]` | No | Parse active or all local task packets |
| `plan [--count n]` | No | Rank tasks and select a compatible batch |
| `render <id>` | No | Render one task prompt |
| `check <id> --phase start\|complete` | No | Run a consistency gate |
| `next [--count n]` | Yes | Claim the highest-ranked compatible ready work |
| `pick <id>` | Yes | Claim or resume an explicit task |
| `renew <id>` | Yes | Extend a claim lease |
| `release <id>` | Yes | Mark a claim inactive and resumable |
| `takeover <id>` | Yes | Explicitly replace active ownership |
| `move <id> --state-name <state>` | Yes | Move the linked Linear issue |
| `finish <id>` | No | Validate and mark the local packet complete before merge |
| `close <id>` | Yes | After merge, close Linear and release activity |
| `complete <id>` | No | Compatibility alias for `finish` |
| `reconcile [id]` | Optional | Repair a partial completion |
| `ui` | No by default | Open the local task board |
| `begin-ui` | Yes | Claim work and open the task board |

Bare invocation and `--help` display help and never claim work. Use `--json` for
machine-readable output; CLI JSON includes `schema_version: 2`. Semantic check
and doctor failures use a nonzero exit status. `reconcile --dry-run` never
repairs.

## WORKFLOW.md

`WORKFLOW.md` contains optional YAML front matter and a Markdown prompt body:

```yaml
---
tracker:
  kind: linear
  endpoint: https://api.linear.app/graphql
  api_key: $LINEAR_API_KEY
  team_key: $LINEAR_TEAM_KEY
  project_slug: sim
  active_states: [Todo, In Progress]
  terminal_states: [Done, Closed, Canceled, Cancelled, Duplicate]
  claim_lease_minutes: 120
tasks:
  glob: .agents/specs/**/tasks.md
workflow_state:
  path: .agents/workflow-state.json
workflow_journal:
  path: .agents/workflow-operations.json
---
Task: {{ issue.title }}
Task body:
{{ issue.task_body }}
```

The parser supports nested maps, scalar values, simple lists, inline lists, and
literal `|` blocks. It rejects unsupported indentation, tracker keys, unsafe
literal API keys, invalid lease values, and template variables.
Environment JSON in `WORKFLOW_SETTINGS` overrides front matter.

Tracker creation requires `team_id` or `team_key`. Project routing, label IDs,
and initial state ID are optional. The canonical token reference is
`$LINEAR_API_KEY`.

## Task packet schema

Each top-level Markdown checkbox and its indented body is one packet. Supported
metadata is case-insensitive:

| Field | Meaning |
|---|---|
| `_id:` | Durable repository-unique ID |
| `_priority:` | `P0` through `P4` |
| `_value:` | `high`, `medium`, or `low` immediate value |
| `_wave:` | Integer delivery wave |
| `_blocked_by:` | Comma-separated task IDs |
| `_reads:` | Expected read paths |
| `_writes:` | Expected write paths |
| `_validation:` | Validation command or evidence requirement |
| `_validation_evidence:` | Concise evidence recorded by `finish` for the landed packet |
| `_Requirements:` | Comma-separated requirement references |

Explicit IDs normalize to lowercase letters, digits, dots, underscores, and
hyphens and receive the `task:` prefix. Without `_id`, the fallback hashes task
file plus numbered sequence, or file plus title when no sequence exists. The
former line-and-title hash is retained as an alias for Linear lookup.

Checkbox markers map as follows: `[ ]` is `Todo`, `[~]` and `[-]` are
`In Progress`, and `[x]` is `Done`.

## Ranking and compatibility

Ranking is deterministic and exposes a numeric score and rationale. Lower
scores rank first; blocked tasks receive a large penalty and are never selected.
Ranking uses:

- readiness and explicit blockers;
- `P0`–`P4` priority;
- immediate value;
- wave;
- completion of earlier tasks in the same task file;
- a fresh cached recommendation;
- source order as the final tie-breaker.

A cached recommendation is stale when any `stale_if_changed` file has changed
after the recommendation timestamp or disappeared.

Parallel selection rejects tasks with explicit dependency relationships,
overlapping write paths, and write/read overlap. Directory and glob paths
conflict with their descendants.
`plan --count n` is the read-only preview; `next --count n` performs claims.

## Claims and leases

Linear descriptions contain these activity markers:

```text
workflow.activity:active
workflow.activity_owner:codex
workflow.activity_lease_id:<uuid>
workflow.activity_expires_at:<ISO timestamp>
workflow.activity_updated_at:<ISO timestamp>
workflow.activity_summary:<short text>
```

An active marker is blocking only until `expires_at`. New claims include owner,
lease ID, and expiry in the initial issue creation. Missing expiration is treated
as active only for compatibility with older claims. `release` writes `inactive`;
`renew` preserves owner, summary, and lease identity while extending expiration;
`takeover` creates a new lease and requires an owner and reason without bypassing
consistency gates.

Linear lookup followed by issue creation cannot provide a database-level unique
constraint. The script uses exact marker lines, a repository fingerprint,
durable IDs, lease ownership, and legacy aliases to minimize cross-linking and
duplicates. Exact issue lookup fails closed rather than mutating the first fuzzy
search result.

## Decision state

`.agents/workflow-state.json` is an atomic local cache, not dispatch authority.
Local state and journal read-modify-write operations use a short-lived lock to
prevent parallel agents from losing records.
Schema version 1 supports:

- `updated_at` and `repo_revision`;
- `recommendation` with rationale, evidence, and stale files;
- `task_activity` with owner, lease ID, expiration, and summary;
- `task_notes` and `dependency_notes`;
- `ranked_candidates` with score, blockers, readiness, and rationale.

Do not store secrets, scratch reasoning, or unsupported guesses.

## Consistency gates

The start gate verifies spec files, unique IDs, dependency completion,
requirement references, write metadata, and active write conflicts. `pick` and
`next` run it before tracker creation.

The completion gate repeats consistency checks and always requires
`--validation-evidence <summary>`. `_validation` describes the expected plan but
is not evidence that it ran. `finish` records the evidence as checked-in task
metadata. `close` requires that evidence plus a clean `HEAD` that exactly matches
`origin/main`; narrow overrides require `--override-reason`.

## Completion journal

`.agents/workflow-operations.json` records the post-merge close operation and steps:

- `linear_moved`
- `activity_released`

Writes are atomic and retain the latest 100 operations. The journal records the
Linear identifier and target state before mutation. `reconcile --dry-run` shows
incomplete operations. Without `--dry-run`, reconciliation can finish a missing
Linear move or activity release idempotently.

## Linear representation

Issue descriptions include durable and legacy task markers, source location,
task body, requirements, reads, writes, validation, activity markers, and the
rendered prompt. The Linear response is attached to `issue.linear` before the
final prompt is rendered.

Terminal issues are not resumed. Non-terminal inactive or expired issues are
resumed instead of duplicated.

## Prompt variables

Templates use strict `{{ value }}` interpolation. Unknown variables, filters,
and Liquid tag blocks are errors. Arrays and objects render as pretty JSON.

Supported `issue` fields include `id`, `aliases`, `identifier`, `title`,
`description`, `priority`, `value`, `wave`, `state`, `branch_name`, `url`,
`labels`, `blocked_by`, `task_file`, `task_line`, `task_body`, `requirements`,
`reads`, `writes`, `validation`, `linear`, and `activity`. `attempt` is also
supported. Arrays render as comma-separated values, empty arrays as `None`, and
null values as `Not set`.

## Local UI

`ui` serves `assets/ui/index.html` with active local task data and does not claim
work by default. `begin-ui` and `ui --data next` claim before rendering. The UI
does not execute repository commands or call Linear directly.

Use `--no-open --json` when an agent runtime needs the server URL, `--host` and
`--port` to override binding, and `--static` to resolve the bundled HTML without
serving data. Non-loopback hosts are rejected unless `--allow-remote` is passed
explicitly; the UI accepts only HTTP(S) Linear links.
