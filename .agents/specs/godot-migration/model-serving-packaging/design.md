# Design: Model Serving and Packaging

## Architecture

Use `crates/world_model` serving diagnostics and launcher traits to describe local Python workers, persistent sessions, and remote worker fallback.

## Components

- `ServingDiagnostics`
- `WorldModelWorkerLauncher`
- `RemoteWorkerConfig`
- `ModelAssetPolicy`

`WorldModelWorkerLauncher` is a native Zed validation boundary. It models local,
persistent, and remote worker launch readiness from supplied environment
metadata, emits stable serving diagnostics for missing Python/packages,
checkpoints, GPU, disk, endpoint, authentication, capabilities, quota, downloads,
and dependency review, and does not start worker processes or download assets.

## Correctness Properties

### Property 1: No Silent Downloads

_For any_ missing model asset or dependency, diagnostics SHALL report the missing prerequisite and require explicit user action.

**Validates: Requirement 1.2, 3.1**

### Property 2: Worker Mode Validation

_For any_ configured worker mode, validation SHALL verify required local or remote settings before launch.

**Validates: Requirement 1.1, 2.1, 2.2**


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
| 3.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
