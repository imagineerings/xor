# Implementation Plan: Mesh Generation Pipeline

## Dependency Gates

- **Primary wave**: W5 Generation outputs and asset pipelines
- **Prerequisite gates**: G0 Spec consistency, G3 Shared world-model foundations, G6 Provenance
- **Dependency gate**: G7 Dependency review before adding real mesh-generation backends or native preview dependencies

## Tasks

- [ ] 1. Add mesh request and generated artifact models
  - Model textured mesh requests, backend descriptors, preview metadata, export targets, and provenance.
  - _Requirements: 1.1, 2.1_
  - _writes: crates/world_model/src/mesh.rs, crates/world_model/src/mesh_artifact.rs, crates/world_model/src/mesh_tests.rs_
