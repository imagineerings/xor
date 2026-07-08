# Implementation Plan: Comfy Media Node Pipelines

## Overview

Represent media functionality as capability groups and implement deterministic operations incrementally. Advanced model-backed and native-codec operations get capability diagnostics first, then real backends after dependency review.

## Gates

- Start gate: G0 spec consistency, G5 graph safety, G6 provenance, G7 dependency review for native/heavy backends, and G8 Comfy harness alignment are satisfied.
- Validation gate: capability snapshots, media operation unit tests, and artifact registration integration tests pass.
- Handoff gate: unsupported media nodes have stable diagnostics rather than missing registry entries.
- Completion gate: no media node bypasses Sim preview, asset, graph, or mesh ownership boundaries.

## Dependency Waves

- Global wave: W4 Generation outputs and asset pipelines.
- Local Wave 1: Task 1 builds capability registry.
- Local Wave 2: Tasks 2-4 implement image, video, and audio foundations.
- Local Wave 3: Tasks 5-7 implement 3D, analysis/control, and utility nodes.

## Tasks

- [x] 1. Build media node capability registry
  - Map Comfy media node modules into image/mask, video, audio, 3D/geometry, analysis/control, and utility groups with backend diagnostics.
  - Represent capability groups, ports, backend requirements, diagnostics, developer-only visibility, and handler ownership with native `SimMedia*` records.
  - _Requirements: 1.1, 2.2, 3.2, 4.1, 5.1, 6.3_
  - _writes: crates/world_model/src/sim_media_capabilities.rs, crates/world_model/src/sim_media_capabilities_tests.rs_

- [x] 2. Implement image, mask, and post-processing node adapters
  - Add deterministic bitmap/mask transforms, batch shape preservation, alpha handling, and GLSL dependency metadata.
  - Represent image and mask artifacts, shapes, deterministic transforms, save formats, GLSL dependency metadata, and diagnostics with native `SimImage*` and `SimMask*` records.
  - _Requirements: 1.1, 1.2, 1.3_
  - _writes: crates/world_model/src/sim_image_nodes.rs, crates/world_model/src/sim_mask_nodes.rs, crates/world_model/src/sim_image_nodes_tests.rs_

- [x] 3. Implement video node adapters and diagnostics
  - Add load/create/save/slice metadata handling and backend diagnostics for interpolation, stitching, merging, upscaling, inpaint, caption, depth, pose, face, and segmentation nodes.
  - Represent video metadata, frame ranges, frame batches, advanced operations, backend status, and diagnostics with native `SimVideo*` records.
  - _Requirements: 2.1, 2.2, 2.3_
  - _writes: crates/world_model/src/sim_video_nodes.rs, crates/world_model/src/sim_video_nodes_tests.rs_

- [x] 4. Implement audio node adapters and diagnostics
  - Add audio metadata handling, simple transforms, codec diagnostics, and audio latent model capability validation.
  - Represent audio metadata, sample ranges, edit operations, codec status, equalization bands, and diagnostics with native `SimAudio*` records.
  - _Requirements: 3.1, 3.2, 3.3_
  - _writes: crates/world_model/src/sim_audio_nodes.rs, crates/world_model/src/sim_audio_nodes_tests.rs_

- [x] 5. Implement 3D, geometry, and Gaussian splat adapters
  - Register 3D artifacts, depth/geometry outputs, point clouds, splats, preview metadata, and mesh lifecycle delegation.
  - Represent mesh, point cloud, Gaussian splat, depth, normal, camera, point-map, preview, provenance, format diagnostic, and mesh-pipeline delegation state with native `SimThreeD*` records.
  - _Requirements: 4.1, 4.2, 4.3_
  - _writes: crates/world_model/src/sim_3d_nodes.rs, crates/world_model/src/sim_3d_nodes_tests.rs_

- [x] 6. Implement analysis and control signal adapters
  - Add typed outputs and graph validation compatibility for canny, pose, bounding boxes, face landmarks, segmentation, detection, depth, optical flow, tracking, and camera controls.
  - Represent analysis output kinds, typed media ports, target compatibility, backend status, metadata, and diagnostics with native `SimControlSignal*` records.
  - _Requirements: 5.1, 5.2_
  - _writes: crates/world_model/src/sim_control_signal_nodes.rs, crates/world_model/src/sim_control_signal_nodes_tests.rs_

- [ ] 7. Implement utility and dataset node adapters
  - Add deterministic string, regex, JSON, math, primitive, logic, seed, and path-confined dataset operations.
  - _Requirements: 6.1, 6.2, 6.3_
  - _writes: crates/world_model/src/comfy_utility_nodes.rs, crates/world_model/src/comfy_dataset_nodes.rs, crates/world_model/src/comfy_utility_nodes_tests.rs_

## Notes

- Preview display belongs to `rendering-media/`.
- Textured mesh artifact lifecycle belongs to `mesh-generation-pipeline/`.
