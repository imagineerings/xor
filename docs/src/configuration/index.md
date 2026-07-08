---
title: Configuration
description: Configure Sim settings, providers, extensions, MCP servers, and troubleshooting workflows.
---

# Configuration

Sim configuration covers editor settings, AI providers, extensions, MCP servers, and project-specific overrides. Start with the Settings Editor for common preferences, then use JSON settings when you need repeatable team configuration.

## Settings Layers

Settings apply in this order:

1. Built-in defaults.
2. User settings.
3. Project settings in `.sim/settings.json`.

Later layers override earlier ones. Use user settings for personal preferences and project settings for repository behavior such as indentation or formatting.

## Common Areas

- [Providers](./providers.md): configure hosted, subscription, gateway, and local AI models.
- [Extensions and MCP](./extensions-mcp.md): install language extensions and connect MCP servers.
- [Troubleshooting](./troubleshooting.md): diagnose install, provider, extension, and agent problems.

See [Configuring Sim](../configuring-sim.md) for the existing settings reference.
