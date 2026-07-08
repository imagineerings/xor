---
title: Quickstart
description: Open a project, navigate code, run commands, and try the Sim agent in a first session.
---

# Quickstart

This walkthrough gets you through a first useful Sim session in a few minutes.

## Open A Project

From a terminal:

```sh
sim ~/projects/my-app
```

Use `sim -n ~/projects/my-app` to force a new window. Inside Sim, use the command palette and search for **Open Folder** or **Open Recent**.

## Navigate

- `Cmd+P` / `Ctrl+P`: open a file by name.
- `Cmd+Shift+O` / `Ctrl+Shift+O`: jump to a symbol in the current file.
- `Cmd+Shift+F` / `Ctrl+Shift+F`: search across the project.
- `` Ctrl+` ``: toggle the integrated terminal.

The command palette is the recovery path for anything you forget.

## Edit And Run

1. Open a source file with file search.
2. Make a small edit.
3. Use the terminal to run your project’s normal test command.
4. Use the diagnostics panel to inspect language server errors and warnings.

For more detail, see [Editing Code](../editing-code.md), [Terminal](../terminal.md), and [Running & Testing](../running-testing.md).

## Try The Agent

Open the Agent Panel and ask for a small, reviewable change. Good first prompts are specific and scoped:

```text
Find the function that parses user settings and explain where validation happens.
```

```text
Add a focused unit test for the error case in this file.
```

Review agent edits before accepting them. For tool controls and approval behavior, see [Tool Permissions](../ai/tool-permissions.md).
