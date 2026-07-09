# Design: Physics and Navigation

## Architecture

Represent physics/navigation files and docs as native Sim metadata. Use native Sim fallback task records when configured. Godot physics and navigation servers are source concepts only and are not embedded.

## Components

- `SimulationBoundaryPolicy`
- `PhysicsNavigationMetadata`

## Correctness Properties

### Property 1: Runtime Exclusion

_For any_ Godot physics or navigation runtime feature, Sim SHALL not embed the runtime implementation.

**Validates: Requirement 1.1, 1.2**

### Property 2: Native Metadata and Fallbacks

_For any_ indexed physics or navigation source, Sim SHALL expose docs symbols and optional fallback task metadata without executing Godot servers.

**Validates: Requirement 2.1**
