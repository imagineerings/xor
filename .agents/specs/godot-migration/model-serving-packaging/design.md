# Design: Model Serving and Packaging

## Architecture

Use `crates/world_model` serving diagnostics and launcher traits to describe local Python workers, persistent sessions, and remote worker fallback.

## Components

- `ServingDiagnostics`
- `WorldModelWorkerLauncher`
- `RemoteWorkerConfig`
- `ModelAssetPolicy`

`WorldModelWorkerLauncher` is a native Sim validation boundary. It models local,
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
