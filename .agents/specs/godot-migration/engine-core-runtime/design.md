# Design: Engine Core and Runtime

## Architecture

Add `crates/sim_game` Sim game metadata primitives for project detection, resource references, parse diagnostics, and boundary decisions. Do not embed Godot runtime services.

## Components

- `SimGameProjectDescriptor`: project root, project file, display name, engine version, and feature flags.
- `SimGameResourceIndex`: scene/resource references and parse state.
- `RuntimeBoundaryPolicy`: blocks scene-tree, OS, rendering, input, and object runtime ports.

## Correctness Properties

### Property 1: Runtime Boundary

_For any_ runtime-only Godot subsystem, the boundary policy SHALL not classify it as a Sim runtime adapter.

**Validates: Requirement 1.1**

### Property 2: Recoverable Parsing

_For any_ invalid project or resource file, parsing SHALL return diagnostics rather than panic.

**Validates: Requirement 2.2, 3.2**
