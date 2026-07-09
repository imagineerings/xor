# Design: Agentic Game Tools

## Architecture

Expose graph editing, world generation, and mesh generation through existing Sim agent tool registration. Tools produce diffs and typed requests, not direct unvalidated filesystem mutations.

Agentic game tooling is implemented as native Sim functionality. The tools use `SimGame*`, `SimWorld*`, and `SimMesh*` records that convert into `world_model` graph, world-generation, mesh-generation, and provenance types; they do not expose Comfy compatibility labels or pass requests through to Comfy workflows.

## Components

- `SimGameGraphTool`
- `SimWorldGenerationTool`
- `SimMeshGenerationTool`

## Correctness Properties

### Property 1: Validated Agent Graph Edits

_For any_ agent graph edit, the graph validator SHALL run before changes are committed.

**Validates: Requirement 1.1**

### Property 2: Typed Generation

_For any_ agent generation request, the tool SHALL produce typed requests and provenance-aware outputs.

**Validates: Requirement 2.1, 2.2**
