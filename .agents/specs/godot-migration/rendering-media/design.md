# Design: Rendering and Media

## Architecture

Reuse Zed media, image, shader-language, and preview infrastructure. Treat world-model outputs as generated artifacts with provenance.

Rendering and media support is a native Zed feature. Godot media files are classified into Zed preview decisions with explicit unsupported reasons, while world-model outputs route through `GeneratedMedia*` records that require provenance metadata instead of delegating to Godot render/audio/text servers or Comfy preview pass-throughs.

## Components

- Existing media/image/component preview classification and routing.
- Existing GPUI/wgpu render surfaces for approved native preview or rendering behavior.
- Existing media diagnostics and provenance owners for generated artifacts.

## Correctness Properties

### Property 1: Render Backend Exclusion

_For any_ Godot render backend feature, the boundary policy SHALL classify it as excluded.

**Validates: Requirement 1.1**

### Property 2: Provenance on Generated Media

_For any_ imported generated media file, preview routing SHALL require provenance metadata.

**Validates: Requirement 3.1**

### D-NATIVE: Native rendering and media path

Godot-compatible files terminate at existing Zed decoders, media records, and preview/render surfaces. Zed owns decoded storage, GPU/media execution, UI, device loss, cancellation, errors, and cleanup. Runtime behavior without a native owner remains excluded or decision-blocked.

**Validates: Requirement 9.1, 9.2, 9.3, 9.4, 9.5**


## Audit traceability reconciliation

### D-TRACE: Preserve legacy design while exposing complete criterion coverage

This reconciliation table preserves the existing design decisions and IDs while making every acceptance criterion visible to the current feature-spec validator. Capability-level gaps and ownership corrections are defined by the Godot full-port coverage catalog.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 3.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
