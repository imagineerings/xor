# Design Phase

Create `.agents/specs/{feature-name}/design.md` from approved or concurrently
requested requirements.

Inspect the existing implementation first. Prefer repository code and local
documentation as evidence. Research external technologies only when needed,
and cite authoritative sources when external research affects a decision.

## Required sections

```markdown
# Design: [Feature]

## Overview

[Approach, why it fits the requirements, and the principal trade-offs.]

## Existing context

[Relevant modules, data flows, conventions, and constraints already present.]

## Design decisions

### [Decision or component]

- Responsibility: [What it owns]
- Integration: [How it fits the existing system]
- Rationale: [Why this is the smallest suitable change]

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | [Decision/component] | [Example, invariant, or scenario test] |

## Testing strategy

- [The smallest useful unit, integration, end-to-end, or manual checks]
```

## Conditional sections

Add these only when they materially improve implementation clarity:

- Architecture or data-flow diagram for relationships that are difficult to
  explain linearly.
- Components and interfaces for new or changed boundaries.
- Data models, persistence, lifecycle, or migration rules.
- Error handling, recovery, concurrency, security, or privacy behavior.
- Compatibility, rollout, rollback, observability, or performance decisions.

Do not invent deployment topology, storage, abstractions, or interfaces for a
feature that does not need them.

## Verification forms

Use the verification form that fits each criterion:

- Example or scenario test for specific event sequences and user flows.
- State-transition test for lifecycle behavior.
- Error-path test for failures and recovery.
- Invariant/property only for behavior that is genuinely universal.

Format real invariants as:

```markdown
### Property N: [Name]

_For any_ [inputs satisfying a precondition], [invariant].

**Validates: Requirement 1.1**
```

Do not mechanically convert every acceptance criterion into a property. Ensure
every criterion appears in the requirements traceability table, and ensure the
proposed verification could actually detect a broken implementation.

If the user requested a full pack, continue to the tasks phase. Otherwise,
stop after summarizing decisions, trade-offs, and unresolved questions.
