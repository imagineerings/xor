# Requirements: Build, Test, and Documentation

## Introduction

Zed should reuse existing build/test infrastructure while adding native Zed docs ingestion, fixture attribution, compatibility metadata, and dependency review gates for Godot/world-model source material.

### Requirement 1: Build Boundary

#### Acceptance Criteria

1. **1.1** IF a build feature duplicates Zed build or CI systems THEN THE system SHALL not port it.

### Requirement 2: Docs and Fixtures

#### Acceptance Criteria

1. **2.1** WHEN Godot docs or class references are ingested THEN THE system SHALL preserve source metadata.
2. **2.2** WHEN fixtures are copied or converted THEN THE system SHALL preserve attribution.
3. **2.3** WHEN source material comes from Comfy-era generated outputs THEN THE system SHALL represent it as native Zed generated asset metadata, not as a thin Comfy compatibility label or pass-through.

### Requirement 3: Dependency Review

#### Acceptance Criteria

1. **3.1** WHEN a heavy, native, vendored, codec, model, or mesh dependency is proposed THEN THE system SHALL require review.
2. **3.2** WHEN dependency review records are stored THEN THE system SHALL use native Zed review records with license, maintenance, security, binary-size, and platform-impact fields.

### Requirement 9: Native Zed Ownership and Source Reuse

#### Acceptance Criteria

1. **9.1** Godot build, test, CI, documentation, localization, generator, and release infrastructure SHALL be evidence only unless a connected Zed behavior requires a native counterpart owned by existing Zed tooling.
2. **9.2** THE migration SHALL NOT build, bundle, link, invoke, or ship Godot tooling as a prerequisite for Zed tests, docs, generation, packaging, or runtime behavior.
3. **9.3** IF Godot source code, generated code, vendor patches, fixtures, docs, assets, fonts, icons, or translations would be copied THEN the exact material SHALL remain blocked pending separate licensing and architecture approval.
4. **9.4** Dependency review SHALL record license, provenance, linkage, transitive/runtime effects, distribution obligations, maintenance, security, binary size, and platform impact; declarations or unused dependencies SHALL NOT count as support.
5. **9.5** Validation SHALL run on a machine without Godot and prove Zed build/test/docs/package/runtime paths do not discover or depend on Godot.
