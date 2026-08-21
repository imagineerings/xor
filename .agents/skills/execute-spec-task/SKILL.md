---
name: execute-spec-task
description: Implement one or more explicitly requested tasks from an existing feature specification under .agents/specs. Use when the user asks to start, execute, implement, or continue a numbered task from an approved requirements.md, design.md, and tasks.md pack. Do not use for general coding requests without a feature spec.
---

# Execute Spec Task

Implement the requested scope from an existing feature spec without expanding
the approved behavior.

## Workflow

1. Read repository instructions such as `AGENTS.md`; they override this skill.
2. Locate and read the feature's `requirements.md`, `design.md`, and `tasks.md`
   completely.
3. Run the spec validator before editing when it is available:

   ```bash
   python3 .agents/skills/feature-spec/scripts/validate_spec.py \
     .agents/specs/{feature-name}
   ```

4. Select the task named by the user. If no task is named, choose the first
   incomplete task whose dependencies are satisfied and state that choice.
   Default to one leaf task; honor an explicit request for multiple tasks.
5. Confirm the task's requirements, dependencies, expected reads and writes,
   and validation. Resolve blocking contradictions before coding.
6. Inspect the real code path and existing tests. Reuse established patterns
   and implement the smallest complete change that satisfies the referenced
   requirements.
7. Run formatting and validation only as authorized by the user and repository
   instructions. Do not substitute an unrequested broad build or test run for
   the task's focused validation.
8. Reconcile `tasks.md` with the delivered work:
   - Mark completed leaf tasks checked.
   - Update `_Reads:` and `_Writes:` when the actual files differ.
   - Add an `_Evidence:` line with validation performed or explicitly not run.
   - Update dependencies for remaining work only when implementation changed
     them.
9. Update requirements or design only when implementation exposed an error in
   the spec. Ask before making a material behavior or architecture change.
10. Stop after the requested scope and report changed files, validation, spec
    reconciliation, and remaining risks. Do not start the next task
    automatically.

## Guardrails

- Do not treat approval of a planning document as authorization to implement
  it; require an implementation request.
- Do not skip security, accessibility, trust-boundary validation, or failure
  handling required by the spec.
- Do not create speculative abstractions, layers, dependencies, or files.
- Do not perform deployments, releases, migrations against live systems, or
  other external mutations unless the user explicitly requests and authorizes
  them.
- Preserve user changes and unrelated work in the workspace.

## Example prompts

- "Implement task 2.1 from the resumable-sessions spec."
- "Start the next unblocked task in `.agents/specs/provider-failover`."
- "Complete tasks 3 and 4 from the approved implementation plan."
