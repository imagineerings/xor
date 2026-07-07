# Implementation Plan: Comfy Diffusion and World Model Runtime

## Overview

Implement Comfy model-execution semantics after model catalogs and graph validation exist. Start with metadata and validation, then add sampling request construction, conditioning/latent/patch semantics, world-model runner profiles, worker bridging, and compatibility fixtures.

## Gates

- Start gate: G0 spec consistency, G3 shared world-model foundations, G4 worker safety, G5 graph safety, G7 dependency review for native/heavy runtimes, and G8 Comfy harness alignment are satisfied.
- Validation gate: sampler registry tests, sampling request tests, conditioning tests, latent/VAE tests, patch-order tests, mock worker tests, and compatibility fixture snapshots pass.
- Handoff gate: unsupported samplers, schedulers, model families, and Sim divergences are visible in machine-readable catalogs.
- Completion gate: no local diffusion or world-model execution can start without model-family validation, worker capability checks, and provenance wiring.

## Dependency Waves

- Global wave: W4 Authoring, graph UX, and Comfy workflows.
- Local Wave 1: Tasks 1-2 define execution capabilities and sampling request construction.
- Local Wave 2: Tasks 3-5 implement conditioning, latent/VAE, and model patch semantics.
- Local Wave 3: Tasks 6-8 add world-model runner profiles, worker bridging, and compatibility fixtures.

## Tasks

- [ ] 1. Implement execution capability registry
  - Add sampler, scheduler, guidance, latent, VAE, patch, model-family, and divergence capability records.
  - _Requirements: 1.2, 4.1, 4.2, 4.3, 4.4, 6.3_
  - _writes: crates/world_model/src/comfy_execution_registry.rs, crates/world_model/src/comfy_execution_registry_tests.rs_

- [ ] 2. Implement sampling run request builder
  - Convert KSampler, advanced sampler, custom sampler, and sampling helper node inputs into validated sampling requests with deterministic metadata.
  - _Requirements: 1.1, 1.2, 1.3, 1.4_
  - _writes: crates/world_model/src/comfy_sampling.rs, crates/world_model/src/comfy_sampling_tests.rs_

- [ ] 3. Implement conditioning runtime semantics
  - Preserve text, vision, pooled, attention, area, mask, range, inpaint, ControlNet, GLIGEN, style, unCLIP, IP-adapter, reference, pose, depth, segmentation, and camera/control metadata.
  - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - _writes: crates/world_model/src/comfy_conditioning.rs, crates/world_model/src/comfy_conditioning_tests.rs_

- [ ] 4. Implement latent and VAE runtime semantics
  - Add latent format validation, VAE encode/decode/tiled/temporal/inpaint metadata, mask handling, compression metadata, and mismatch diagnostics.
  - _Requirements: 3.1, 3.4_
  - _writes: crates/world_model/src/comfy_latents.rs, crates/world_model/src/comfy_vae.rs, crates/world_model/src/comfy_latents_tests.rs_

- [ ] 5. Implement model component and patch pipeline
  - Compose loader outputs and apply LoRA, hypernetwork, ControlNet, GLIGEN, model patch, model merge, and edit-model records in deterministic order with provenance.
  - _Requirements: 3.2, 3.3, 3.4_
  - _writes: crates/world_model/src/comfy_model_components.rs, crates/world_model/src/comfy_model_patches.rs, crates/world_model/src/comfy_model_patches_tests.rs_

- [ ] 6. Add diffusion and world-model runner profiles
  - Define runner profiles for supported image diffusion, video/world-model, audio, 3D, geometry, depth, segmentation, and detection families with explicit unsupported diagnostics.
  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _writes: crates/world_model/src/comfy_runner_profiles.rs, crates/world_model/src/comfy_world_model_profiles.rs, crates/world_model/src/comfy_runner_profiles_tests.rs_

- [ ] 7. Implement worker execution adapter
  - Send validated sampling requests through Sim worker boundaries with capability checks, progress, previews, cancellation, terminal state mapping, output collection, and provenance updates.
  - _Requirements: 5.1, 5.2, 5.3, 5.4_
  - _writes: crates/world_model/src/comfy_worker_execution.rs, crates/world_model/src/comfy_worker_execution_tests.rs_

- [ ] 8. Add model execution compatibility fixtures
  - Add fixture snapshots for text-to-image, image-to-image, inpaint, ControlNet, LoRA, VAE, sampler/scheduler, and video/world-model workflows using mock runners where production weights are unavailable.
  - _Requirements: 6.1, 6.2, 6.3_
  - _writes: crates/world_model/fixtures/comfy/model_execution_manifest.json, crates/world_model/tests/comfy_model_execution.rs_

## Notes

- Graph scheduling, cache policy, and node ordering stay in `comfy-graph-node-runtime/`.
- Model folder discovery, family metadata, memory policy, device policy, precision, and quantization stay in `comfy-model-memory-runtime/`.
- Python environments, worker launch, package setup, GPU diagnostics, and large downloads stay in `model-serving-packaging/`.
- Media preview and artifact display stay in `rendering-media/`; asset indexing stays in `comfy-asset-library/`.
