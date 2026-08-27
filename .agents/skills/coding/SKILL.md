---
name: coding
description: Deliver complete repository features from specification through implementation and validation. Use for new features, incomplete specifications, implementing an existing specification in full, or completing all remaining executable tasks. Do not use when the user requests only named task(s) or one next-unblocked task; use execute-spec-task for that bounded scope.
---

# End-to-end spec-driven delivery

Own the complete requested local delivery: create or reconcile the specification,
implement every pending executable task in dependency order, and validate the
result. Do not stop between tasks unless the work is genuinely blocked.

## Authorization boundary

- A request to build a feature, implement an existing specification, or complete
  all remaining tasks authorizes the corresponding specification reconciliation
  and local implementation work.
- A request for planning or specification only does not authorize product-code
  changes. Stop after the requested planning artifacts.
- A request for named/numbered tasks or the next unblocked task belongs to
  `execute-spec-task`; do not broaden it into full delivery.
- Publishing, landing, deployment, live migrations, and other external mutations
  remain separate actions unless the user explicitly authorizes them.

## Delivery modes

- **New or incompletely specified feature:** inspect the repository, create or
  complete the smallest unambiguous spec pack, then implement all executable
  leaves.
- **Implement an existing specification:** validate and reconcile the approved
  pack with the repository, then implement every pending executable leaf.
- **Complete all remaining tasks:** preserve completed state and continue through
  every pending, dependency-satisfied leaf until none remains.
- **Specification only:** create or update only the requested planning artifacts
  when the user explicitly excludes implementation.

## Canonical specification contract

`feature-spec` owns the canonical artifacts and task dialect. Read the references
needed for the requested phases:

- [requirements](../feature-spec/references/requirements.md)
- [design](../feature-spec/references/design.md)
- [tasks and task state](../feature-spec/references/tasks.md)
- [shared implementation and evidence workflow](../feature-spec/references/implementation.md)

Future or materially reworked task plans must use milestone headings, epic parent
checkboxes, decimal implementation leaves, and the exact `_Requirements:`,
`_Depends on:`, `_Reads:`, `_Writes:`, and `_Validation:` keys. Do not generate
the retired top-level packet dialect, durable `_id`, lowercase metadata,
`_blocked_by`, priority/value/wave fields, or packet-only Outcome/Design/Done
metadata.

## Validate and deliver

Before implementation, run the canonical validator in complete, canonical mode
for new or migrated packs:

```bash
python3 .agents/skills/feature-spec/scripts/validate_spec.py \
  .agents/specs/{feature-name} --require-complete --dialect canonical
```

For an existing legacy `coding` pack, use the validator's default `auto` dialect.
Compatibility validation permits execution and state/evidence maintenance without
silently invalidating the pack. Do not use compatibility mode to author a new
legacy plan. Avoid structural migration during unrelated task execution; if the
user requests a task-plan rewrite, migrate it deliberately while preserving task
meaning, order, checkbox state, durable legacy IDs, and evidence.

Follow the shared implementation reference for each task. Reconcile changed
paths and validation metadata as implementation discovers the real scope. When
behavior or architecture changes, update the affected requirements and design,
revalidate, and continue. Never redefine a completed task; add corrective work.

Completion means all requested executable leaves are implemented with concrete
evidence and the relevant repository checks pass. If no executable work remains,
report that directly. If blocked, identify the exact decision, permission, or
external dependency and preserve the remaining task state.
