# Design: Unified Authoring App

## Architecture

Compose existing workspace, project, editor, inspector, media, task, agent, and graph surfaces around Godot-compatible import metadata and Zed-native runtime state. Add no parallel Godot-specific workspace or runtime.

Unified authoring is implemented as native Zed functionality. Workspace items, routes, previews, diagnostics, and generated artifact panels reuse records at their existing owners and compose Zed graph, preview, generated asset, and world-model provenance data rather than exposing compatibility labels or pass-through workflow state.

## Components

- Existing workspace/project/editor routing and item models.
- Existing inspector, preview, media, artifact, task, and diagnostics owners.
- Existing app registration integration point where a distinct product surface is approved.

## Correctness Properties

### Property 1: Unified Routing

_For any_ authoring item, the app SHALL route it to the appropriate editor, preview, inspector, or task view based on typed metadata.

**Validates: Requirement 1.2**

### Property 2: Safe Preview

_For any_ world-model preview request, execution SHALL require worker diagnostics and artifact provenance support.

**Validates: Requirement 2.1, 2.2**

### D-NATIVE: Native unified authoring path

Godot-compatible project/file concepts terminate at import and presentation boundaries. Existing Zed owners retain UI, storage, preview/runtime execution, tasks, provenance, cancellation, recovery, persistence, and cleanup. Unsupported runtime behavior is explicit and no action delegates to Godot.

**Validates: Requirement 9.1, 9.2, 9.3, 9.4, 9.5**


## Audit traceability reconciliation

### D-TRACE: Preserve legacy design while exposing complete criterion coverage

This reconciliation table preserves the existing design decisions and IDs while making every acceptance criterion visible to the current feature-spec validator. Capability-level gaps and ownership corrections are defined by the Godot full-port coverage catalog.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 1.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
