# Tasks Phase

Create `.agents/specs/{feature-name}/tasks.md` from the complete requirements
and design. Produce implementation units, not a capability inventory.

## Contents

- [Planning levels](#planning-levels)
- [Structure](#structure)
- [Task rules](#task-rules)
- [Task state and evidence](#task-state-and-evidence)
- [Legacy compatibility](#legacy-compatibility)
- [Decomposition audit](#decomposition-audit)
- [Generic examples](#generic-examples)

## Planning levels

- **Milestone:** an outcome-oriented heading that groups related epics. It is
  not a checkbox and has no task metadata.
- **Epic:** a parent checkbox for a capability that requires multiple
  implementation boundaries or independently reviewable outcomes. It has no
  task metadata and is not executable as one unit.
- **Implementation leaf:** an indented checkbox that produces one coherent
  behavior or artifact within one primary implementation boundary. It can be
  completed in one focused agent run and reviewed independently.

Decompose every epic until every leaf names concrete read/write paths, has
focused validation proving its outcome, and can be executed without
rediscovering its scope. Do not combine independently reviewable domain,
persistence, transport, UI, client, migration, deployment, or test work in one
leaf. A phrase such as “implement X, Y, and Z across backend, UI, clients, and
deployment” identifies an epic, not a leaf.

## Structure

```markdown
# Implementation Plan: [Feature]

## Approach

[Brief ordering rationale and the existing repository patterns to reuse.]

## Dependency waves

- Wave 1: [Unblocked tasks; identify parallel-safe tasks when useful]
- Wave 2: [Tasks that depend on Wave 1]

## Tasks

### Milestone 1: [Outcome]

- [ ] 1. [Capability epic]
  - [ ] 1.1. Add [coherent behavior] at [primary boundary]
    - Reuse [existing module or pattern].
    - Handle [focused success and failure behavior].
    - _Requirements: 1.1, 1.2_
    - _Depends on: none_
    - _Reads: path/to/existing.rs_
    - _Writes: path/to/existing.rs, path/to/focused_test.rs_
    - _Validation: focused observable check or repository command_
  - [ ] 1.2. Add [next independently reviewable outcome]
    - _Requirements: 1.2_
    - _Depends on: 1.1_
    - _Reads: path/to/existing.rs_
    - _Writes: path/to/next_boundary.rs_
    - _Validation: focused observable check or repository command_
```

Omit dependency waves when the plan is strictly linear and the task ordering is
already obvious.

## Task rules

- Use milestone headings, integer epic IDs such as `2`, and decimal leaf IDs
  such as `2.1`. Use no deeper task nesting.
- Put `_Requirements:`, `_Depends on:`, `_Reads:`, `_Writes:`, and
  `_Validation:` metadata on every leaf and only on leaves. Spell and capitalize
  these keys exactly. `_Evidence:` is the only optional execution-state metadata.
- Reference every acceptance criterion from at least one leaf task.
- Include `_Depends on: none_` when a task has no prerequisite.
- Make dependencies reference leaf IDs. Recompute them whenever leaves split.
- List concrete paths or focused globs in `_Reads:` and `_Writes:`. Use `none`
  only when a task genuinely makes no file changes.
- Treat validator warnings for recursive or directory-wide globs, more than
  five write targets, multiple subsystem roots, or unsequenced repeated writes
  as prompts to split or sequence the leaf.
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

Do not use the retired alternate `coding` dialect for new or materially rewritten
plans. In particular, do not create top-level executable packets with `_id`,
lowercase `_reads`/`_writes`/`_validation`, `_blocked_by`, `_priority`, `_value`,
`_wave`, or packet-only Outcome/Design/Done fields.

## Task state and evidence

Use the same state markers for canonical leaves and legacy compatibility packets:

- `[ ]` pending and not yet implemented;
- `[~]` started, implemented but not fully validated, or otherwise incomplete;
- `[x]` implemented and validated;
- `[-]` deliberately superseded or removed without redefining the original work.

Add one `_Evidence:` line when transitioning a leaf to `[x]` or `[-]`. Evidence
must name the validation performed and its result, or explain why the task was
superseded. Validation that was not run is not completion evidence: keep the leaf
at `[~]`, record what remains, and do not mark it `[x]`. Preserve existing evidence
and append material new evidence rather than erasing delivery history.

Epic state summarizes its leaves and carries no task metadata: mark it `[x]` only
when every non-superseded leaf is `[x]`, `[~]` while any descendant work is active,
and `[ ]` while pending work remains. Never change a completed leaf to describe
new behavior; add a new leaf instead.

## Legacy compatibility

Existing packs may use the retired top-level packet dialect previously generated
by `coding`. The canonical validator auto-detects these packs and validates them
through a compatibility path. A legacy top-level numbered checkbox is executable,
its durable `_id` remains stable, and `_blocked_by` names durable prerequisite
IDs. Narrow execution updates may preserve the existing spelling and add
`_Evidence:` without migrating the entire pack.

Compatibility is not an authoring mode. Use `--dialect canonical` when creating a
new pack or materially rewriting task structure. Do not silently migrate a legacy
pack during unrelated implementation. When migration is explicitly requested,
preserve task meaning, ordering, checkbox state, evidence, completed history, and
durable legacy IDs in an explicit migration mapping rather than as canonical leaf
metadata; validate the migrated plan in canonical mode before using it.

## Decomposition audit

After drafting, perform this manual audit and revise the plan before running
the validator:

1. Identify compound leaves, including conjunctions, multiple primary
   boundaries, and lists of independently testable outcomes.
2. Split every independently testable or reviewable outcome into its own leaf.
3. Recompute leaf dependencies and dependency waves after every split.
4. Sequence overlapping writes; do not mark conflicting leaves parallel-safe.
5. Confirm each leaf states one outcome, one primary boundary, concrete paths,
   focused validation, and enough detail to execute without scope discovery.
6. Confirm every requirement traces through the design to an implementation
   leaf, not only an epic.
7. Run `scripts/validate_spec.py` as directed by the main skill and manually
   review every granularity warning. Passing validation does not replace this
   semantic review.

## Generic examples

Invalid epic presented as a leaf:

```markdown
- [ ] 1. Implement sharing across the domain model, database, API, web UI,
  mobile client, migration, deployment, and tests
```

The same epic decomposed:

```markdown
### Milestone 1: Sharing available end to end
- [ ] 1. Add sharing
  - [ ] 1.1. Define sharing policy in `src/domain/sharing.rs`
  - [ ] 1.2. Persist shares in `src/storage/shares.rs`
  - [ ] 1.3. Expose share commands in `src/api/shares.rs`
  - [ ] 1.4. Render sharing controls in `web/sharing.tsx`
```

This decomposition-only view omits metadata; every leaf still requires the
metadata shown in the valid leaf example.

A valid implementation leaf:

```markdown
  - [ ] 1.2. Persist accepted shares through the existing repository
    - _Requirements: 1.2_
    - _Depends on: 1.1_
    - _Reads: src/domain/sharing.rs, src/storage/repository.rs_
    - _Writes: src/storage/shares.rs, tests/storage/shares_test.rs_
    - _Validation: run the focused storage tests for accepted and rejected shares_
```

Stop after reporting the completed plan and validation result. Do not implement
any task without a separate request.
