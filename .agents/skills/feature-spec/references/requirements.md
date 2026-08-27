# Requirements Phase

Create `.agents/specs/{feature-name}/requirements.md` from the user's request
and repository context. Generate a useful first draft before asking
non-blocking questions.

## Structure

```markdown
# Requirements: [Feature]

## Problem

[Who is affected, what problem exists, and why it matters.]

## Scope

### In scope

- [Behavior included in this feature]

### Out of scope

- [Adjacent behavior deliberately excluded]

## Glossary

- **Term**: [Definition, only when domain vocabulary needs clarification]

## Requirements

### Requirement 1: [Observable capability]

**User story:** As a [role], I want [capability], so that [outcome].

#### Acceptance criteria

1. **1.1** WHEN [event] THEN THE [system] SHALL [observable response].
2. **1.2** IF [unwanted condition], THEN THE [system] SHALL [response].
3. **1.3** THE [system] SHALL [unconditional behavior].

## Constraints

- [Compatibility, security, accessibility, performance, or policy constraint]

## Open questions

- [Only unresolved decisions that materially affect the feature]
```

Omit empty optional sections such as the glossary or open questions.

## EARS patterns

Use the pattern that matches the behavior; do not force every criterion to use
a different pattern.

| Pattern | Form |
| --- | --- |
| Ubiquitous | `THE [system] SHALL [response]` |
| Event-driven | `WHEN [event] THEN THE [system] SHALL [response]` |
| State-driven | `WHILE [state] THE [system] SHALL [response]` |
| Optional feature | `WHERE [feature is included], THE [system] SHALL [response]` |
| Unwanted behavior | `IF [condition], THEN THE [system] SHALL [response]` |

Combine clauses when necessary, for example: `WHILE [state], WHEN [event], THE
[system] SHALL [response]`.

## Quality check

- Keep acceptance criteria observable and independently verifiable.
- Cover relevant success, boundary, failure, and state-transition behavior.
- Do not prescribe components, types, or algorithms unless they are genuine
  constraints.
- Use explicit criterion IDs in the form `requirement.acceptance`, such as
  `2.3`; never renumber approved IDs casually.
- Record exclusions so the design and tasks do not expand scope accidentally.

If the user requested a full pack, continue to the design phase. Otherwise,
stop after summarizing assumptions and material open questions.
