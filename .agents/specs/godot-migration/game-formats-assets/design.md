# Design: Game Formats and Assets

## Architecture

Use parsers at existing project, worktree, filesystem, preview, media, and artifact owners for text project, scene, resource, and import metadata. Heavyweight formats require an approved Zed-native importer or an explicit unsupported/decision-required outcome; Godot is never used as an external tool.

Game format and generated asset handling is implemented as native Zed functionality. Godot-origin text resources are parsed into records at existing project/worktree owners without executing scripts, `.import` metadata is linked by Zed-owned import records, and generated mesh assets register through existing artifact/media provenance owners rather than compatibility labels or pass-through import hooks.

## Components

- Existing project/worktree format classification and source indexing.
- Existing filesystem/import cache and dependency ownership.
- Existing preview/media/artifact records for imported and generated outputs.

## Correctness Properties

### Property 1: Lightweight Parsing

_For any_ Godot text resource, parsing SHALL extract references without executing resource scripts.

**Validates: Requirement 1.1**

### Property 2: Generated Asset Provenance

_For any_ generated mesh asset, registration SHALL require provenance metadata.

**Validates: Requirement 3.1**

### D-NATIVE: Native format and import path

Godot-compatible bytes and metadata terminate at Zed-owned parsers/importers. Their outputs, caches, dependency graph, diagnostics, cancellation, recovery, and cleanup remain in existing Zed owners. Unsupported heavyweight formats do not fall back to Godot.

**Validates: Requirement 9.1, 9.2, 9.3, 9.4, 9.5**


## Audit traceability reconciliation

### D-TRACE: Preserve legacy design while exposing complete criterion coverage

This reconciliation table preserves the existing design decisions and IDs while making every acceptance criterion visible to the current feature-spec validator. Capability-level gaps and ownership corrections are defined by the Godot full-port coverage catalog.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 3.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
