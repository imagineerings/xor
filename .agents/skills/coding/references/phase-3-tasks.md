# Phase 3: Implementation Task List

## Tasks Phase

Convert approved design into actionable, test-driven implementation tasks.

### Prerequisites

- Ensure `design.md` exists and is approved
- Requirements and design provide context for tasks

### Task Generation Instructions

**Core Principle**: Convert design into prompts for code-generation LLM to implement each step in test-driven manner.

**Focus**:
- Incremental progress with early testing
- Build on previous tasks - no orphaned code
- ONLY tasks involving writing, modifying, or testing code
- No big jumps in complexity

**Exclude**:
- User acceptance testing or feedback gathering
- Deployment to production/staging
- Performance metrics gathering
- Running application for manual testing (but OK to write automated end-to-end tests)
- User training or documentation creation
- Business process changes
- Marketing or communication activities

### Task Format

Create `.agent/specs/{feature-name}/tasks.md` with:

```markdown
# Implementation Plan: [Feature Name]

## Overview

[Brief description of the implementation approach. Explain the dependency
waves, task order, parallel-safe groups, and key decisions about how work is
structured.]

## Gates

- Start gate: [What must be true before implementation starts]
- Validation gate: [Commands, tests, or observable checks required before
  completion]
- Handoff gate: [What evidence/status must be handed to the next agent or
  reviewer]
- Completion gate: [What makes this task safe to mark complete]

## Dependency Waves

- Wave 1: [Tasks that can run first; note which can run in parallel]
- Wave 2: [Tasks that depend on Wave 1]
- Wave N: [Continue as needed]

## Tasks

- [ ] 1. Set up project structure and core interfaces
  - Create directory structure for models, services, repositories
  - Define interfaces that establish system boundaries
  - _Requirements: 1.1_
  - _writes: src/types/index.ts, src/interfaces/repository.ts_

- [ ] 2. Implement data models and validation
  - [ ] 2.1 Create core data model interfaces and types
    - Write TypeScript interfaces for all data models
    - Implement validation functions for data integrity
    - _Requirements: 2.1, 3.3, 1.2_
    - _writes: src/models/user.ts, src/models/index.ts_

  - [ ] 2.2 Implement User model with validation
    - Write User class with validation methods
    - Create unit tests for User model validation
    - _Requirements: 1.2_
    - _writes: src/models/user.ts, src/models/__tests__/user.test.ts_

- [ ] 3. Create storage mechanism
  - [ ] 3.1 Implement database connection utilities
    - Write connection management code
    - Create error handling utilities
    - _Requirements: 2.1, 3.3_
    - _writes: src/db/connection.ts, src/db/errors.ts_

[Additional tasks...]

## Notes

- Each task includes `_writes:` manifest for file conflict detection
- [Any additional implementation notes or constraints]
```

### Task Requirements

**Structure**:
- Maximum two-level hierarchy (tasks and sub-tasks)
- Use decimal notation for sub-tasks (1.1, 1.2, 2.1)
- Each item must be a checkbox
- Simple structure preferred

**Each Task Must Include**:
- Clear objective involving code (writing, modifying, testing)
- Additional info as sub-bullets
- Specific requirement references (granular sub-requirements, not just user stories)
- `_writes:` manifest for expected file edits when known
- Dependency wave or prerequisite note when ordering matters
- Validation gate for the task, especially the command or check that proves it
  works

**Quality Standards**:
- Discrete, manageable coding steps
- Incremental builds on previous steps
- Test-driven development prioritized
- Covers all design aspects implementable through code
- Validates core functionality early
- Requirements, design properties, and task references agree before approval

### Consistency Pass Before Approval

Before asking for task approval:

1. Tighten gates so start, validation, handoff, and completion checks are
   concrete and actionable.
2. Update dependency waves so task order and parallel-safe groups reflect the
   current design, requirements, and `_writes:` manifests.
3. Check that `requirements.md`, `design.md`, and `tasks.md` agree:
   - every `_Requirements:` reference exists,
   - every design property validates one or more requirements,
   - every design component has implementation coverage or an explicit reason it
     needs no code task,
   - no task asks for behavior contradicted by requirements or design.

### Review & Iteration

3. **Ask for Approval**
   - After creating/updating tasks
   - Ask: "Do the tasks look good?"
     - Make modifications if user requests changes
   - Continue feedback-revision cycle until explicit approval
   - **Stop once approved - do not proceed to implementation**

### Completion

**Important**: This workflow is ONLY for creating planning artifacts.

- DO NOT implement the feature as part of this workflow
- Inform user they can execute tasks by:
  - Opening tasks.md
  - Clicking "Start task" next to items
  - Or asking you to execute specific tasks
