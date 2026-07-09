# Design: Game Formats and Assets

## Architecture

Use lightweight parsers for text project, scene, resource, and import metadata. Route heavyweight asset parsing to existing preview systems or external tools.

Game format and generated asset handling is implemented as native Sim functionality. Godot-origin text resources are parsed into `SimGame*` records without executing scripts, `.import` metadata is linked by Sim-owned import records, and generated mesh assets register through `SimGeneratedAsset*` records backed by `world_model` mesh/provenance metadata rather than Comfy compatibility labels or pass-through import hooks.

## Components

- `SimGameFormatClassifier`
- `SimGameTextResourceParser`
- `SimGameImportMetadataLinker`
- `SimGeneratedAssetRegistry`

## Correctness Properties

### Property 1: Lightweight Parsing

_For any_ Godot text resource, parsing SHALL extract references without executing resource scripts.

**Validates: Requirement 1.1**

### Property 2: Generated Asset Provenance

_For any_ generated mesh asset, registration SHALL require provenance metadata.

**Validates: Requirement 3.1**
