# Implementation Plan: Game Formats and Assets

## Dependency Gates

- **Primary wave**: W2 Godot compatibility substrate; W5 Generation outputs and asset pipelines for generated mesh assets
- **Prerequisite gates**: G0 Spec consistency, G1 Boundary policy, G2 Shared Godot metadata
- **Generated asset gates**: G3 Shared world-model foundations, G6 Provenance
- **Dependency gate**: G7 Dependency review before adding importers, codecs, or native geometry dependencies

## Tasks

- [ ] 1. Implement Godot format parsing and generated asset registration
  - Classify files, parse text resources, link `.import` metadata, and register generated mesh assets.
  - _Requirements: 1.1, 2.1, 3.1_
  - _writes: crates/godot/src/formats.rs, crates/godot/src/imports.rs, crates/godot/src/generated_assets.rs_
