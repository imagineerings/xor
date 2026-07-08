# Requirements: Comfy Model and Memory Runtime

## Introduction

Sim needs Comfy-compatible model discovery, loader profiles, precision controls, quantization metadata, and memory policies for local and remote generation workflows. These policies are core world-model harness functionality because graph validation and worker execution depend on Comfy model categories, family detection, and memory choices. This spec owns model catalog and runtime policy. It delegates process launching and dependency checks to `model-serving-packaging/`, graph execution to `comfy-graph-node-runtime/`, sampler/model-family execution semantics to `comfy-diffusion-world-model-runtime/`, and asset indexing to `comfy-asset-library/`. Comfy compatibility defines the expected model and memory semantics and fixtures, but every supported model feature must be recreated as native Sim functionality backed by Sim catalog, metadata, policy, worker, artifact, and diagnostic services rather than passed through to ComfyUI or represented by a compatibility label alone.

## Glossary

- **Model Folder**: A named model category such as checkpoints, diffusion models, text encoders, VAE, LoRA, ControlNet, upscale models, or embeddings.
- **Loader Profile**: A model-family-specific loading configuration derived from checkpoint metadata and node requirements.
- **Memory Policy**: Runtime choices for GPU/CPU placement, offload, cache behavior, and free-memory actions.
- **Precision Policy**: Runtime choices for fp32, fp16, bf16, fp8, quantized weights, and intermediate tensor precision.
- **Model Capability**: The media and node capabilities supported by a loaded model family.

## Requirements

### Requirement 1: Model Folder Catalog

**User Story:** As a workflow author, I want Sim to find Comfy model files in expected folders so existing workflows can resolve model names.

#### Acceptance Criteria

1.1 WHEN Comfy compatibility is enabled THEN THE system SHALL register model folders for checkpoints, configs, LoRA, VAE, text encoders, diffusion models, CLIP vision, style models, embeddings, Diffusers, VAE approximations, ControlNet, GLIGEN, upscale models, latent upscale models, hypernetworks, model patches, audio encoders, background removal, frame interpolation, geometry estimation, optical flow, and detection.
1.2 WHEN extra model path config is supplied THEN THE system SHALL merge additional roots without replacing Sim project asset roots.
1.3 WHEN a legacy folder name is requested THEN THE system SHALL map it to the canonical category.

### Requirement 2: Model Discovery and Metadata

**User Story:** As a user, I want the model browser to show names, size, timestamps, previews, and safetensors metadata.

#### Acceptance Criteria

2.1 WHEN a model folder is listed THEN THE system SHALL return visible files with path index, relative name, size, created timestamp, and modified timestamp.
2.2 WHEN a preview image exists next to a model file THEN THE system SHALL expose it through a safe preview route.
2.3 WHEN a safetensors file has metadata THEN THE system SHALL expose header metadata without loading full weights.

### Requirement 3: Loader and Capability Profiles

**User Story:** As a workflow author, I want Sim to recognize Comfy-supported model families so graph validation can catch incompatible pipelines.

#### Acceptance Criteria

3.1 WHEN a checkpoint or standalone model is selected THEN THE system SHALL detect the model family and expose media capabilities, latent format, text encoder requirements, VAE requirements, and supported conditioning modes.
3.2 WHEN a workflow references LoRA, ControlNet, style model, GLIGEN, hypernetwork, or model patch inputs THEN THE system SHALL validate that the base model and adapter are compatible.
3.3 IF a model family is unsupported by Sim THEN THE system SHALL report an unsupported model diagnostic with the missing capability.

### Requirement 4: Precision, Quantization, and Device Policy

**User Story:** As a developer, I want memory and precision controls equivalent to Comfy launch options without unsafe defaults.

#### Acceptance Criteria

4.1 WHEN a precision policy is selected THEN THE system SHALL apply fp32, fp16, bf16, fp8, or quantized behavior only to compatible model components.
4.2 WHEN quantization metadata is present THEN THE system SHALL parse layer formats and scaling parameters before choosing quantized operations.
4.3 WHEN a device policy is selected THEN THE system SHALL validate CPU, CUDA, HIP, DirectML, oneAPI, Ascend, multi-GPU, and default-device constraints before execution.
4.4 WHEN a memory policy is selected THEN THE system SHALL support GPU-only, high-VRAM, low-VRAM, no-VRAM, dynamic VRAM, async offload, pinned memory, mmap, and cache release intents where supported.

### Requirement 5: Diagnostics and Explicit Downloads

**User Story:** As a maintainer, I want model setup failures to be diagnosable and never silently download large files.

#### Acceptance Criteria

5.1 IF a model file, dependency, precision mode, or device backend is unavailable THEN THE system SHALL return actionable diagnostics before worker execution.
5.2 IF setup requires downloading model weights or heavy packages THEN THE system SHALL require explicit user action and dependency review.
5.3 WHEN a memory-free or unload-models command is issued THEN THE system SHALL release model resources through Sim worker APIs and report the result.
