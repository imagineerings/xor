---
title: Configuration
description: Configure editor settings, AI providers, extensions, and project-level overrides in Sim.
---

# Configuration

Sim configuration is layered: built-in defaults, user settings, and project settings. Start with the Settings Editor, then use JSON files for advanced options.

## Editor Settings

Open settings with `Cmd+,` on macOS or `Ctrl+,` on Linux and Windows. Common first changes include:

- Theme and appearance.
- Font family and font size.
- Format-on-save behavior.
- Terminal font and shell.
- Vim or Helix compatibility modes.

See [Configuring Sim](../configuring-sim.md) for the full settings model.

## AI Providers

Open the Agent settings and configure the provider you want to use. The AI docs cover common provider paths:

- [Use API Access](../ai/use-api-access.md)
- [Use an Existing Subscription](../ai/use-an-existing-subscription.md)
- [Use a Local Model](../ai/use-a-local-model.md)
- [Use a Gateway](../ai/use-a-gateway.md)

After configuring a provider, open the Agent Panel and send a small prompt to verify credentials and model selection.

## Extensions

Install extensions for languages, themes, debuggers, snippets, and MCP integrations from the Extensions view. Start with:

- [Installing Extensions](../extensions/installing-extensions.md)
- [Language Extensions](../extensions/languages.md)
- [MCP Server Extensions](../extensions/mcp-extensions.md)

## Project Settings

Create `.sim/settings.json` in a project to override editor behavior for that repository:

```json
{
  "tab_size": 2,
  "format_on_save": "on"
}
```

Use project settings for code style and language tooling. Keep personal choices, such as theme or global keybindings, in user settings.
