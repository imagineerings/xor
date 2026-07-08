# Implementation Plan: Model Serving and Packaging

## Overview

Add diagnostics and launcher models in W2, then defer real local/remote worker launch hardening to W6 after safety and dependency gates are in place.

## Gates

- Start gate: G0 spec consistency, G3 shared world-model foundations, and applicable G8 Comfy model/runtime alignment decisions are satisfied.
- Validation gate: local environment, checkpoint, GPU, persistent session, remote worker, and explicit-download tests pass.
- Handoff gate: missing Python, packages, checkpoints, GPU, disk, endpoint, auth, quota, and model downloads produce stable diagnostic codes.
- Completion gate: G4 worker safety is satisfied before any real model worker starts, and heavy setup or packaging dependencies require G7 dependency review.

## Dependency Waves

- W2 Value-first world-model serving substrate: diagnostics and launcher models produce G4 worker safety.
- W6 Comfy provider, extension, and packaging hardening: real worker launch, remote execution, and packaging paths depend on G4 and G7.

## Tasks

- [x] 1. Add serving diagnostics and worker launcher models
  - Validate local Python/GPU/checkpoint setup, persistent session configuration, and remote worker metadata.
  - Keep launch readiness as native Sim diagnostics and model records only; block downloads and heavy dependencies until explicit approval/dependency review without starting worker processes.
  - _Requirements: 1.1, 1.2, 2.1, 2.2, 3.1_
  - _writes: crates/world_model/src/serving.rs, crates/world_model/src/worker_launcher.rs, crates/world_model/src/serving_tests.rs_
