---
name: symphony-workflow
description: Run Symphony-style agent work packets from GitHub Projects or local `.agents/specs/**/tasks.md` files using repository-owned WORKFLOW.md prompt templates. Use when an agent needs to list, claim, render, or complete work items; validate or author WORKFLOW.md files; update project status; or use the bundled single-page HTML UI for Symphony workflows.
---

# Symphony Workflow

## Overview

Use this skill to turn work items — from GitHub Projects or local `.agents/specs/**/tasks.md` files — into actionable agent prompts using a repository-owned `WORKFLOW.md` file. Each task in a `tasks.md` file is treated as a discrete work packet, with its checklist items rendered into the agent prompt. Prefer the existing MCP server when available; use the bundled static UI when a human wants a browser control panel for loading items, rendering prompts, and moving statuses.

## Workflow

1. Locate the repository root and read `WORKFLOW.md`.
2. Validate the workflow contract before claiming work.
3. Load active work items:
   - **GitHub Projects**: Fetch items from the configured owner/project.
   - **Local task files**: Load task packets from `.agents/specs/**/tasks.md`
     files matched by the `work_sources.local_tasks.glob` pattern. Each
     top-level `- [ ]` checkbox in a section is a discrete work packet.
4. Render the prompt for the selected item using strict `{{ issue.field }}`
   interpolation. For local task packets, fields like `title`, `body`, and
   `labels` are populated from the task file section.
5. Claim the item by moving its status (GitHub Projects) or marking the
   packet as in-progress when the user or workflow asks for it.
6. Complete the requested work in the target repository.
7. Update the item status after completion if the user authorizes status
   changes.

## Available Implementations

- **MCP server**: Use `server/symphony-mcp.js` when an MCP-capable agent host is configured. It exposes `symphony_validate_workflow`, `symphony_list_items`, `symphony_render_prompt`, `symphony_next_work_item`, and `symphony_update_item_status`.
- **Static UI**: Open `assets/ui/index.html` in a browser when the user wants a single-page app. The UI calls GitHub GraphQL directly from the browser, keeps the GitHub token in session storage only, and stores non-secret settings in local storage.
- **Workflow reference**: Read `references/workflow-contract.md` when editing `WORKFLOW.md` or explaining supported template variables.

## Settings

Use these fields consistently across MCP and UI implementations:

- `github_token`: GitHub token literal or `$GITHUB_TOKEN` reference; requires Projects v2 read access and write access for status updates.
- `owner`: GitHub organization or user login that owns the project.
- `project_number`: GitHub Projects v2 project number.
- `repository_path`: Repository root used to resolve relative `workflow_path` values.
- `workflow_path`: Workflow file path; defaults to `WORKFLOW.md`.
- `status_field`: Project single-select status field; defaults to `Status`.
- `active_states`: Status values considered claimable work.
- `terminal_states`: Status values excluded when no active states are configured.
- `work_sources`: List of work item sources; each entry has a `kind` field:
  - `github_projects` — source items from GitHub Projects (uses `owner`,
    `project_number`, `status_field`).
  - `local_tasks` — source items from task files on disk. Requires a `glob`
    field (defaults to `.agents/specs/**/tasks.md`).

## Prompt Rendering Rules

- Treat unknown variables as errors; do not silently remove them.
- Support dot paths such as `{{ issue.title }}`, `{{ issue.repository }}`, `{{ issue.labels }}`, and `{{ attempt }}`.
- Render arrays and objects as pretty JSON.
- Reject filters or pipes such as `{{ issue.title | default }}` unless the implementation explicitly adds support.

## Status Changes

Before moving a GitHub Projects item, confirm the requested target status is a valid option for the configured status field. Do not invent statuses. When claiming work, prefer `In Progress` only if it exists in the project.
