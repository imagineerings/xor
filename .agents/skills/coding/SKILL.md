---
name: coding
description: Define and deliver repository features through traceable specifications under .agents/specs/ and implementation of every resulting task. Use when Codex needs to turn an idea into requirements, design, validation strategy, and working code; implement an existing spec pack; resume unfinished spec tasks; or update specifications while delivering a change. Stop after planning only when the user explicitly requests specification or planning work without implementation.
---

# Spec-driven delivery

Create the smallest spec pack that makes implementation unambiguous. Store it at
`.agents/specs/{feature-name}/`, using a kebab-case feature name.

## Select the delivery mode

- **Specification only:** Create or update only the requested planning artifacts
  when the user explicitly asks for planning without implementation.
- **Implement existing specification:** Validate the existing pack, reconcile it
  with the repository, and complete every pending executable task.
- **Full delivery:** Define the requirements and design, create executable tasks,
  then continue through implementation without requiring another prompt.

When implementation is authorized, do not stop after writing the specification.
Continue until every executable task is implemented and validated, or until a
blocking decision, permission, or external dependency prevents further progress.

## Establish scope

1. Read applicable repository instructions, related specs, relevant code, tests,
   dependencies, and build commands before proposing behavior or architecture.
2. Determine which artifacts the user requested and which already exist.
3. Update only affected artifacts unless the user requests a complete spec pack.
4. Preserve established repository terminology. Narrative prose may use the
   user's language, but keep machine-parsed headings, identifiers, and metadata
   keys exactly as shown in the references.

Do not force a new feature through separate conversational gates when the request
is already clear. Produce all requested artifacts in one pass. Pause after
requirements only when unresolved choices would materially change the design.

## Create the artifacts

- For requirements work, read
  [references/requirements.md](references/requirements.md).
- For architecture or design work, read
  [references/design.md](references/design.md).
- For an implementation plan or checklist, read
  [references/tasks.md](references/tasks.md).
- For implementation or full delivery, read
  [references/implementation.md](references/implementation.md).

Use only sections relevant to the feature. Do not add empty deployment, data,
integration, performance, or property-testing sections for template compliance.

## Maintain traceability

Use stable identifiers and maintain this chain for every testable behavior:

`requirement -> design decision/component -> task -> validation`

Classify verification according to the behavior. Use example-based, integration,
property-based, static, accessibility, performance, or manual verification as
appropriate. Do not mechanically rewrite every acceptance criterion as a
universal property.

Before starting implementation, reconcile all changed documents and run:

```bash
python3 .agents/skills/coding/scripts/validate_spec.py .agents/specs/{feature-name}
```

Add `--require-complete` before implementation or when the user requested a
complete pack. Requirements are a prerequisite for design, and requirements plus
design are prerequisites for executable tasks.

Treat canonical `_reads:` and `_writes:` entries as coordination estimates. Keep
them current as implementation discovery changes the expected scope.

## Deliver the implementation

Use the repository's `workflow` skill when applicable to plan, claim, validate,
and hand off task packets. Preserve durable task IDs and task state. Execute tasks
in dependency order, and parallelize only packets whose dependencies and read or
write sets do not conflict.

For each task, implement the smallest complete increment, run its declared
validation, and use `living-documentation` when delivered behavior or design
changes. If implementation invalidates the specification, update and revalidate
the affected artifacts before continuing. Do not mark a task complete without
concrete validation evidence.

After all tasks are implemented, rerun pack validation and the smallest relevant
repository checks. Distinguish locally implemented and validated work from merged
delivery: publish, land, deploy, or mutate external systems only when authorized.
Ask first only for expensive, destructive, privileged, or externally mutating
checks that are not already implied by the requested delivery workflow.
