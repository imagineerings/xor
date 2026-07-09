# Implementation Plan: Comfy Model and Memory Runtime

## Overview

Implement the model runtime as catalog and policy primitives first, then connect validation and worker diagnostics. Model downloads and Python worker startup remain under `model-serving-packaging/`; sampler/scheduler and model-family execution semantics remain under `comfy-diffusion-world-model-runtime/`.

## Gates

- Start gate: G0 spec consistency, G3 shared world-model foundations, G4 worker safety, and G8 Comfy harness alignment, G9 Sim coverage are satisfied.
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
  - _CoverageOwner: .agents/specs/godot-migration/comfy-model-memory-runtime_

- [x] 2. Implement model file catalog listing
  - Add recursive visible file search, path indexes, size/timestamp metadata, mtime cache invalidation, and safe path resolution.
  - _Requirements: 2.1_
  - _writes: crates/world_model/src/comfy_model_catalog.rs, crates/world_model/src/comfy_model_catalog_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-model-memory-runtime_

- [x] 3. Implement model preview and safetensors metadata reads
  - Add native Sim adjacent preview lookup and bounded safetensors header metadata extraction without loading model weights or passing metadata reads through to ComfyUI.
  - _Requirements: 2.2, 2.3_
  - _writes: crates/world_model/src/comfy_model_metadata.rs, crates/world_model/src/comfy_model_metadata_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-model-memory-runtime_

- [x] 4. Implement model family capability detection
  - Add native Sim model-family and capability records for Comfy-supported image, video, audio, 3D, adapter, segmentation, depth, and detection families.
  - _Requirements: 3.1, 3.2, 3.3_
  - _writes: crates/world_model/src/comfy_model_family.rs, crates/world_model/src/comfy_model_family_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-model-memory-runtime_

- [x] 5. Implement precision, quantization, device, and memory policy resolver
  - Parse native Sim runtime settings, quantization metadata, backend support, dynamic VRAM/offload options, and compatibility diagnostics without passing policy resolution through to ComfyUI.
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 5.1, 5.2_
  - _writes: crates/world_model/src/comfy_runtime_policy.rs, crates/world_model/src/comfy_quantization.rs, crates/world_model/src/comfy_runtime_policy_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-model-memory-runtime_

- [x] 6. Implement model resource release bridge
  - Wire native Sim unload-models and free-memory intents to worker APIs and report success or diagnostic failure without passing resource management through to ComfyUI.
  - _Requirements: 5.1, 5.3_
  - _writes: crates/world_model/src/comfy_model_resources.rs, crates/world_model/src/comfy_model_resources_tests.rs_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-model-memory-runtime_

- [x] 7. Materialize remaining model and memory coverage backlog
  - Convert 120 planned coverage records in model-memory-runtime into native Sim implementation, delegation, unsupported, or divergent outcomes without ComfyUI pass-through.
  - Coverage IDs: all model-memory-runtime records in `crates/world_model/fixtures/comfy/coverage_ledger.json` now marked `Implemented` with `crates/world_model/fixtures/comfy/model_memory_backlog.json` evidence; representative IDs: modelfamily:projects_comfy_comfy_supported_models_py:ACEStep, modelfamily:projects_comfy_comfy_supported_models_py:ACEStep15, modelfamily:projects_comfy_comfy_supported_models_py:Anima, modelfamily:projects_comfy_comfy_supported_models_py:AuraFlow, modelfamily:projects_comfy_comfy_supported_models_py:Boogu, modelfamily:projects_comfy_comfy_supported_models_py:Chroma, modelfamily:projects_comfy_comfy_supported_models_py:ChromaRadiance, modelfamily:projects_comfy_comfy_supported_models_py:CogVideoX_I2V.
  - Expected native Sim writes: crates/world_model/src/comfy_model_folders.rs, crates/world_model/src/comfy_model_family.rs, crates/world_model/src/world_model.rs, crates/world_model/src/comfy_model_family_tests.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/*.json.
  - Validation: `cargo test -p world_model comfy_model`.
  - Parity evidence: Mark records implemented only with model catalog, metadata, family, policy, resource, or fixture evidence; no model downloads.
  - _CoverageTask: coverage-backlog-model-memory-runtime_
  - _CoverageOwner: .agents/specs/godot-migration/comfy-model-memory-runtime_
  - _Requirements: 9.1, 9.2, 9.3, 9.4_
  - _writes: crates/world_model/src/comfy_model_folders.rs, crates/world_model/src/comfy_model_family.rs, crates/world_model/src/world_model.rs, crates/world_model/src/comfy_model_family_tests.rs, crates/world_model/tests/comfy_compat_suite.rs, crates/world_model/fixtures/comfy/*.json

## Notes

- Asset indexing for model files belongs to `comfy-asset-library/`.
- Sampler/scheduler behavior and model-family execution semantics belong to `comfy-diffusion-world-model-runtime/`.
- Process launching and package installation belong to `model-serving-packaging/`.
