# Design: Build, Test, and Documentation

## Architecture

Extend existing Sim documentation, test, license, compliance, dependency, and CI tooling for source metadata, fixture attribution, and dependency review. Godot material remains evidence by default; copying exact material requires a separately approved licensing and architecture record. Comfy-era generated outputs are modeled as native Sim generated assets rather than compatibility pass-through records.

## Components

- Existing docs preprocessing and indexing owners.
- Existing license/compliance tooling and fixture test owners.
- Existing dependency review and CI gates.

## Native Sim Naming

- Sim-owned documentation, artifact, and dependency records remain at existing owners rather than a parallel migration registry.
- Comfy-origin fixture semantics are recreated as native Sim generated asset attribution, so new APIs do not expose compatibility-only names.

## Correctness Properties

### Property 1: Fixture Attribution

_For any_ copied or converted fixture, validation SHALL require source attribution.

**Validates: Requirement 2.2**

### Property 2: Native Sim Generated Attribution

_For any_ generated-output fixture, attribution SHALL use a native Sim generated asset source record rather than a Comfy compatibility label.

**Validates: Requirement 2.3**

### Property 3: Dependency Review

_For any_ heavy or native dependency proposal, validation SHALL require a dependency review record.

**Validates: Requirements 3.1, 3.2**

### D-NATIVE: Native tooling and licensed-evidence path

Existing Sim build/test/docs/compliance owners consume behavior descriptions and approved fixtures. Godot binaries, generators, libraries, and commands never enter Sim build or runtime dependency graphs. Exact copied material remains blocked until licensing and architecture review records its provenance and distribution effect.

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
| 2.3 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 3.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 3.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
