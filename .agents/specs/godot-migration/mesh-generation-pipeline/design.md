# Design: Mesh Generation Pipeline

## Architecture

Represent mesh generation as typed requests and artifacts in `crates/world_model`, with previews routed through existing media/project infrastructure.
The pipeline is a native Zed feature: mesh requests, backend choices, export
formats, texture options, and generated artifact metadata are represented by
Zed-owned `Mesh*` world-model records rather than Comfy labels or pass-through
backend state.

## Components

- `MeshGenerationRequest`
- `MeshBackendDescriptor`
- `GeneratedMeshArtifact`

## Correctness Properties

### Property 1: Mesh Provenance

_For any_ generated mesh artifact, registration SHALL require request and backend provenance.

**Validates: Requirement 2.1**


## Audit traceability reconciliation

### D-TRACE: Preserve legacy design while exposing complete criterion coverage

This reconciliation table preserves the existing design decisions and IDs while making every acceptance criterion visible to the current feature-spec validator. Capability-level gaps and ownership corrections are defined by the Godot full-port coverage catalog.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
