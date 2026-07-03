# Design: Mesh Generation Pipeline

## Architecture

Represent mesh generation as typed requests and artifacts in `crates/world_model`, with previews routed through existing media/project infrastructure.

## Components

- `MeshGenerationRequest`
- `MeshBackendDescriptor`
- `GeneratedMeshArtifact`

## Correctness Properties

### Property 1: Mesh Provenance

_For any_ generated mesh artifact, registration SHALL require request and backend provenance.

**Validates: Requirement 2.1**
