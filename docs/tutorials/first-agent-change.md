---
title: Make Your First Agent-Assisted Change
description: Use the Sim agent to inspect a project, make a small edit, and validate the result.
---

# Make Your First Agent-Assisted Change

Use this tutorial to practice a small, reviewable agent workflow.

## Before You Start

- Open a project in Sim.
- Make sure the project is under version control.
- Confirm an AI provider is configured.

## Steps

1. Open the Agent Panel.
2. Ask the agent to inspect a narrow area:

   ```text
   Find the README section that explains setup and suggest one clarity improvement.
   ```

3. Review the files the agent reads.
4. Ask for a small edit:

   ```text
   Apply the smallest wording change needed and show me the diff.
   ```

5. Review the diff.
6. Run any relevant formatting or docs validation.
7. Commit the change if it is correct.

## Expected Result

You should finish with one small diff, a clear explanation of why it changed, and a validation command or reason why validation was not needed.
