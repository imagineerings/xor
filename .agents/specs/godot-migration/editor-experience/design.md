# Design: Editor Experience

## Architecture

Register game commands and project-panel metadata only when Godot project detection succeeds. Run/debug workflows use existing Sim task and debugger surfaces. Migrated Comfy/world-model authoring affordances are recreated as native Sim game commands and metadata instead of compatibility labels.

## Components

- `SimGameCommandProvider`
- `SimGameProjectPanelMetadata`
- `SimGameRunDebugTemplate`

## Correctness Properties

### Property 1: Scoped Commands

_For any_ non-Godot workspace, Godot commands SHALL remain unregistered.

**Validates: Requirement 1.2**

### Property 2: Native Sim Commands

_For any_ migrated authoring command, Sim SHALL expose a native `sim_game.*` command id rather than a Comfy compatibility route.

**Validates: Requirement 1.3**

### Property 3: External Execution

_For any_ run/debug operation, Sim SHALL invoke configured external Godot tooling rather than embedding the Godot runtime.

**Validates: Requirement 3.1**
