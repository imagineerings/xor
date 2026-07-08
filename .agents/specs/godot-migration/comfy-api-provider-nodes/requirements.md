# Requirements: Comfy API Provider Nodes

## Introduction

Sim needs Comfy API-provider node coverage for cloud and third-party media/LLM/model services such as OpenAI, Gemini, Anthropic, OpenRouter, ByteDance, BFL, Stability, Ideogram, Recraft, Runway, Luma, Kling, Vidu, Wan, Veo, Sora, ElevenLabs, Topaz, Tripo, Meshy, Rodin, Hunyuan3D, and related providers. These provider nodes are policy-gated but core world-model harness extension points when enabled. This spec owns provider-node normalization, request lifecycle, credentials, offline mode, and diagnostics. It delegates local model execution to `comfy-model-memory-runtime/` and worker packaging to `model-serving-packaging/`. Comfy compatibility defines the expected provider-node semantics and fixtures, but every supported provider feature must be recreated as native Sim functionality backed by Sim connector, secret, task, media, and diagnostic services rather than passed through to ComfyUI or represented by a compatibility label alone.

## Glossary

- **API Provider Node**: A Comfy node that calls an external service for generation, enhancement, captioning, LLM, audio, video, image, vector, or 3D output.
- **Provider Connector**: Sim service wrapper for one provider's authentication, request schema, task polling, uploads, downloads, and error handling.
- **Remote Task**: A provider-side asynchronous job that must be polled or subscribed until terminal status.
- **Offline Mode**: Runtime mode where API provider nodes are disabled and frontend/network communication to external services is blocked.
- **Provider Capability**: A declared media operation such as text-to-image, image-edit, text-to-video, image-to-3D, TTS, STT, vectorize, or upscale.

## Requirements

### Requirement 1: Provider Catalog

**User Story:** As a workflow author, I want Sim to expose Comfy API provider nodes as typed capabilities.

#### Acceptance Criteria

1.1 WHEN API nodes are enabled THEN THE system SHALL register provider nodes with provider name, node id, media capability, input schema, output schema, cost/rate metadata when available, and required credentials.
1.2 WHEN API nodes are disabled THEN THE system SHALL omit provider nodes from object info and block external provider calls.
1.3 IF a provider capability is unsupported in Sim THEN THE system SHALL expose an unsupported-provider diagnostic instead of a half-registered node.

### Requirement 2: Credential and Secret Handling

**User Story:** As a user, I want provider credentials protected while still allowing workflows to run.

#### Acceptance Criteria

2.1 WHEN a provider requires credentials THEN THE system SHALL resolve them through Sim secret infrastructure, not workflow JSON plaintext.
2.2 WHEN an API request is serialized for history, logs, or job status THEN THE system SHALL redact credentials, signed URLs, and sensitive provider payload fields.
2.3 IF credentials are missing or invalid THEN THE system SHALL fail the node with an actionable authentication diagnostic.

### Requirement 3: Request, Upload, Poll, and Download Lifecycle

**User Story:** As a workflow author, I want remote provider nodes to behave like normal graph nodes.

#### Acceptance Criteria

3.1 WHEN a provider node starts THEN THE system SHALL validate inputs, upload required source media, create the provider request, and attach the remote task id to the Sim job node state.
3.2 WHILE a remote task is running THE system SHALL report progress or provider status when available and support cancellation where the provider supports it.
3.3 WHEN a remote task completes THEN THE system SHALL download or import outputs, register assets, and attach provenance.
3.4 IF a provider task fails, times out, rate limits, or returns malformed data THEN THE system SHALL fail the node with provider-specific diagnostics.

### Requirement 4: Provider Media Coverage

**User Story:** As a creator, I want provider nodes for image, video, audio, LLM, vector, and 3D workflows.

#### Acceptance Criteria

4.1 WHEN provider nodes perform text-to-image, image-to-image, image edit, inpaint, outpaint, background removal, upscale, relight, style transfer, or vectorization THEN THE system SHALL register image/vector outputs with MIME metadata.
4.2 WHEN provider nodes perform text-to-video, image-to-video, first-last-frame video, video edit, video extend, lip sync, avatar, or enhancement THEN THE system SHALL register video outputs with duration and preview metadata where available.
4.3 WHEN provider nodes perform text-to-audio, speech-to-text, text-to-speech, speech-to-speech, sound effects, dialogue, music, or audio isolation THEN THE system SHALL register audio/text outputs with sample and transcript metadata where available.
4.4 WHEN provider nodes perform text/image/multiview-to-3D, texture, rig, animate, retarget, convert, topology, or model import THEN THE system SHALL register 3D outputs through Sim 3D artifact routing.
4.5 WHEN provider nodes perform LLM or prompt enhancement requests THEN THE system SHALL preserve text output and model/provider metadata.

### Requirement 5: Governance and Cost Safety

**User Story:** As a maintainer, I want remote provider execution controlled because it may spend money or send user data off-device.

#### Acceptance Criteria

5.1 WHEN a provider call may incur cost or transmit project media externally THEN THE system SHALL require explicit user or workspace policy approval.
5.2 WHEN provider quotas, model availability, or endpoint capabilities are known THEN THE system SHALL validate them before execution.
5.3 WHEN offline mode is enabled THEN THE system SHALL prevent provider calls and show disabled-by-policy diagnostics.
