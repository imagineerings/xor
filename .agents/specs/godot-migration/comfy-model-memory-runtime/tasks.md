# Implementation Plan: Comfy Model and Memory Runtime

## Overview

Implement the model runtime as catalog and policy primitives first, then connect validation and worker diagnostics. Model downloads and Python worker startup remain under `model-serving-packaging/`; sampler/scheduler and model-family execution semantics remain under `comfy-diffusion-world-model-runtime/`.

## Gates

- Start gate: G0 spec consistency, G3 shared world-model foundations, G4 worker safety, and G8 Comfy harness alignment are satisfied.
- Validation gate: folder catalog tests, metadata tests, runtime policy tests, and worker diagnostic integration tests pass.
- Handoff gate: unsupported model families and incompatible policies produce stable diagnostic codes.
- Completion gate: no task silently downloads models or installs heavy dependencies.

## Dependency Waves

- Global wave: W2 Value-first world-model serving substrate.
- Local Wave 1: Tasks 1-3 build catalog foundations.
- Local Wave 2: Tasks 4-5 add model family and runtime policy validation.
- Local Wave 3: Task 6 integrates resource release and diagnostics.

## Tasks

- [x] 1. Implement Comfy model folder registry
  - Register default Comfy model categories, allowed extensions, legacy folder mapping, and extra path config merge.
  - _Requirements: 1.1, 1.2, 1.3_
  - _writes: crates/world_model/src/comfy_model_folders.rs, crates/world_model/src/comfy_model_folders_tests.rs_

- [ ] 2. Implement model file catalog listing
  - Add recursive visible file search, path indexes, size/timestamp metadata, mtime cache invalidation, and safe path resolution.
  - _Requirements: 2.1_
  - _writes: crates/world_model/src/comfy_model_catalog.rs, crates/world_model/src/comfy_model_catalog_tests.rs_

- [ ] 3. Implement model preview and safetensors metadata reads
  - Add adjacent preview lookup and bounded safetensors header metadata extraction.
  - _Requirements: 2.2, 2.3_
  - _writes: crates/world_model/src/comfy_model_metadata.rs, crates/world_model/src/comfy_model_metadata_tests.rs_

- [ ] 4. Implement model family capability detection
  - Add model-family and capability records for Comfy-supported image, video, audio, 3D, adapter, segmentation, depth, and detection families.
  - _Requirements: 3.1, 3.2, 3.3_
  - _writes: crates/world_model/src/comfy_model_family.rs, crates/world_model/src/comfy_model_family_tests.rs_

- [ ] 5. Implement precision, quantization, device, and memory policy resolver
  - Parse runtime settings, quantization metadata, backend support, dynamic VRAM/offload options, and compatibility diagnostics.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 5.1, 5.2_
  - _writes: crates/world_model/src/comfy_runtime_policy.rs, crates/world_model/src/comfy_quantization.rs, crates/world_model/src/comfy_runtime_policy_tests.rs_

- [ ] 6. Implement model resource release bridge
  - Wire unload-models and free-memory intents to worker APIs and report success or diagnostic failure.
  - _Requirements: 5.1, 5.3_
  - _writes: crates/world_model/src/comfy_model_resources.rs, crates/world_model/src/comfy_model_resources_tests.rs_

## Notes

- Asset indexing for model files belongs to `comfy-asset-library/`.
- Sampler/scheduler behavior and model-family execution semantics belong to `comfy-diffusion-world-model-runtime/`.
- Process launching and package installation belong to `model-serving-packaging/`.
