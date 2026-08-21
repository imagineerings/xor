# Shared specification execution

Use this reference only after an implementation skill has established its
authorization boundary and selected executable task scope. It does not authorize
implementation by itself.

## Routing contract

| Request | Skill | Authorized stopping point |
| --- | --- | --- |
| `Build this feature` | `coding` | Complete local feature delivery |
| `Implement this existing specification` | `coding` | All remaining executable tasks |
| `Complete all remaining tasks` | `coding` | All remaining executable tasks |
| `Implement task 2.1` | `execute-spec-task` | Task 2.1 only |
| `Start the next unblocked task` | `execute-spec-task` | One dependency-satisfied leaf only |

An explicit task list is bounded to that list. A named epic is bounded to its
descendant leaves. A full-spec or all-remaining request is never reduced to one
task merely because the pack is already present.

## Establish executable scope

1. Read repository instructions and preserve unrelated work.
2. Read the complete requirements, design, and task plan before changing code.
3. Validate the pack with the canonical validator. Use canonical mode for new or
   migrated plans and compatibility auto-detection for existing legacy packs.
4. Reconcile task state with the code and evidence. Do not redo work that is
   already present and validated, but do not trust a checked box without evidence.
5. Select only the tasks authorized by the invoking skill:
   - `coding` selects every pending executable leaf in dependency order;
   - `execute-spec-task` selects the named leaves, or one next-unblocked leaf when
     the user named none.

Canonical epic checkboxes are grouping state, not implementation units. Legacy
top-level packets are executable compatibility units, and their `_blocked_by`
values refer to durable legacy IDs.

## Execute a selected leaf

For each selected leaf:

1. Confirm its referenced requirements, dependencies, expected reads and writes,
   and focused validation against the current repository.
2. Inspect the real code path and existing tests. Reuse established repository
   patterns and implement the smallest complete increment.
3. Handle required success, failure, recovery, security, accessibility, and trust
   boundaries. Propagate failures to the appropriate user-visible layer.
4. Run the declared validation and any newly necessary focused check permitted by
   repository instructions. Do not substitute an unrelated broad suite for the
   task's observable completion signal.
5. Reconcile the task's actual read/write paths and validation command. Update
   requirements or design when implementation discovery changes documented
   behavior or architecture; never silently redefine approved behavior.
6. Record `_Evidence:` and transition state according to the canonical task-state
   rules. A task whose required validation did not run remains `[~]`, not `[x]`.
7. Revalidate the affected spec pack before selecting more work.

When an assumption is wrong, stop dependent work, preserve completed history,
update the affected specification, and add corrective tasks instead of rewriting
a completed leaf. Ask before choosing a materially different behavior or
architecture when the invoking skill does not authorize that expansion.

## Finish the authorized scope

After the selected scope is implemented:

- confirm requirement, design, task, and verification traceability;
- run the smallest relevant formatting, static, and test checks;
- leave all unselected task state unchanged;
- report completed tasks, evidence, specification reconciliation, and risks.

`coding` continues until every requested executable leaf is complete or blocked.
`execute-spec-task` stops after its named task set or single next-unblocked leaf,
even when more work is available.

Local implementation completion is distinct from publishing, merging, releasing,
deploying, or mutating live systems. Perform those external actions only when the
user separately authorizes them.
