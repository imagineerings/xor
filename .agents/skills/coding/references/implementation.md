# Implementation

Use this workflow after a complete specification passes validation and the user
has authorized implementation.

## Establish the work queue

1. Inspect repository state and preserve unrelated user changes.
2. Run the repository workflow's read-only planning command when available.
3. Reconcile task state with the code. Do not redo work that is already present
   and validated, but do not trust a checked box without supporting evidence.
4. Select every pending task in dependency order. Execute compatible packets in
   parallel only when the repository workflow confirms they do not conflict.

If an existing pack predates the current schema, migrate its traceability and
task metadata before implementation. Preserve stable IDs and checkbox state,
record unresolved legacy findings, and do not use migration as evidence that a
previously checked task was actually delivered.

## Execute each task

For each task packet:

1. Run its start consistency gate when the repository provides one.
2. Read the declared files and enough adjacent code and tests to confirm the
   planned approach still fits.
3. Implement the smallest complete increment. Propagate failures to the user-facing
   layer and follow repository instructions.
4. Run the task's declared `_validation` plus any newly relevant focused check.
5. Update requirements, design, traceability, task paths, or validation when
   implementation discovery changes the documented behavior or architecture.
6. Revalidate the affected spec pack.
7. Record concise validation evidence and transition the task only when the
   workflow's completion criteria are satisfied.

Do not pause between tasks merely to ask whether to continue. Continue until no
executable task remains. Pause only for a decision that materially changes the
requirements or design, missing authority for an external mutation, or a blocker
that prevents meaningful progress.

## Replan safely

When implementation reveals an invalid assumption, stop work on dependent tasks,
update the affected requirements and design, reconcile task dependencies and
manifests, and rerun complete-pack validation. Preserve stable identifiers and
completed task history. Add corrective tasks rather than silently redefining
completed work.

## Finish delivery

After all tasks are implemented:

1. Confirm every acceptance criterion has design, task, and verification coverage.
2. Run complete-pack validation.
3. Run the smallest relevant repository-wide formatting, static, and test checks.
4. Confirm living documentation describes delivered behavior.
5. Report completed tasks, validation evidence, and any publishing or merge work
   that remains outside the user's authorization.

Implementation completion means the code and local validation are complete.
Merged, released, or deployed delivery is a separate state unless the user also
authorized those external workflow steps.
