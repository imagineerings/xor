# Design: Platform and Export

## Architecture

Use existing Sim task execution to expose native Sim game run/export task templates. Godot export presets are parsed as source metadata only; Sim owns the executable settings, task records, diagnostics, and dependency-review boundary. Platform runtime templates are not migrated.

## Components

- `SimGameExecutableSettings`
- `SimGameExportPresetParser`
- `SimGameExportTaskTemplate`

## Correctness Properties

### Property 1: Native Sim Export Task

_For any_ export request, Sim SHALL create a native Sim game task template and SHALL NOT migrate Godot platform templates.

**Validates: Requirement 1.1, 1.2, 2.1**
