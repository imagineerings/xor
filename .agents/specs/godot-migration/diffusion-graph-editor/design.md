# Design: Diffusion Graph Editor

## Architecture

Use GPUI/app infrastructure for the editor surface and `crates/world_model` graph primitives for validation and execution planning.

## Components

- `DiffusionGraph`
- `GraphNode`
- `GraphPort`
- `GraphValidationReport`

## Correctness Properties

### Property 1: Validated Execution

_For any_ graph execution request, execution SHALL be blocked until validation succeeds.

**Validates: Requirement 1.1, 3.1**
