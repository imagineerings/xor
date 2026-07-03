# Design: Platform and Export

## Architecture

Use existing Baymax task execution to invoke configured Godot CLI commands for run/export. Store only preset metadata and diagnostics.

## Components

- `GodotExecutableSettings`
- `GodotExportPresetParser`
- `GodotExportTaskTemplate`

## Correctness Properties

### Property 1: External Export

_For any_ export request, Baymax SHALL invoke configured external tooling rather than migrate Godot platform templates.

**Validates: Requirement 1.1, 2.1**
