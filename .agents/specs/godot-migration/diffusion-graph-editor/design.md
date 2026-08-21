# Design: Diffusion Graph Editor

## Architecture

Use GPUI/app infrastructure for the editor surface and `crates/world_model` graph primitives for validation and execution planning.
The editor state is a native Zed feature: `DiffusionGraphEditorState` owns the
graph, validation state, artifact outputs, and execution plan metadata directly
instead of presenting Comfy graph compatibility labels or pass-through state.

## Components

- `DiffusionGraph`
- `GraphNode`
- `GraphPort`
- `GraphValidationReport`
- `DiffusionGraphEditorState`

## Correctness Properties

### Property 1: Validated Execution

_For any_ graph execution request, execution SHALL be blocked until validation succeeds.

**Validates: Requirement 1.1, 3.1**


## Audit traceability reconciliation

### D-TRACE: Preserve legacy design while exposing complete criterion coverage

This reconciliation table preserves the existing design decisions and IDs while making every acceptance criterion visible to the current feature-spec validator. Capability-level gaps and ownership corrections are defined by the Godot full-port coverage catalog.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 3.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
