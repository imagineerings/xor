# Design: XR and Spatial Tooling

## Architecture

Use native Sim metadata-only support for XR action maps, camera/spatial classes, docs, and preview routes. Keep runtime execution excluded. Godot-origin XR symbols are source metadata for Sim authoring, not runtime adapters.

## Components

- `SimGameXrBoundary`
- `SpatialAssetMetadata`

## Correctness Properties

### Property 1: XR Exclusion

_For any_ XR runtime feature, Sim SHALL not classify it as a native runtime adapter.

**Validates: Requirement 1.1, 1.2**

### Property 2: Native Spatial Metadata

_For any_ spatial asset metadata, Sim SHALL expose docs symbols and preview routing through native Sim records.

**Validates: Requirement 2.1**
