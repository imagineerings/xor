# Requirements: Comfy Media Node Pipelines

## Introduction

Baymax needs the feature coverage represented by Comfy's media-processing node library: image, video, audio, 3D, depth, segmentation, pose, detection, post-processing, and utility nodes. These node capabilities are core world-model harness functionality because they define the media transformations and control signals available to generation workflows. This spec owns capability grouping and node-level media transformations. It delegates preview display to `rendering-media/`, mesh artifact lifecycle to `mesh-generation-pipeline/`, graph execution semantics to `comfy-graph-node-runtime/`, and diffusion/world-model execution semantics to `comfy-diffusion-world-model-runtime/`.

## Glossary

- **Media Node**: A node that transforms, loads, saves, previews, analyzes, or generates image, video, audio, 3D, mask, latent, or text data.
- **Previewable Output**: Output media that can be shown by Baymax preview infrastructure: image, video, audio, text, 3D, or generated preview metadata.
- **Latent Media**: Intermediate tensors or encoded representations used by diffusion/audio/video models.
- **Analysis Node**: A node that computes detection, segmentation, pose, depth, geometry, face landmarks, or metadata from source media.
- **Post-Processing Node**: A node that performs deterministic media transformations such as color, crop, blur, sharpen, composite, resize, or channel operations.

## Requirements

### Requirement 1: Image and Mask Operations

**User Story:** As a creator, I want Comfy image and mask operations available in Baymax workflows.

#### Acceptance Criteria

1.1 WHEN an image pipeline uses load, save, preview, resize, crop, pad, invert, batch, stitch, tile, rotate, flip, add noise, SVG save, or get-size nodes THEN THE system SHALL provide equivalent node behavior or an explicit unsupported diagnostic.
1.2 WHEN a mask pipeline uses mask-to-image, image-to-mask, color-to-mask, solid mask, invert, crop, composite, feather, grow, threshold, or preview nodes THEN THE system SHALL preserve Comfy-compatible data shapes.
1.3 WHEN post-processing nodes perform blend, blur, quantize, sharpen, color transfer, brightness, contrast, levels, hue, saturation, curves, film grain, glow, chromatic aberration, edge-preserving blur, or unsharp mask THEN THE system SHALL route deterministic operations through Baymax media processing services.

### Requirement 2: Video Operations

**User Story:** As a creator, I want Comfy video workflows for generation, slicing, saving, merging, interpolation, upscaling, captioning, inpainting, and pose/depth/segmentation analysis.

#### Acceptance Criteria

2.1 WHEN a video node loads, slices, creates, saves, or decomposes video THEN THE system SHALL preserve frame rate, frame range, audio link, metadata, and output artifact references where supported.
2.2 WHEN a pipeline uses frame interpolation, video stitch, merge, upscale, inpaint, depth estimation, pose extraction, face detection, segmentation, or captioning THEN THE system SHALL expose node capability metadata and backend diagnostics.
2.3 IF a video operation requires unsupported codecs or native dependencies THEN THE system SHALL require dependency review and return an unsupported diagnostic until approved.

### Requirement 3: Audio Operations

**User Story:** As a creator, I want Comfy audio nodes for generation, preview, loading, saving, recording, trimming, channel operations, concatenation, mixing, volume, equalization, and speech/music provider outputs.

#### Acceptance Criteria

3.1 WHEN an audio node loads, previews, saves WAV/MP3/Opus, records, trims, splits channels, joins channels, concatenates, merges, adjusts volume, creates empty audio, or applies equalization THEN THE system SHALL preserve sample rate, channels, duration, and MIME metadata.
3.2 WHEN an audio diffusion node encodes, decodes, or conditions audio latents THEN THE system SHALL validate model and VAE capabilities before execution.
3.3 IF audio encoding requires unavailable codecs THEN THE system SHALL surface actionable diagnostics.

### Requirement 4: 3D, Geometry, and Gaussian Splat Operations

**User Story:** As a game creator, I want Comfy 3D and geometry outputs integrated with Baymax project assets.

#### Acceptance Criteria

4.1 WHEN a node loads, previews, transforms, renders, merges, saves, or converts 3D assets or Gaussian splats THEN THE system SHALL route artifacts through Baymax 3D asset and preview infrastructure.
4.2 WHEN depth or geometry estimation produces mesh, point cloud, normal, camera, or point-map outputs THEN THE system SHALL register outputs with provenance and preview metadata.
4.3 IF a node creates textured meshes or game-ready exports THEN THE system SHALL delegate artifact lifecycle to `mesh-generation-pipeline/`.

### Requirement 5: Analysis, Detection, and Control Signals

**User Story:** As a technical artist, I want detection and control-signal nodes for guided generation.

#### Acceptance Criteria

5.1 WHEN a pipeline uses canny, pose, keypoints, bounding boxes, face landmarks, segmentation, RT-DETR, SAM3, depth, geometry, optical flow, camera trajectory, or tracking nodes THEN THE system SHALL expose typed output ports and capability diagnostics.
5.2 WHEN analysis outputs feed ControlNet, inpainting, pose-to-image, pose-to-video, depth-to-image, or depth-to-video nodes THEN THE system SHALL validate type compatibility before execution.

### Requirement 6: Utility Media Nodes

**User Story:** As a workflow author, I want text, math, primitive, dataset, and logic utility nodes to support media pipelines.

#### Acceptance Criteria

6.1 WHEN a pipeline uses string, regex, JSON extraction, number conversion, math expression, boolean logic, switches, primitive values, or seed nodes THEN THE system SHALL provide deterministic node behavior.
6.2 WHEN a dataset node loads, saves, shuffles, deduplicates, buckets, or prepares image/text training data THEN THE system SHALL confine filesystem access and preserve source attribution.
6.3 IF a utility node is test-only or unsafe for production THEN THE system SHALL hide it unless developer mode is enabled.
