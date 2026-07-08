---
title: Tools
description: Understand Sim's native tools, terminal access, MCP integrations, and tool approval behavior.
---

# Tools

Tools let the agent interact with your workspace and external systems. Sim separates built-in editor tools, terminal tools, and Model Context Protocol integrations so you can choose the right level of access.

## Native Tools

Native tools are built into Sim and understand editor state. Common examples include:

- Reading and editing files.
- Searching the project.
- Listing diagnostics.
- Creating directories or moving paths.
- Inspecting symbols and code actions.

Native tools usually produce structured updates in the Agent Panel so you can see what happened.

## Terminal Tools

Terminal access is powerful because it can run arbitrary commands. Keep terminal approvals conservative for unfamiliar projects, and prefer narrow commands over broad scripts.

## MCP Tools

MCP servers expose external capabilities such as local services, memory, visualization, or team-specific integrations. Install MCP servers only from sources you trust, and review the server configuration before enabling it.

## Approval Guidelines

- Allow low-risk reads when you understand the project.
- Confirm writes, deletes, moves, terminal commands, and network access.
- Deny commands that touch secrets, credentials, or unrelated directories.
- Use project-specific settings for teams with stricter policies.

See [Security and permissions](./security-permissions.md) for the configuration model.
