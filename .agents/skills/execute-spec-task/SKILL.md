---
name: execute-spec-task
description: Implement bounded task scope from an existing approved specification under .agents/specs. Use for named or numbered task(s), or one next-unblocked task when none is named. Do not use for new features, incomplete specifications, full-spec implementation, or completing all remaining tasks.
---

# Execute approved spec task

Implement only the requested task scope from an existing approved specification.
Stop when that scope is complete or blocked; do not continue to the next task.

## Authorization boundary

- Require an existing `requirements.md`, `design.md`, and `tasks.md` pack whose
  behavior and architecture the user has approved or explicitly asked to execute.
- Do not treat planning approval, a checked box, or repository presence alone as
  implementation authorization.
- Do not add behavior, change architecture, or expand task scope without asking.
- Do not create a missing specification or turn an incomplete pack into a full
  delivery. Route that work to `coding` or `feature-spec` as appropriate.
- Do not deploy, release, publish, run live migrations, or perform other external
  mutations unless separately requested and authorized.

## Workflow

1. Read repository instructions and the feature's `requirements.md`, `design.md`,
   and `tasks.md` completely.
2. Read the canonical [task schema and state conventions](../feature-spec/references/tasks.md)
   and [shared implementation workflow](../feature-spec/references/implementation.md).
3. Validate the existing pack with compatibility auto-detection:

   ```bash
   python3 .agents/skills/feature-spec/scripts/validate_spec.py \
     .agents/specs/{feature-name} --require-complete
   ```

   Resolve contradictions that make the requested task unsafe or ambiguous. A
   compatibility notice for a legacy `coding` pack is not itself a blocker.
4. Select exactly the task(s) the user named. A named epic means its incomplete
   descendant leaves, not unrelated epics. If no task is named, choose the first
   incomplete leaf whose dependencies are complete, state the selection, and
   execute only that one leaf. For a legacy packet pack, a top-level numbered
   packet is an executable leaf and `_blocked_by` defines its prerequisites.
5. Confirm requirement coverage, dependencies, expected reads/writes, and focused
   validation. Inspect the real code and tests before editing.
6. Follow the shared implementation workflow for the selected scope. Implement
   the smallest complete change that satisfies the approved behavior.
7. Reconcile only task-execution facts: actual `_Reads:`/`_Writes:`, validation,
   state, and evidence. Preserve legacy metadata spelling when making a narrow
   state/evidence update to a compatibility pack.
8. If implementation exposes a specification error, fix a non-material factual
   inconsistency and revalidate. Ask before changing approved behavior,
   architecture, task decomposition, or dependencies outside the selected scope.
9. Stop after the requested task scope. Report changed files, validation evidence,
   spec reconciliation, and remaining risks without starting another task.

## Guardrails

- Preserve user changes, completed task state, durable legacy IDs, and unrelated
  specifications.
- Do not skip security, accessibility, trust-boundary, failure, or recovery
  requirements attached to the selected task.
- Do not create speculative abstractions, layers, dependencies, or files.
- Do not replace focused task validation with an unrequested broad test run.
