# Phase 4: Task Execution

## Execute Phase

Implement specific tasks from the feature specification with precision and focus.

### Prerequisites

**ALWAYS read spec files first**:
- `.agent/specs/{feature-name}/requirements.md`
- `.agent/specs/{feature-name}/design.md`
- `.agent/specs/{feature-name}/tasks.md`

Never execute tasks without understanding full context.

### Execution Process

1. **Task Selection**
   - If task number/description provided: Focus on that specific task
   - If no task specified: Review task list and recommend next logical task
   - If task has sub-tasks: Always complete sub-tasks first

2. **Quick Start-Gate Consistency Check**
   - Confirm the task is still valid to begin.
   - Verify prerequisites and dependency wave placement.
   - Check for obvious `_writes:` conflicts with parallel work.
   - Scan `requirements.md`, `design.md`, and `tasks.md` for obvious
     contradictions or missing requirement references that would block the task.
   - If the check reveals blocking ambiguity, stop and ask for clarification or
     update the spec before implementation.

3. **Implementation**
   - **ONE task at a time** - Never implement multiple without approval
   - **Minimal code** - Write only what's necessary for current task
   - **Follow the design** - Adhere to architecture decisions
   - **Verify requirements** - Ensure implementation meets specifications

4. **Full Completion-Gate Consistency Pass**
   - After implementation and validation, run the full pass before marking the
     task complete.
   - Tighten start, validation, handoff, and completion gates based on the
     actual validation performed.
   - Update dependency waves if implementation changed ordering, prerequisites,
     or parallel safety for later tasks.
   - Ensure requirements, design, and tasks still agree with the delivered
     behavior, including requirement references, design properties, task
     reads/writes, and done conditions.
   - Update spec documents before marking the task complete.

5. **Completion Protocol**
   - Once task complete and the full completion-gate pass is done, STOP and
     inform user
   - DO NOT proceed to next task automatically
   - Wait for user review and approval
   - Only run tests if explicitly requested

### Efficiency Principles

- **Parallel operations**: Execute independent operations simultaneously
- **Batch edits**: Use MultiEdit for multiple changes to same file
- **Minimize steps**: Complete tasks in fewest operations
- **Check your work**: Verify implementation meets requirements

### Response Patterns

**For implementation requests**:
1. Read relevant spec files
2. Identify the specific task
3. Run the quick start-gate consistency check
4. Implement with minimal code
5. Validate and run the full completion-gate consistency pass
6. Stop and await review

**For information requests**:
- Answer directly without starting implementation
- Examples: "What's the next task?", "What tasks are remaining?"

### Key Behaviors

- Be decisive and precise
- Focus intensely on single requested task
- Communicate progress clearly
- Never assume user wants multiple tasks done
- Respect the iterative review process
