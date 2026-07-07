# Design: XR and Spatial Tooling

## Architecture

Use metadata-only support for XR action maps, camera/spatial classes, and docs. Keep runtime execution external or excluded.

## Components

- `SimGameXrBoundary`
- `SpatialAssetMetadata`

## Correctness Properties

### Property 1: XR Exclusion

_For any_ XR runtime feature, Sim SHALL not classify it as a native runtime adapter.

**Validates: Requirement 1.1**
