---
title: Troubleshooting
description: Diagnose common Sim configuration, provider, extension, and agent issues.
---

# Troubleshooting

Use this checklist when Sim launches but a configured feature does not behave as expected.

## Provider Errors

- Confirm the selected model is available for the configured provider.
- Check whether credentials are expired, missing, or scoped incorrectly.
- Try a small prompt to separate provider access from project-specific context.
- If using a gateway, confirm the gateway URL, model name, and authentication header.

## Extension Issues

- Reload the window after installing or updating an extension.
- Confirm the extension supports the file type or project language.
- Check language server diagnostics and logs when completions or formatting fail.
- Disable recently added extensions if startup behavior changes unexpectedly.

## MCP Issues

- Run the configured MCP command manually to confirm it starts.
- Check that required environment variables are present.
- Confirm the server can read/write its configured data directory.
- Review tool permissions if the agent can see the MCP server but calls fail.

## Agent Tool Issues

- Confirm the active profile enables the tool.
- Review the approval prompt and permission rules.
- Use a narrower prompt if the agent chooses a risky or unrelated tool.
- Check terminal output or diagnostics when a command fails.

For broader install and platform issues, see the existing [Troubleshooting](../troubleshooting.md) page.
