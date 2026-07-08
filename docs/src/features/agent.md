---
title: Agent Features
description: Use Sim's native agent for project-aware chat, edits, terminal work, and parallel tasks.
---

# Agent Features

The Sim agent can inspect your project, explain code, propose edits, run commands, and coordinate longer workflows. It works best when requests are specific, scoped, and reviewable.

## Agent Panel

Use the Agent Panel for project-aware conversations. The agent can read files, search the workspace, inspect diagnostics, and suggest edits. Keep prompts concrete:

```text
Find where provider credentials are loaded and explain the fallback order.
```

```text
Add a regression test for the empty-session case in this module.
```

## Inline Assistance

Inline assistance is best for localized edits inside the current file. Use it for refactors, comments, small transformations, and focused explanations.

## Terminal Threads

Terminal threads connect command output and agent reasoning. Use them when a task depends on build output, test failures, or CLI diagnostics.

## Parallel Agents

Parallel agents are useful for independent subtasks that can be reviewed separately. Keep parallel work on separate branches or worktrees when code changes may overlap.

## Review Flow

Treat agent changes like any other code review:

1. Read the diff.
2. Run the relevant validation.
3. Keep commits small enough to revert.
4. Prefer follow-up prompts over asking for large, ambiguous rewrites.

See [Tool Permissions](../ai/tool-permissions.md) for approval controls and [Agent Profiles](../ai/agent-profiles.md) for profile-specific tool availability.
