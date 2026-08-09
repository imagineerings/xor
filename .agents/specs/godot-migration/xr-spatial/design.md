# Design: XR and Spatial Tooling

## Architecture

Use existing Sim project, media, preview, docs, settings, and platform owners for XR action maps, camera/spatial metadata, docs, and preview routes. Keep unapproved runtime execution explicitly excluded. Godot-origin XR symbols are source metadata for Sim authoring, not runtime adapters or delegated execution targets.

## Components

- Existing project/media/preview records for imported spatial metadata.
- Existing docs/settings/platform diagnostics for supported and excluded outcomes.

## Correctness Properties

### Property 1: XR Exclusion

_For any_ XR runtime feature, Sim SHALL not classify it as a native runtime adapter.

**Validates: Requirement 1.1, 1.2**

### Property 2: Native Spatial Metadata

_For any_ spatial asset metadata, Sim SHALL expose docs symbols and preview routing through native Sim records.

**Validates: Requirement 2.1**

### D-NATIVE: Native spatial path

Imported compatibility data terminates at existing Sim owners. Supported preview and metadata lifecycles remain Sim-owned; excluded runtime behavior has no external fallback. Hermetic tests remove Godot and inspect loaders, processes, packages, and XR runtime selection.

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
