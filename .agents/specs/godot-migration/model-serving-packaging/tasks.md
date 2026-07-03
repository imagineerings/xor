# Implementation Plan: Model Serving and Packaging

## Dependency Gates

- **Primary wave**: W3 World-model serving substrate; W6 External execution hardening for real worker launch paths
- **Prerequisite gates**: G0 Spec consistency, G3 Shared world-model foundations
- **Gate produced/extended**: G4 Worker safety

## Tasks

- [ ] 1. Add serving diagnostics and worker launcher models
  - Validate local Python/GPU/checkpoint setup, persistent session configuration, and remote worker metadata.
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 3.1_
  - _writes: crates/world_model/src/serving.rs, crates/world_model/src/worker_launcher.rs, crates/world_model/src/serving_tests.rs_
