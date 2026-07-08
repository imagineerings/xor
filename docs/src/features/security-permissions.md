---
title: Security and Permissions
description: Configure how Sim approves agent tools, terminal commands, file changes, and MCP integrations.
---

# Security and Permissions

Sim's agent can read, edit, run commands, and call integrations depending on the active profile and permissions. Configure these controls before giving the agent broad access to important projects.

## Permission Modes

Tool permissions can allow, deny, or require confirmation. Use confirmation for operations that can modify files, execute commands, access the network, or interact with external services.

## Tool Rules

Rules can match tool input with regular expressions. Use them to block dangerous patterns or require confirmation for sensitive paths:

- Secret files such as `.env`, `.pem`, and `.key`.
- Destructive commands.
- Production deployment commands.
- Paths outside the current project.

## Profiles

Profiles control which tools are available. A read-oriented profile can keep write and terminal tools disabled, while an implementation profile can enable them with confirmations.

## MCP Security

MCP servers run outside Sim's core process and may have their own capabilities. Before enabling a server:

1. Check the command and arguments.
2. Confirm where it stores data.
3. Review network behavior.
4. Limit it to projects that need it.

## Practical Defaults

For unfamiliar repositories, start with read tools and confirmation for terminal commands. Enable broader write access only after you understand the project and have version control in a clean state.
