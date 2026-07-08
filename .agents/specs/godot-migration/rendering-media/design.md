# Design: Rendering and Media

## Architecture

Reuse Sim media, image, shader-language, and preview infrastructure. Treat world-model outputs as generated artifacts with provenance.

Rendering and media support is a native Sim feature. Godot media files are classified into Sim preview decisions with explicit unsupported reasons, while world-model outputs route through `GeneratedMedia*` records that require provenance metadata instead of delegating to Godot render/audio/text servers or Comfy preview pass-throughs.

## Components

- `SimGameMediaClassification`
- `SimGameMediaClassifier`
- `GeneratedMediaPreviewRoute`
- `GeneratedMediaPreviewDiagnostic`

## Correctness Properties

### Property 1: Render Backend Exclusion

_For any_ Godot render backend feature, the boundary policy SHALL classify it as excluded.

**Validates: Requirement 1.1**

### Property 2: Provenance on Generated Media

_For any_ imported generated media file, preview routing SHALL require provenance metadata.

**Validates: Requirement 3.1**
