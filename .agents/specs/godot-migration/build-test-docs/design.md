# Design: Build, Test, and Documentation

## Architecture

Add documentation ingestion, fixture attribution, and dependency review helpers while leaving Sim build/test systems authoritative. Comfy-era generated outputs are modeled as native Sim generated assets rather than compatibility pass-through records.

## Components

- `SimGameDocsIngestion`
- `FixtureAttributionValidator`
- `DependencyReviewGate`

## Native Sim Naming

- Source records exposed by `sim_game` use Sim-owned names such as `SimGameDocsRecord`, `SimGeneratedAsset`, and `SimGameDependencyReviewRecord`.
- Comfy-origin fixture semantics are recreated as native Sim generated asset attribution, so new APIs do not expose `Comfy*` compatibility names.

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
