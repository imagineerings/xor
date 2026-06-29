# Symphony Workflow Contract

`WORKFLOW.md` contains optional YAML front matter followed by a Markdown prompt template.

## Front Matter

Supported front matter is intentionally simple:

```yaml
---
tracker:
  kind: github_projects
polling:
  interval_ms: 30000
---
```

The current implementation accepts nested maps, scalar values, simple `- value` lists, inline lists, and literal `|` block scalars. Avoid anchors, aliases, folded block scalars, and object-valued list items.

## Template Body

Use strict double-brace interpolation:

```markdown
You are working on a GitHub Projects item.

Title: {{ issue.title }}
Repository: {{ issue.repository }}
URL: {{ issue.url }}
Status: {{ issue.status }}

Issue body:
{{ issue.body }}
```

Supported values include:

- `issue.id`
- `issue.type`
- `issue.title`
- `issue.body`
- `issue.url`
- `issue.number`
- `issue.state`
- `issue.repository`
- `issue.status`
- `issue.labels`
- `issue.assignees`
- `issue.fields`
- `attempt`

Unknown variables are errors. Filters such as `{{ issue.title | default: "Untitled" }}` are not supported.

## Recommended Agent Prompt Shape

Include enough context for a fresh agent to start without querying GitHub again:

```markdown
You are working on a GitHub Projects item.

Title: {{ issue.title }}
Repository: {{ issue.repository }}
URL: {{ issue.url }}
Status: {{ issue.status }}
Labels: {{ issue.labels }}
Assignees: {{ issue.assignees }}

Issue body:
{{ issue.body }}
```
