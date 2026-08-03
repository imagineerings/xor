# Requirements

Create or update `.agents/specs/{feature-name}/requirements.md`.

## Discovery

Confirm the current behavior, affected users, repository vocabulary, constraints,
and nearby behavior before writing requirements. Record assumptions only when
they affect scope. Ask the user only about decisions that cannot be inferred
safely and would materially change the result.

## Suggested structure

```markdown
# Requirements: Feature name

## Problem

[Who is affected, what is wrong or missing, and the intended outcome.]

## Scope

- In scope: ...
- Out of scope: ...

## Requirements

### Requirement 1: Descriptive title

**User story:** As a [role], I want [capability], so that [benefit].

#### Acceptance criteria

1. WHEN [event] THEN THE [system] SHALL [observable response].
2. IF [condition] THEN THE [system] SHALL [observable response].
```

Use a user story only when it clarifies the actor and outcome. For internal or
system requirements, use a concise actor/outcome statement instead. Add a
glossary, assumptions, constraints, non-functional requirements, or open
questions only when they improve precision.

## Acceptance criteria

Use EARS when it makes behavior clearer:

- `WHEN [event] THEN THE [system] SHALL [response]`
- `IF [condition] THEN THE [system] SHALL [response]`
- `WHILE [state] THE [system] SHALL [response]`
- `WHERE [feature applies] THE [system] SHALL [response]`
- `THE [system] SHALL [response]`

Refer to criteria as `1.1`, `1.2`, and so on: requirement number followed by
criterion number. Make each criterion observable and independently verifiable.
Cover relevant success, boundary, failure, permission, accessibility, and state
transition behavior without inventing speculative cases.

In Markdown source, number criteria explicitly as `1.`, `2.`, and so on within
each requirement. Cross-reference the second criterion under Requirement 3 as
`3.2`. Do not use repeated `1.` markers or literal `3.2.` list markers.

Never renumber a surviving requirement or criterion when updating a pack. Append
new identifiers, and mark removed behavior as superseded when retaining the old
identifier is necessary for traceability.

Classify open questions as blocking or non-blocking. Resolve blocking questions
before producing design or executable tasks. For a non-blocking question, record
the assumed default and the consequence if it proves false.

## Quality gate

- Separate required behavior from implementation choices.
- Define ambiguous domain terms once and use them consistently.
- State meaningful exclusions to prevent scope creep.
- Ensure every acceptance criterion has a plausible verification method.
- Avoid duplicate or contradictory criteria.
