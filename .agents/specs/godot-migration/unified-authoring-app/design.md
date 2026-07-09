# Design: Unified Authoring App

## Architecture

Add a Sim app registry entry for game authoring that composes existing project, media, task, agent, and graph surfaces around Godot/world-model metadata.

Unified authoring is implemented as native Sim functionality. Workspace items, routes, previews, diagnostics, and generated artifact panels use `SimAuthoring*` and `SimGameAuthoring*` records that compose Sim graph, preview, generated asset, and world-model provenance data rather than exposing Comfy compatibility labels or pass-through workflow state.

## Components

- `SimGameAuthoringApp`
- `SimAuthoringItem`
- `SimAuthoringPreviewRoute`
- `SimGeneratedAssetRecord`

## Correctness Properties

### Property 1: Unified Routing

_For any_ authoring item, the app SHALL route it to the appropriate editor, preview, inspector, or task view based on typed metadata.

**Validates: Requirement 1.2**

### Property 2: Safe Preview

_For any_ world-model preview request, execution SHALL require worker diagnostics and artifact provenance support.

**Validates: Requirement 2.1, 2.2**
