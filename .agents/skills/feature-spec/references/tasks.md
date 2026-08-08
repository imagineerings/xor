# Tasks Phase

Create `.agents/specs/{feature-name}/tasks.md` from the complete requirements
and design. Make each leaf task independently actionable and small enough to
review.

## Structure

```markdown
# Implementation Plan: [Feature]

## Approach

[Brief ordering rationale and the existing repository patterns to reuse.]

## Dependency waves

- Wave 1: [Unblocked tasks; identify parallel-safe tasks when useful]
- Wave 2: [Tasks that depend on Wave 1]

## Tasks

- [ ] 1. Add the core behavior at the existing integration point
  - Reuse [existing module or pattern].
  - Handle [relevant success and failure behavior].
  - _Requirements: 1.1, 1.2_
  - _Depends on: none_
  - _Reads: path/to/existing.rs_
  - _Writes: path/to/existing.rs_
  - _Validation: focused observable check or repository command_

- [ ] 2. Add regression coverage
  - Verify the behavior through the repository's existing test pattern.
  - _Requirements: 1.1, 1.2_
  - _Depends on: 1_
  - _Reads: path/to/existing.rs_
  - _Writes: crates/example/tests/feature.rs_
  - _Validation: focused test command_
```

Omit dependency waves when the plan is strictly linear and the task ordering is
already obvious.

## Task rules

- Use at most two levels: tasks and subtasks. Metadata is required on every
  leaf task; parent grouping tasks do not need metadata.
- Use stable decimal IDs such as `2` and `2.1`.
- Reference every acceptance criterion from at least one leaf task.
- Include `_Depends on: none_` when a task has no prerequisite.
- List files or focused globs in `_Reads:` and `_Writes:` when known. Use
  `none` only when a task genuinely makes no file changes.
- Make `_Validation:` concrete. Name the command only when repository
  instructions permit running it; otherwise name the observable check and the
  authorization needed.
- Follow the repository's language, structure, and existing abstractions. Do
  not begin with generic scaffolding, layers, interfaces, or directories.
- Include tests, migrations, configuration, generated artifacts, and user or
  developer documentation when required to ship the behavior.
- List rollout or production operations as handoff steps when necessary, but do
  not execute them as part of planning.
- Avoid orphaned code, speculative extensibility, and tasks that exist only to
  create boilerplate.

## Consistency pass

Before completing the pack:

1. Confirm every referenced requirement exists.
2. Confirm every requirement is covered by the design traceability table and a
   leaf task.
3. Reconcile task dependencies with the dependency waves.
4. Review repeated `_Writes:` paths for sequencing or parallel conflicts.
5. Confirm the planned validation respects repository instructions.
6. Run `scripts/validate_spec.py` as directed by the main skill.

Stop after reporting the completed plan and validation result. Do not implement
any task without a separate request.
