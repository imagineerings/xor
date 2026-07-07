# Design: Game Formats and Assets

## Architecture

Use lightweight parsers for text project, scene, resource, and import metadata. Route heavyweight asset parsing to existing preview systems or external tools.

## Components

- `SimGameFormatClassifier`
- `SimGameTextResourceParser`
- `GeneratedAssetRegistry`

## Correctness Properties

### Property 1: Lightweight Parsing

_For any_ Godot text resource, parsing SHALL extract references without executing resource scripts.

**Validates: Requirement 1.1**

### Property 2: Generated Asset Provenance

_For any_ generated mesh asset, registration SHALL require provenance metadata.

**Validates: Requirement 3.1**
