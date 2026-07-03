# Design: Editor Experience

## Architecture

Register Godot commands and project-panel metadata only when Godot project detection succeeds. Run/debug workflows use existing Baymax task and debugger surfaces.

## Components

- `GodotCommandProvider`
- `GodotProjectPanelMetadata`
- `GodotRunDebugTemplate`

## Correctness Properties

### Property 1: Scoped Commands

_For any_ non-Godot workspace, Godot commands SHALL remain unregistered.

**Validates: Requirement 1.2**

### Property 2: External Execution

_For any_ run/debug operation, Baymax SHALL invoke configured external Godot tooling rather than embedding the Godot runtime.

**Validates: Requirement 3.1**
