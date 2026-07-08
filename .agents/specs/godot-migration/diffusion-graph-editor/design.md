# Design: Diffusion Graph Editor

## Architecture

Use GPUI/app infrastructure for the editor surface and `crates/world_model` graph primitives for validation and execution planning.
The editor state is a native Sim feature: `DiffusionGraphEditorState` owns the
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
