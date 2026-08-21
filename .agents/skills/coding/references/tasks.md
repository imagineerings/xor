# Implementation tasks

Create or update `.agents/specs/{feature-name}/tasks.md`. Convert the established
behavior and design into small, executable task packets. When implementation is
authorized, these packets are the work queue for the remainder of the skill.

## Suggested structure

```markdown
# Implementation plan: Feature name

## Tasks

- [ ] 1. Implement descriptive outcome
  - _id: feature-name-descriptive-outcome_
  - _priority: P1_
  - _value: high_
  - _wave: 1_
  - _reads: path/to/existing.rs_
  - _writes: path/to/code.rs, path/to/test.rs_
  - _validation: cargo test -p relevant_crate test_name_
  - _Requirements: 1.1, 1.2_
  - Outcome: Observable result of completing this task.
  - Design: D1 / Component name
  - Done when: Concrete completion evidence.
```

Every executable packet must be a top-level checkbox. Use headings or plain
bullets for grouping; nested checkboxes are not independently dispatchable.
Use `_blocked_by: feature-name-prerequisite_` only when dependencies exist, and
omit it otherwise. Never encode absence as `None`.

Required fields are `_id`, `_validation`, `_Requirements`, Outcome, Design, and
Done when. Include `_writes` for every task that changes files, and `_reads` when
it improves coordination. `_priority`, `_value`, `_wave`, and `_blocked_by` are
optional, but use them when they affect ordering or parallel execution.

## Task design

- Deliver an independently verifiable increment; avoid scaffolding that no task
  uses yet.
- Reference granular acceptance criteria rather than only broad user stories.
- Name the exact command or observable check that proves completion.
- Include documentation, migration, accessibility, or manual verification tasks
  when the requirements demand them; do not restrict the plan to code edits.
- Treat reads and writes as coordination estimates. Update them when discovery
  changes the expected scope.
- Put tasks in the same wave only when neither consumes another task's output,
  their dependencies do not conflict, and their expected read or write paths do
  not overlap unsafely.
- Prefer repository build and test commands, including `./script/clippy` rather
  than `cargo clippy` in this repository.

## Consistency gate

Before finishing:

- Ensure each referenced requirement exists.
- Ensure every requirement has design and task coverage, or document why no code
  task is needed with an explicit top-level
  `- No task: 1.2 — covered by validation-only behavior` entry.
- Ensure every task has a durable repository-unique ID, validation method, design
  mapping, outcome, and done condition.
- Reconcile waves, dependencies, reads, and writes.
- Run `scripts/validate_spec.py` as directed by the skill.

## Updating task state

Preserve explicit `_id` values and `[x]`, `[~]`, or `[-]` state when updating a
pack. Do not rewrite a completed packet to describe different work; append a
corrective task instead. Mark `[x]` only after implementation and validation
evidence exist. Keep work implemented but awaiting merge distinct from merged
delivery according to the repository workflow contract.
