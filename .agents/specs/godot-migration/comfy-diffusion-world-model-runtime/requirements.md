# Requirements: Comfy Diffusion and World Model Runtime

## Introduction

Baymax needs Comfy's runtime knowledge for running diffusion models and world models, not only Comfy's API and graph orchestration. This spec owns sampler, scheduler, denoising, conditioning, latent, VAE, model patch, guidance, and model-family execution semantics for local harness workflows. It delegates graph scheduling and cache decisions to `comfy-graph-node-runtime/`, model discovery and memory policy to `comfy-model-memory-runtime/`, worker process setup to `model-serving-packaging/`, artifact/media routing to `comfy-asset-library/` and `rendering-media/`, and interactive control semantics to `world-model-runtime/`.

## Glossary

- **Sampling Run**: A model execution request that denoises latent inputs using a sampler, scheduler, conditioning, seed/noise policy, model profile, and runtime policy.
- **Sampler**: The algorithm that advances latent state across diffusion steps.
- **Scheduler**: The timestep or sigma schedule used by a sampler.
- **Conditioning Bundle**: Text, image, control, area, mask, timing, pooled output, and guidance metadata consumed by a model family.
- **Latent Format**: The tensor layout, channel count, scale factor, temporal dimensions, and metadata used by a model family.
- **Model Patch**: A LoRA, ControlNet, GLIGEN, style, hypernetwork, model patch, or other modifier applied to model or conditioning state.
- **World Model Backbone**: A video or interactive world-generation model family such as Wan, Hunyuan Video, LTXV, CogVideo, Cosmos, Genmo/Lightricks, or related temporal diffusion families.

## Requirements

### Requirement 1: Sampler and Scheduler Semantics

**User Story:** As a workflow author, I want Baymax to run Comfy sampler nodes with the same meaningful inputs and progress behavior.

#### Acceptance Criteria

1.1 WHEN a KSampler, advanced sampler, custom sampler, or sampling helper node starts THEN THE system SHALL capture sampler name, scheduler, seed, noise policy, steps, CFG or guidance, denoise amount, start/end step bounds, latent shape, positive conditioning, negative conditioning, and model profile.
1.2 IF a sampler, scheduler, or guidance mode is unsupported by Baymax THEN THE system SHALL reject execution with an unsupported-sampling diagnostic before model work starts.
1.3 WHEN deterministic execution is requested and the selected worker supports it THEN THE system SHALL record seed, noise, sampler, scheduler, backend, precision, and model hash metadata needed to reproduce the run.
1.4 WHILE a sampling run is executing THE system SHALL report current step, total steps, preview availability, and cancellation state through the parent Baymax job.

### Requirement 2: Conditioning and Guidance Semantics

**User Story:** As a workflow author, I want prompt, image, control, and regional conditioning to survive migration into Baymax.

#### Acceptance Criteria

2.1 WHEN CLIP, text encoder, vision encoder, or prompt nodes produce conditioning THEN THE system SHALL preserve token embeddings, pooled outputs, attention metadata, model-family encoder identity, and source prompt metadata where available.
2.2 WHEN conditioning transform nodes combine, average, concatenate, multiply, zero, set area, set mask, set range, or attach inpaint data THEN THE system SHALL preserve Comfy-compatible conditioning bundle metadata.
2.3 WHEN ControlNet, GLIGEN, style model, unCLIP, IP-adapter, reference image, pose, depth, segmentation, or camera/control signals are attached THEN THE system SHALL validate compatibility against the selected model family before execution.
2.4 IF conditioning data is incompatible with the selected sampler, model family, latent format, or worker backend THEN THE system SHALL fail validation with node-scoped diagnostics.

### Requirement 3: Latent, VAE, and Model Component Execution

**User Story:** As a creator, I want model loaders, latent operations, and VAE operations to produce outputs that can feed generation workflows.

#### Acceptance Criteria

3.1 WHEN VAE encode, VAE decode, tiled VAE, temporal VAE, or inpaint VAE nodes run THEN THE system SHALL preserve latent format, temporal size, tile overlap, mask, color, and compression metadata.
3.2 WHEN checkpoint, diffusion model, CLIP, VAE, UNet, text encoder, or Diffusers loaders run THEN THE system SHALL compose typed model components using the model catalog and loader profiles.
3.3 WHEN LoRA, hypernetwork, ControlNet, GLIGEN, model patch, model merge, or edit-model nodes run THEN THE system SHALL apply patches in a deterministic order and attach patch provenance to the sampling run.
3.4 IF a latent, VAE, model component, or model patch does not match the selected model family THEN THE system SHALL reject execution with an actionable compatibility diagnostic.

### Requirement 4: Diffusion and World Model Family Execution

**User Story:** As a game creator, I want Baymax to preserve Comfy's knowledge of how image, video, audio, 3D, and world-model families execute.

#### Acceptance Criteria

4.1 WHEN a workflow uses image diffusion families such as SD, SDXL, SD3, Flux, PixArt, Cascade, Chroma, Qwen Image, HiDream, Aura, Lumina, Kandinsky, or related Comfy-supported families THEN THE system SHALL route execution through a compatible runner profile or report the missing capability.
4.2 WHEN a workflow uses video or world-model backbones such as Wan, Hunyuan Video, LTXV, CogVideo, Cosmos, Genmo, Lightricks, or related temporal families THEN THE system SHALL preserve temporal latent shape, frame count, reference frames, camera/control inputs, guidance metadata, and model-family execution constraints.
4.3 WHEN a workflow uses audio, 3D, geometry, depth, segmentation, detection, or other specialized generative families THEN THE system SHALL validate the model-family execution profile and delegate artifact lifecycle to the owning media or mesh spec.
4.4 IF a model family is present in Comfy but unsupported in Baymax THEN THE system SHALL expose a family-specific unsupported diagnostic instead of silently falling back to another runner.

### Requirement 5: Worker Runtime Boundary

**User Story:** As a maintainer, I want model execution to use Baymax worker infrastructure without losing Comfy semantics.

#### Acceptance Criteria

5.1 WHEN local execution requires Python, PyTorch, GPU APIs, custom kernels, or native packages THEN THE system SHALL use the `model-serving-packaging/` worker boundary instead of loading those dependencies into Baymax UI code.
5.2 WHEN execution needs precision, quantization, device, attention, offload, or memory decisions THEN THE system SHALL use policies from `comfy-model-memory-runtime/`.
5.3 IF execution requires model weights, external packages, or large downloads THEN THE system SHALL require explicit user action and dependency review before execution.
5.4 WHEN execution completes, fails, or is cancelled THEN THE system SHALL update Baymax job state, output artifacts, previews, diagnostics, and provenance consistently.

### Requirement 6: Compatibility Fixtures and Divergence Records

**User Story:** As a maintainer, I want Comfy execution behavior captured in fixtures so future changes do not drift.

#### Acceptance Criteria

6.1 WHEN sampling support is implemented THEN THE system SHALL include compatibility fixtures for text-to-image, image-to-image, inpaint, ControlNet, LoRA, VAE, sampler/scheduler, and video/world-model workflows.
6.2 WHEN production model weights are unavailable in tests THEN THE system SHALL use mock runners, metadata snapshots, or threshold fixtures that do not require silent downloads.
6.3 IF Baymax intentionally diverges from a Comfy execution behavior THEN THE system SHALL record the safety, security, dependency, platform, or product reason in a machine-readable divergence catalog.
