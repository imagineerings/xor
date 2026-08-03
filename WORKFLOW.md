---
tracker:
  kind: linear
  api_key: $LINEAR_API_KEY
  team_key: $LINEAR_TEAM_KEY
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
You are working on a local Sim spec task.

Task: {{ issue.title }}
Task ID: {{ issue.id }}
Source: {{ issue.task_file }}:{{ issue.task_line }}
Priority: {{ issue.priority }}
Blocked by: {{ issue.blocked_by }}
Requirements: {{ issue.requirements }}
Reads: {{ issue.reads }}
Writes: {{ issue.writes }}
Validation: {{ issue.validation }}
Owner: {{ issue.activity.owner }}
Lease expires: {{ issue.activity.expires_at }}

Task body:
{{ issue.task_body }}

Linear: {{ issue.linear.url }}

The claim command already ran the start gate. Renew the lease before it expires.
If work stops, release it with a concise summary. Before merge, run `workflow
finish` with validation evidence. After the merged main branch is synced
locally, run `workflow close`.
