# Requirements: Build, Test, and Documentation

## Introduction

Sim should reuse existing build/test infrastructure while adding native Sim docs ingestion, fixture attribution, compatibility metadata, and dependency review gates for Godot/world-model source material.

### Requirement 1: Build Boundary

#### Acceptance Criteria

1.1 IF a build feature duplicates Sim build or CI systems THEN THE system SHALL not port it.

### Requirement 2: Docs and Fixtures

#### Acceptance Criteria

2.1 WHEN Godot docs or class references are ingested THEN THE system SHALL preserve source metadata.
2.2 WHEN fixtures are copied or converted THEN THE system SHALL preserve attribution.
2.3 WHEN source material comes from Comfy-era generated outputs THEN THE system SHALL represent it as native Sim generated asset metadata, not as a thin Comfy compatibility label or pass-through.

### Requirement 3: Dependency Review

#### Acceptance Criteria

3.1 WHEN a heavy, native, vendored, codec, model, or mesh dependency is proposed THEN THE system SHALL require review.
3.2 WHEN dependency review records are stored THEN THE system SHALL use native Sim review records with license, maintenance, security, binary-size, and platform-impact fields.
