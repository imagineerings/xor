# Implementation Plan: Mesh Generation Pipeline

## Overview

Model mesh requests and artifacts in W4 after shared world-model request and provenance primitives exist. Real generation backends remain blocked by dependency review.

## Gates

- Start gate: G0 spec consistency, G3 shared world-model foundations, and G6 provenance are satisfied.
- Validation gate: mesh request, artifact metadata, preview/export, and provenance tests pass.
- Handoff gate: unsupported backends and missing preview/export formats produce stable diagnostics.
- Completion gate: real mesh-generation backends or native preview dependencies require G7 dependency review, and Comfy-backed 3D node behavior references G8 Comfy harness alignment.

## Dependency Waves

- W4 Generation outputs and asset pipelines: mesh request and artifact models depend on W2 world-model foundations and G6 provenance.

## Tasks

- [ ] 1. Add mesh request and generated artifact models
  - Model textured mesh requests, backend descriptors, preview metadata, export targets, and provenance.
  - _Requirements: 1.1, 2.1_
  - _writes: crates/world_model/src/mesh.rs, crates/world_model/src/mesh_artifact.rs, crates/world_model/src/mesh_tests.rs_
