# Design

Create or update `.agents/specs/{feature-name}/design.md` from the requirements
and repository discovery.

## Research order

1. Inspect existing code paths, tests, dependencies, and adjacent designs.
2. Reuse repository-native abstractions and patterns where they fit.
3. Consult external primary documentation only when the answer is not available
   locally or may have changed. Cite sources that materially influence a decision.

## Suggested structure

```markdown
# Design: Feature name

## Overview

[Approach, boundaries, and why it fits the existing system.]

## Decisions

### D1: Decision title

- Choice: ...
- Rationale: ...
- Alternatives considered: ...
- Consequences: ...

## Components and flow

[Only the interfaces, state, data flow, or diagram needed for implementation.]

## Failure and recovery

[Relevant validation, dependency failures, partial success, concurrency, and
user-visible recovery behavior.]

## Traceability

| Criterion | Design coverage | Verification type | Planned check / expected signal |
| --- | --- | --- | --- |
| 1.1 | D1 / Component name | Integration | `cargo test -p crate test_name` passes |
```

Include data models, API contracts, migrations, security, accessibility,
performance budgets, rollout, or deployment only when the feature needs them.
Use a diagram only when relationships or state transitions are materially clearer
than prose.

## Correctness properties

Add a property only for genuinely universal or invariant behavior:

```markdown
### Property 1: Descriptive name

For any [inputs satisfying a precondition], [invariant behavior].

**Validates: Requirement 1.2**
```

Use scenario or example tests for concrete workflows, static checks for structural
constraints, accessibility checks for interaction semantics, and manual checks
only when automation is impractical. Every testable acceptance criterion needs a
verification classification, but not necessarily a property.

Never renumber a surviving decision or property during an update. Append new
identifiers and preserve references from tasks and prior implementation evidence.

## Quality gate

- Explain non-obvious choices and meaningful trade-offs.
- Specify component boundaries precisely enough to implement.
- Address relevant trust boundaries, failure states, and recovery paths.
- Map every testable criterion to design coverage, verification type, and a
  planned check in the Traceability table.
- Avoid architecture or abstractions that no requirement needs.
