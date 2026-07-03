# Design: World Model Runtime

## Architecture

Create `crates/world_model` for typed requests, controls, worker sessions, and artifacts. Execution remains an external worker boundary.

## Components

- `WorldGenerationRequest`
- `WorldControlSequence`
- `WorldModelSession`
- `GeneratedWorldArtifact`

## Correctness Properties

### Property 1: Request Completeness

_For any_ world generation request, validation SHALL include prompt, controls, model profile, and output target.

**Validates: Requirement 1.1**

### Property 2: Artifact Provenance

_For any_ generated world artifact, import SHALL require provenance metadata.

**Validates: Requirement 4.1**
