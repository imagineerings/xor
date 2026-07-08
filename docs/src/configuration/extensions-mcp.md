---
title: Extensions and MCP
description: Configure Sim extensions and Model Context Protocol servers.
---

# Extensions and MCP

Extensions add languages, themes, debugger adapters, snippets, and agent integrations. MCP servers expose external tools and services to the agent.

## Extensions

Use the Extensions view to install and manage extensions. Common extension types include:

- Language support.
- Themes and icon themes.
- Debugger integrations.
- Snippets.
- Agent server integrations.

See [Installing Extensions](../extensions/installing-extensions.md) and [Extension Capabilities](../extensions/capabilities.md).

## MCP Servers

MCP servers are configured as commands that Sim can launch or connect to. Review the command, arguments, environment, and working directory before enabling a server.

Prefer project-scoped MCP configuration when an integration is only useful for one repository. Prefer user-level configuration for trusted tools that you use everywhere.

## Safety Checklist

- Install servers from trusted sources.
- Review requested environment variables.
- Avoid passing secrets through shell history.
- Confirm where the server stores local data.
- Keep broad filesystem or network tools behind confirmation.

See [MCP Server Extensions](../extensions/mcp-extensions.md) and [Security and Permissions](../features/security-permissions.md).
