---
name: coding
description: Create a spec for a feature in .agents/specs create/update feature spec/PRD/RFC—requirements, design doc, and implementation tasks checklist. Trigger on spec/specification/PRD/RFC/tech spec, requirements/user story/acceptance criteria/EARS, design doc/architecture, task breakdown/implementation plan/checklist; Requirements/Acceptance/Design/Task.
---

# Agent: Spec-Driven Development Workflow

Transform ideas into comprehensive specifications, design documents, and actionable implementation plans.

## When to use

Use this skill when you want a spec pack under `.agents/specs/`:

1. requirements with EARS acceptance criteria,
2. a design doc with architecture + correctness properties,
3. an executable tasks checklist.

## Workflow

1. **Requirements** → Define what to build (EARS format) → [Details](references/phase-1-requirements.md)
2. **Design** → How to build it (architecture + correctness properties) → [Details](references/phase-2-design.md)
3. **Tasks** → Actionable implementation steps → [Details](references/phase-3-tasks.md)
4. **Execute** → Implement one task at a time → [Details](references/phase-4-execute.md)

When implementation work touches `webapp/`, use the `style-guide` skill before
editing and again during review so React, TypeScript, SCSS, accessibility, i18n,
testing, Redux, and networking choices follow the Simtropolis web app style
guide.

**Storage**: `.agent/specs/{feature-name}/` (kebab-case)

---

## Core Rules

- **Sequential phases** — Never skip phases
- **Explicit approval** — Get user approval after each document
- **One task at a time** — During execution, focus on single task
- **Correctness mandatory** — Every design MUST include properties from EARS
- **Consistency passes mandatory** — Before executing a task and before marking
  it complete, reconcile gates, dependency waves, and all relevant spec
  documents.

## Consistency Pass

Run this pass at two points: before an agent begins a task, and after
implementation is validated but before the task is marked complete.

1. **Tighten gates** — Ensure requirements, design, and tasks have concrete
   start gates, validation gates, handoff gates, and completion gates. Replace
   vague checks such as "verify" with explicit commands, observable outcomes,
   acceptance criteria, or done conditions where possible.
2. **Update dependency waves** — Confirm the task order, prerequisites, and
   parallel-safe groups match the current design and `_writes:` manifests.
   Document dependencies between tasks and adjust waves when implementation
   discoveries change ordering or parallel safety.
3. **Check document agreement** — Read the relevant `requirements.md`,
   `design.md`, and `tasks.md` together. Confirm every task references existing
   requirements, design properties validate those requirements, task writes and
   reads are accurate, and no document contradicts another. Update the spec
   files or ask for clarification before proceeding on an inconsistency.

## Quick Reference

### EARS Acceptance Criteria Format

```
WHEN [event] THEN THE [system] SHALL [response]
IF [condition] THEN THE [system] SHALL [response]
WHILE [state] THE [system] SHALL [response]
```

### Correctness Property Format

```markdown
### Property N: [Name]

_For any_ [inputs], [precondition], [system] SHALL [behavior].

**Validates: Requirement X.Y**
```

### Phase Outputs

| Phase        | Output File       | Key Content                            |
| ------------ | ----------------- | -------------------------------------- |
| Requirements | `requirements.md` | User stories + EARS ACs                |
| Design       | `design.md`       | Architecture + Interfaces + Properties |
| Tasks        | `tasks.md`        | Checkbox task list                     |

## Workflow Diagram

```mermaid
stateDiagram-v2
  [*] --> Requirements

  Requirements --> ReviewReq : Complete
  ReviewReq --> Requirements : Changes
  ReviewReq --> Design : Approved

  Design --> ReviewDesign : Complete
  ReviewDesign --> Design : Changes
  ReviewDesign --> Tasks : Approved

  Tasks --> ReviewTasks : Complete
  ReviewTasks --> Tasks : Changes
  ReviewTasks --> [*] : Approved

  Execute : Execute Single Task
  [*] --> Execute : Task Request
  Execute --> [*] : Complete
```

## Detection Logic

Determine current state by checking:

```bash
# Check for .agent directory
if [ -d ".agents/specs" ]; then
  # List features
  ls .agents/specs/

  # For specific feature, check phase
  FEATURE="$1"
  if [ -f ".agents/specs/$FEATURE/requirements.md" ]; then
    echo "Requirements exists"
  fi
  if [ -f ".agents/specs/$FEATURE/design.md" ]; then
    echo "Design exists"
  fi
  if [ -f ".agents/specs/$FEATURE/tasks.md" ]; then
    echo "Tasks exists - ready for execution"
  fi
fi
```

## Summary

Kiro provides a structured, iterative approach to feature development:

- Start with **requirements** (what to build)
- Progress to **design** (how to build it)
- Create **tasks** (implementation steps)
- **Execute** tasks one at a time

Each phase requires explicit user approval before proceeding, ensuring alignment and quality throughout the development process.

## Supporting Files

- [Agent Identity](references/agent-identity.md) — Response style
- [Workflow Diagrams](references/workflow-diagrams.md) — Visual references
