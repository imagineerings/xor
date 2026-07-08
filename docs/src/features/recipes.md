---
title: Recipes
description: Package repeatable Sim agent workflows as recipes for common tasks.
---

# Recipes

Recipes are reusable prompts and workflow definitions for the agent. They help teams standardize repeated tasks such as triage, release notes, migration steps, and code review preparation.

## When To Use A Recipe

Use a recipe when a task has a repeatable shape:

- Collect context from known files.
- Run the same validation commands.
- Produce a standard output format.
- Follow a team-specific checklist.

Avoid recipes for one-off questions or tasks where the right process is still unclear.

## Recipe Inputs

Good recipes define the information the agent needs up front:

- Target files or directories.
- Required commands.
- Output format.
- Safety constraints.
- Review or handoff checklist.

## Running Recipes

Open the Agent Panel and choose the recipe flow when available. Review generated plans and diffs the same way you would review ordinary agent work.

## Team Practices

Keep recipes small and version them with the project when they encode project-specific behavior. Update recipes when validation commands, ownership boundaries, or release processes change.
