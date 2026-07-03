# Design: Physics and Navigation

## Architecture

Represent physics/navigation files and docs as metadata. Use external tasks for simulation fallback when configured.

## Components

- `SimulationBoundaryPolicy`
- `PhysicsNavigationMetadata`

## Correctness Properties

### Property 1: Runtime Exclusion

_For any_ Godot physics or navigation runtime feature, Baymax SHALL not embed the runtime implementation.

**Validates: Requirement 1.1**
