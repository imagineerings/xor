# Design: Editor Experience

## Architecture

Register game commands and project-panel metadata through existing Zed command, workspace, project-panel, task, and debugger integration points only when project detection succeeds. Supported run/debug workflows execute through Zed-owned runtime services; unsupported runtime behavior remains explicit rather than launching Godot. Migrated Comfy/world-model authoring affordances are recreated as native Zed commands and metadata instead of compatibility labels.

## Components

- Existing command palette and menu action registration.
- Existing project-panel metadata and worktree state.
- Existing task, DAP, debugger UI, and diagnostics owners for native run/debug behavior.

## Correctness Properties

### Property 1: Scoped Commands

_For any_ non-Godot workspace, Godot commands SHALL remain unregistered.

**Validates: Requirement 1.2**

### Property 2: Native Zed Commands

_For any_ migrated authoring command, Zed SHALL expose a native `zed_game.*` command id rather than a Comfy compatibility route.

**Validates: Requirement 1.3**

### Property 3: Native Execution

_For any_ supported run/debug operation, Zed SHALL execute through Zed-owned task/runtime/debugger services; unavailable native behavior SHALL remain unresolved or excluded.

**Validates: Requirement 3.1, 3.2**

### D-NATIVE: Native editor path

Commands, project metadata, UI state, task execution, debugging, diagnostics, cancellation, and cleanup remain inside existing Zed owners. Godot-compatible inputs are translated at file or protocol boundaries, and hermetic validation denies any Godot process or library.

**Validates: Requirement 9.1, 9.2, 9.3, 9.4, 9.5**


## Audit traceability reconciliation

### D-TRACE: Preserve legacy design while exposing complete criterion coverage

This reconciliation table preserves the existing design decisions and IDs while making every acceptance criterion visible to the current feature-spec validator. Capability-level gaps and ownership corrections are defined by the Godot full-port coverage catalog.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 1.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 1.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 3.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 3.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
