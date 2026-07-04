# Requirements: Comfy Runtime Control Plane

## Introduction

Baymax needs a Comfy-compatible runtime control plane so existing Comfy workflow clients, scripts, and frontends can submit prompts, observe execution, manage queues, and retrieve outputs without copying ComfyUI's web-server implementation. This control plane is core world-model harness functionality because it defines prompt/job lifecycle, realtime progress, queue state, and output retrieval for harness workflows. This spec owns protocol compatibility and safety. It delegates graph editing to `diffusion-graph-editor/`, node execution to `comfy-graph-node-runtime/`, assets to `comfy-asset-library/`, and model worker setup to `model-serving-packaging/`.

## Glossary

- **Prompt**: A serialized Comfy workflow execution request containing node instances, inputs, and optional metadata.
- **Job**: Baymax's durable representation of a submitted prompt, including status, queue priority, timestamps, outputs, and errors.
- **Client Session**: A connected UI or script identified by a client id and optional negotiated feature flags.
- **Control Plane**: HTTP and WebSocket APIs that coordinate prompt submission, queue state, progress, cancellation, and output access.
- **Preview Event**: A binary or JSON event carrying intermediate image, text, video, audio, or 3D preview metadata.

## Requirements

### Requirement 1: Comfy-Compatible API Surface

**User Story:** As a workflow client author, I want Baymax to expose Comfy-compatible endpoints so existing Comfy API scripts can run against Baymax with minimal changes.

#### Acceptance Criteria

1.1 WHEN a client posts a valid prompt to `/prompt` or `/api/prompt` THEN THE system SHALL create a Baymax job and return a prompt id, queue number, and node validation errors.
1.2 WHEN a client requests `/queue`, `/history`, `/history/{prompt_id}`, `/prompt`, `/features`, `/object_info`, or `/object_info/{node_class}` THEN THE system SHALL return Comfy-compatible response shapes backed by Baymax state.
1.3 WHEN a client requests `/models`, `/models/{folder}`, `/embeddings`, or `/extensions` THEN THE system SHALL return catalog data from Baymax model, embedding, and extension registries.
1.4 IF an endpoint has both legacy and `/api` forms THEN THE system SHALL route both forms to the same handler behavior.

### Requirement 2: Queue and Job Lifecycle

**User Story:** As a user, I want prompt jobs to be queued, inspected, cancelled, and interrupted predictably.

#### Acceptance Criteria

2.1 WHEN a prompt includes a client-supplied prompt id THEN THE system SHALL accept only canonical lowercase hyphenated UUID strings.
2.2 WHEN a job is queued THEN THE system SHALL expose pending, running, completed, failed, and cancelled states through the jobs API.
2.3 WHEN a client cancels one or more job ids THEN THE system SHALL cancel running and pending jobs idempotently and treat terminal or unknown jobs as no-ops.
2.4 WHEN a client interrupts a prompt id THEN THE system SHALL interrupt only the matching running job.
2.5 WHEN a client clears queue or history entries THEN THE system SHALL apply the change without leaking sensitive prompt extra data.

### Requirement 3: Realtime Status and Preview Streaming

**User Story:** As a UI client, I want realtime status updates and previews so users can monitor long-running generations.

#### Acceptance Criteria

3.1 WHEN a client opens `/ws` THEN THE system SHALL assign or reuse a session id and send initial queue status.
3.2 WHEN the first WebSocket message contains client feature flags THEN THE system SHALL store them and respond with server feature flags.
3.3 WHEN node execution progresses THEN THE system SHALL emit executing, progress, status, and preview events scoped to the correct prompt and client session.
3.4 IF a client supports preview metadata THEN THE system SHALL send preview events with metadata instead of legacy unencoded preview payloads.

### Requirement 4: HTTP Safety and File Access

**User Story:** As a maintainer, I want Comfy-compatible routes to preserve Baymax's security boundaries.

#### Acceptance Criteria

4.1 WHEN a browser request originates cross-site against a loopback host THEN THE system SHALL reject unsafe mismatched host/origin requests.
4.2 IF API nodes are disabled THEN THE system SHALL apply a content security policy that blocks external frontend communication.
4.3 WHEN a client uploads or views files THEN THE system SHALL reject path traversal and absolute-path escape attempts.
4.4 WHEN a client views potentially executable content THEN THE system SHALL force a safe download content type.
4.5 WHEN cacheable static assets or non-cacheable dynamic responses are served THEN THE system SHALL apply cache-control behavior matching the endpoint purpose.

### Requirement 5: Baymax Integration Boundary

**User Story:** As a Baymax developer, I want Comfy control-plane behavior to reuse Baymax infrastructure rather than fork another application server.

#### Acceptance Criteria

5.1 IF Baymax already has task, process, HTTP, WebSocket, media, project, or secret infrastructure THEN THE Comfy control plane SHALL adapt those systems instead of duplicating them.
5.2 WHEN a control-plane event references generated outputs THEN THE system SHALL reference artifacts through the shared generated artifact and asset systems.
5.3 IF full Comfy parity is not implemented for an endpoint THEN THE system SHALL return an explicit unsupported capability error rather than a partial silent response.
