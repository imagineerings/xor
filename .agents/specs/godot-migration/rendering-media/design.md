# Design: Rendering and Media

## Architecture

Reuse Baymax media, image, shader-language, and preview infrastructure. Treat world-model outputs as generated artifacts with provenance.

## Components

- `BaymaxGameMediaClassifier`
- `GeneratedMediaPreviewRoute`
- `UnsupportedPreviewReason`

## Correctness Properties

### Property 1: Render Backend Exclusion

_For any_ Godot render backend feature, the boundary policy SHALL classify it as excluded.

**Validates: Requirement 1.1**

### Property 2: Provenance on Generated Media

_For any_ imported generated media file, preview routing SHALL require provenance metadata.

**Validates: Requirement 3.1**
