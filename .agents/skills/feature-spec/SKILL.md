---
name: feature-spec
description: Create, review, or update feature specification packs under .agents/specs, including requirements, acceptance criteria, technical designs, and implementation task plans. Use for PRDs, RFCs, feature specs, requirements, user stories, EARS acceptance criteria, architecture or design documents, traceability reviews, and implementation checklists. Do not use to implement the planned code.
---

# Feature Spec

Create traceable planning artifacts under `.agents/specs/{feature-name}/`, where
`feature-name` is kebab-case.

## Workflow

1. Read repository instructions such as `AGENTS.md`. They override this skill.
2. Inspect the relevant code, documentation, and existing spec pack before
   making architectural claims.
3. Determine the requested scope:
   - For requirements, read [requirements.md](references/requirements.md).
   - For design, read [design.md](references/design.md) and the requirements.
   - For tasks, read [tasks.md](references/tasks.md) and the full spec pack.
   - For a full spec pack, read all three references and produce the phases in
     order during the same turn.
4. Preserve existing decisions when updating a pack. Reconcile contradictions
   instead of silently replacing approved behavior.
5. After creating or updating `tasks.md`, run:

   ```bash
   python3 .agents/skills/feature-spec/scripts/validate_spec.py \
     .agents/specs/{feature-name}
   ```

6. Before handoff, perform the mandatory manual decomposition audit in
   [tasks.md](references/tasks.md). The validator cannot prove semantic task
   size or implementation readiness.
7. Report the files created or updated, material assumptions, validation
   results, and unresolved decisions. Do not implement the feature.

## Authorization and review

- Treat an explicit request for a complete spec, PRD, RFC, or implementation
  plan as authorization to create all requested planning phases.
- Stop after a single phase when the user requested only that phase.
- Ask before choosing between materially different product or architecture
  directions when repository context does not resolve the choice.
- Always require a separate implementation request before changing product
  code. Use the `execute-spec-task` skill for that work.

## Quality rules

- Write in the user's language when practical.
- Reuse repository terminology and established architecture.
- Include only sections that help implement or review this feature.
- Give every acceptance criterion a stable ID such as `1.1`.
- Trace every criterion through the design and at least one leaf task.
- Structure implementation plans as milestone headings, epic parent tasks, and
  implementation leaves. Never present a capability epic as a leaf.
- Prefer concrete observable behavior over implementation detail in
  requirements.
- Prefer the smallest design that satisfies the approved requirements without
  weakening security, accessibility, validation, or failure handling.
- Plan tests and necessary migrations, configuration, or documentation when
  they are part of shipping the feature. Do not perform deployment or other
  production mutations.

## Example prompts

- "Create a complete RFC for resumable agent sessions."
- "Write EARS acceptance criteria for offline reconnect behavior."
- "Update the design for the existing provider failover spec."
- "Turn these approved requirements into an implementation checklist."
- "Review this feature spec for missing traceability and contradictions."
