# ComfyUI backend evidence inventory

## Scope and status

This report covers the Python ComfyUI execution engine, registered built-in and hosted API nodes, HTTP and WebSocket protocols, queue/history/jobs, model and device registries, persisted formats, custom-node and public extension contracts, configuration, and security boundaries in `projects/comfy/ComfyUI`. It is supporting evidence for the shared parity specification; it does not select the final Zed architecture or implement any behavior.

The effective static registries for this source snapshot are reconciled in [`catalogs/backend-reconciliation.json`](catalogs/backend-reconciliation.json). Capabilities that could not be exercised are retained as `code-inferred`, `documented-only`, or `uncertain`, rather than being treated as runtime-confirmed.

## Baseline

| Property | Evidence-backed value |
|---|---|
| Source path | `projects/comfy/ComfyUI` |
| Git metadata | No nested Git metadata was present; no commit SHA can be attributed to this source tree. |
| Package version | `0.27.1`, declared by `pyproject.toml` and `comfyui_version.py` |
| Python requirement | `>=3.10`, declared by `pyproject.toml` |
| All-file source-tree fingerprint | 949 regular files; SHA-256 `21de8fece20d8d5bfa94daaa52d6ccfe2db6726ca0803ca3b383ad164cbd1d5f` over sorted relative paths and file digests |
| `rg --files` fingerprint | 884 visible files; SHA-256 `38fec15d8dadeef9ccd1f677d61c8f36db5e1a2080d445ef5bd8a28ee1e6dea6` |
| Runtime environment | System Python 3.9.6; required packages including `torch`, `aiohttp`, `safetensors`, `yaml`, `numpy`, `PIL`, `sqlalchemy`, `pydantic`, and `pytest` were unavailable. |
| Direct observation | `python3 projects/comfy/ComfyUI/main.py --help` exited successfully and exposed 101 CLI options; `python3 projects/comfy/ComfyUI/main.py --list-feature-flags` exited successfully and printed `show_signin_button=false` and `enable_telemetry=false`. |
| Runtime limitation | Server startup, model execution, GPU/device behavior, registered-node instantiation, protocol probes, and the test suite were not run because the environment does not satisfy the declared Python version or dependencies. |

The all-file fingerprint is deterministic for this tree: enumerate regular files by sorted relative path and hash the path/digest sequence. The per-file SHA-256 values are preserved in [`catalogs/backend-source-coverage.csv`](catalogs/backend-source-coverage.csv).

## Discovery and evidence method

The inventory was derived from executable entry points, AST evaluation of registration mappings and schema methods, route decorators, OpenAPI operations, WebSocket send sites and tests, CLI parser declarations, environment reads, model/device registries, Alembic migrations, typed schemas, persistence code, extension loading, and existing tests. Registration expressions were resolved through imports, inheritance, comprehensions, dictionary expansion, and V3 extension providers. Each catalog row preserves source file, symbol, line, evidence level, availability, Zed status, gap, and test evidence where applicable.

Evidence labels have their shared meanings: `observed` is direct execution, `test-backed` is demonstrated by an existing test, `code-inferred` is supported by executable source without runtime confirmation, `documented-only` is present only in a declarative API document, and `unverified`/`uncertain` preserves insufficient evidence. A test was used as backing only when its body or parametrization directly exercises the cataloged behavior; discovery of a test name alone did not upgrade evidence.

## Registry reconciliation

| Registry or coverage surface | Reconciled result |
|---|---:|
| Registered node identifiers | 789 unique: 565 local built-ins and 224 hosted API nodes |
| Node schema generations | 654 V3 and 135 legacy V1 |
| Registered node availability | 432 active, 118 experimental, 214 cloud/paid, 25 deprecated/dead |
| Node evidence | 23 test-backed, 766 code-inferred |
| Duplicate or unresolved registered identifiers | 0 duplicate, 0 unresolved |
| Explicit `comfy_extras` registration modules | 121 |
| Schema-bearing classes excluded from registration | 12, preserved as deprecated/dead inactive entries |
| Runtime route definitions before aliases | 73: 68 decorated handlers and 5 static routes |
| Effective runtime HTTP routes | 120: 73 definitions plus 47 `/api` compatibility aliases |
| OpenAPI operations | 77 |
| OpenAPI-only operations not matched to executable routes | 21, preserved as documented-only and uncertain |
| HTTP catalog rows | 141 |
| WebSocket wire messages/events | 26 |
| CLI, environment, settings, path, and feature-flag entries | 153 |
| Model, sampler, scheduler, latent, dtype, memory, attention, and hardware entries | 211 |
| Hosted outbound proxy endpoints | 217 unique paths |
| Execution, extension, and security capabilities | 107 |
| Persisted formats and migrations | 40 |
| Typed/OpenAPI schemas | 1,010 |
| Existing test functions | 970 |
| Accounted source files | 949 of 949 |

The 120 runtime route count is the effective server surface after `PromptServer.add_routes` installs `/api` aliases for extension compatibility. The 21 OpenAPI-only rows are not assumed to be reachable. Of the 217 hosted outbound paths, 156 have statically resolvable methods and 61 use a dynamic or inherited wrapper whose exact verb remains explicit as `UNKNOWN`.

## Registered nodes

[`catalogs/backend-nodes.csv`](catalogs/backend-nodes.csv) assigns one stable feature ID to every registered identifier. It records the registration source, implementation source and symbol, display name, category, schema API, resolved schema implementation, execution function, required/optional/hidden inputs, defaults, socket types, outputs, list semantics, lazy inputs, output-node status, validation, caching, change detection, execution blocking, error behavior, availability, evidence, and relevant tests. V1 schema inheritance is resolved across local and imported Python base classes. Dynamic V3 schemas are preserved as source expressions when exact runtime values require unavailable dependencies.

The 789 registrations reconcile to the runtime mappings and V3 extension providers reachable from `nodes.py:init_builtin_extra_nodes`, including 121 explicitly loaded `comfy_extras` modules. The 224 hosted nodes remain inventory entries even though they depend on credentials, network services, account state, service availability, or billing. The 12 classes with node-schema methods that are commented out, omitted from an extension provider, or otherwise not registered are separately cataloged in [`catalogs/backend-inactive-nodes.csv`](catalogs/backend-inactive-nodes.csv); they are not counted as executable nodes.

Node contracts are not homogeneous. The inventory preserves:

- V1 `INPUT_TYPES`, `RETURN_TYPES`, `RETURN_NAMES`, `OUTPUT_IS_LIST`, `INPUT_IS_LIST`, `OUTPUT_NODE`, `VALIDATE_INPUTS`, and `IS_CHANGED` contracts.
- V3 `ComfyNodeABC`/schema contracts, typed IO, hidden inputs, lazy markers, list IO, fingerprinting, validation, async execution, progress, dynamic subgraphs, and class clone isolation.
- `ExecutionBlocker` propagation, dynamic `GraphBuilder` expansion, whole-list invocation, per-item list mapping with last-value broadcast, and merged list outputs.
- Core and hosted domains including loaders, checkpoints, model families, diffusion/model sampling, CLIP, VAE, LoRA, ControlNet, conditioning, latent, image, mask, audio, video, 3D, upscaling, merging, preprocessing, utility, and external API operations.

## Execution, validation, caching, queue, and recovery

The 60 execution entries in [`catalogs/backend-features.csv`](catalogs/backend-features.csv) split independently testable behavior instead of grouping it under a single execution feature. The key contracts are:

- Prompt submission assigns canonical prompt/job identity, runs prompt preprocess handlers and node replacements, validates output nodes, and returns structured node validation failures before queue admission.
- Validation covers missing node types and required inputs, link shape and bounds, socket compatibility, primitive coercion, numeric bounds, combo and multiselect membership, custom node validation, and independent validity of requested outputs.
- Execution uses a topological dependency list, resolves lazy inputs on demand, supports dynamic prompt expansion, runs synchronous and asynchronous nodes, preserves current-node context, and isolates mutable V3 node instances with locked clones.
- Mapping semantics distinguish whole-list inputs from element mapping, broadcast the final value of shorter lists, combine list-designated outputs, and propagate `ExecutionBlocker` values.
- Cache modes include classic, LRU, null, RAM-pressure, and extension-supplied providers. Input signatures, `IS_CHANGED`, and V3 fingerprinting drive incremental re-execution; cached UI payloads are replayed.
- A single prompt worker consumes heap-priority queue entries. Public queue snapshots strip sensitive extra data while the worker retains a private tuple. Queue clear/delete, targeted atomic cancellation, batch cancellation, legacy interrupt, free/unload flags, and periodic garbage collection have separate contracts.
- History is in-memory and capped at 10,000 completed items. Query, deletion, wipe, normalized job list, and normalized job detail are distinct protocol behaviors.
- Progress includes lifecycle WebSocket events, legacy sampler preview callbacks, metadata-aware progress state, queue status broadcast, timing/history status, and per-prompt preview-method override.
- Startup clears the temporary directory. Port collisions can advance to another port when configured. Database initialization can be optional or startup-fatal depending on configuration. Asset scanning can pause around inference and generated outputs can be registered and enriched when the asset database is enabled.

Structured execution failures preserve prompt/node identifiers, exception type/message, traceback, current inputs/outputs, dependent-output invalidation, and terminal execution state. Interruption is checked between node executions and through model-management callbacks. The catalogs distinguish cancellation from validation failure and execution failure; runtime timing and exact GPU cleanup remain unobserved in this environment.

## HTTP and WebSocket protocols

[`catalogs/backend-http-routes.csv`](catalogs/backend-http-routes.csv) records method, canonical and alias path, handler, request/path/query data, success and error behavior, permission/flag conditions, side effects, source, tests, and OpenAPI linkage for each row. It also retains a route-specific static handler or YAML excerpt, extracted request keys/operations, response constructors and return branches, explicit status/content-type evidence, schema confidence, and an explicit unresolved-schema statement. Those fields prevent dynamic Python or hosted response shapes from being represented by a generic placeholder; `VAL-HTTP-001` remains responsible for capturing runtime-only variants. Runtime coverage includes server status and feature flags, object info and embeddings/extensions, prompt/queue/history/jobs, interrupt/free, image/media upload and view, user data and settings, workflow templates, models/model folders, internal diagnostics, asset CRUD/content/tags/thumbnails, and static content. Static routes and developer-facing internal routes are explicit inventory entries.

The route inventory intentionally distinguishes executable routes from OpenAPI-only declarations. A schema or operation in `openapi.yaml` is not treated as a runtime route unless a handler or compatibility alias was found. Conversely, executable routes absent from OpenAPI remain in the runtime inventory.

[`catalogs/backend-websocket-events.csv`](catalogs/backend-websocket-events.csv) records 26 text and binary wire contracts with direction, event type or binary code, payload schema, trigger, ordering/concurrency behavior, tests, and uncertainty. These cover connection/status state, execution start/cache/progress/error/success/interruption, preview frames, metadata-aware progress, feature negotiation, log/diagnostic forwarding, and client-to-server preview configuration. Binary messages preserve their numeric discriminator and payload framing rather than being represented as generic notifications.

WebSocket session identity uses a client-provided identifier and replaces an existing socket with the same ID. This is session routing, not authentication. The server can replay current status on connect; messages associated with an executing prompt are routed by client ID, while broadcast events have different delivery semantics.

## Configuration, CLI, and environment

[`catalogs/backend-config.csv`](catalogs/backend-config.csv) contains 153 independently named configuration surfaces:

- 101 CLI flags directly observed through `main.py --help`, including listener/TLS/CORS, directories, custom-node and API-node policy, preview and cache mode, memory/attention/device/backend selection, dtype and quantization controls, database, logging, manager policy, deterministic/performance switches, and frontend selection.
- 2 CLI-settable feature flags directly observed at their default `false` values through `main.py --list-feature-flags`, plus 5 server feature flags cataloged from code/tests.
- 14 environment variables read by production code.
- 26 model search-path categories.
- Extra-model-path YAML and custom-node project TOML configuration.
- Database configuration plus persisted settings and users JSON files.

Defaults, choices, mutually exclusive groups, value shapes, platform conditions, and source declarations are captured per row. Help output confirms parser exposure but does not confirm that each option works on every platform or hardware backend.

## Models, devices, memory, and numeric formats

[`catalogs/backend-models.csv`](catalogs/backend-models.csv) reconciles 211 registry entries:

| Kind | Count |
|---|---:|
| Registered model architecture detector/loader | 94 |
| Sampling algorithm | 44 |
| Sigma scheduler | 9 |
| Latent tensor scaling/channel contract | 33 |
| Numeric dtype | 9 |
| Quantization format | 4 |
| Hardware backend | 9 |
| Memory mode | 5 |
| Attention backend | 4 |

The catalog keeps model detection keys, tensor/channel and scaling contracts, input/default controls, dependency/platform gates, and failure behavior tied to their source symbols. It covers CPU and supported accelerator branches, normal/high/low/no-VRAM and shared modes, device/offload choices, attention implementations, dtype selection, float8 and quantized weight paths, and scheduler/sampler registries. Availability is 53 active, 149 conditional, and 9 platform-specific. Actual kernel availability, memory pressure, performance, numerical parity, model-file compatibility, preview output, and offload recovery require deterministic runtime validation on supported hardware.

## Persistence and media compatibility

[`catalogs/backend-formats.csv`](catalogs/backend-formats.csv) preserves 40 execution, protocol, file, database, and migration contracts. These include prompt graph JSON, node-info schema JSON, queue/history/job tuples and objects, file locators, latent/image/mask/audio/video/conditioning/3D tensor contracts, PNG/APNG/WebP/SVG/FLAC/MP3/Opus/MP4 media behavior and metadata, safetensors and PyTorch checkpoint loading, latent files, YAML/TOML configuration, settings/users JSON, five database tables, four Alembic migrations, and the OpenAPI description.

[`catalogs/backend-schemas.csv`](catalogs/backend-schemas.csv) contains 1,010 named shapes: 952 executable Python `BaseModel`, `TypedDict`, `NamedTuple`, or dataclass schemas and 58 OpenAPI component schemas. The large cloud/paid share reflects typed request and response contracts for hosted API nodes. The 58 OpenAPI components are `documented-only` and `uncertain` until reconciled dynamically with executable handlers and wire responses.

## Extension and custom-node compatibility

The 28 extension capabilities in [`catalogs/backend-features.csv`](catalogs/backend-features.csv) cover:

- Extra model search-path configuration.
- Arbitrary Python custom-node discovery, import, prestartup scripts, disable/whitelist policy, and manager extension policy.
- Legacy V1 `NODE_CLASS_MAPPINGS`/`NODE_DISPLAY_NAME_MAPPINGS` and V3 `ComfyExtension` registration.
- `pyproject.toml` custom-node metadata and manifest web directories.
- Legacy `WEB_DIRECTORY` exposure, extension enumeration, localization aggregation, templates, and custom/blueprint subgraphs.
- Public API versions `latest`, `0.0.2`, and `0.0.1`, plus V3-to-V1 object-info conversion.
- Hidden prompt, dynamic-prompt, media-metadata, unique-node, and hosted credential inputs.
- Node replacement, progress, cache-provider, and route-alias APIs.
- Hosted API-node offline/disable policy, retries/polling, and media upload/download helpers.

[`catalogs/backend-external-services.csv`](catalogs/backend-external-services.csv) inventories 217 unique hosted proxy paths by provider, auth behavior, source use, node identifiers, retry/cancel behavior, and tests. These entries are classified `cloud/paid`; no real accounts, credentials, paid calls, or externally mutating operations were used. Sixty-one methods remain `UNKNOWN` because their verb is selected through dynamic provider wrappers. Endpoint reachability, response compatibility, rate limits, and current service availability are not inferred from source presence.

## Security boundaries

The 19 security capabilities in [`catalogs/backend-features.csv`](catalogs/backend-features.csv) make the following boundaries explicit:

- By default, cross-site requests are restricted by Host/Origin checks, including loopback matching; an operator can configure CORS and TLS.
- Core routes do not enforce the authentication schemes declared globally by OpenAPI. Multi-user identity can be selected by the `comfy-user` header, so it must not be treated as authenticated identity without a trusted boundary in front of the server.
- User data and input/output/temp file APIs apply directory and path confinement. System-user namespaces, asset ownership scope, upload limits, and dangerous MIME download hardening are separate controls.
- Queue/history serialization separates sensitive hosted credentials from the public tuple, and offline mode blocks hosted API-node network content with middleware and a restrictive content-security policy.
- Custom-node prestartup scripts and imports execute arbitrary Python code. Extension script directories are exposed to the frontend. Both are deliberate trust boundaries, not sandboxed plugin contracts.
- Checkpoint loading prefers safetensors/weights-only mechanisms where supported, but compatibility paths and third-party nodes still require an explicit untrusted-model threat analysis.
- Developer-only internal routes expose logs and filesystem diagnostics. WebSocket `clientId` selection and replacement provide routing but not identity verification.

These are code-level boundaries, not a penetration test. Remote deployment, proxy configuration, filesystem permissions, third-party nodes, hosted credentials, and OS isolation need separate security validation.

## Test and source-file accounting

[`catalogs/backend-tests.csv`](catalogs/backend-tests.csv) lists 970 discovered test functions, their classes, line numbers, async/parametrized status, and mapped feature IDs. Parametrized cases are not expanded into synthetic test counts. The tests were not executed because `pytest` and runtime dependencies are absent; `test-backed` means the checked-in test explicitly demonstrates the behavior, not that it passed here.

[`catalogs/backend-source-coverage.csv`](catalogs/backend-source-coverage.csv) accounts for all 949 regular files with classification, mapped feature IDs where applicable, explicit reason, per-file digest, and size. The classifications are 288 production source, 45 production configuration/data, 106 production data/template, 37 production data placeholders, 358 infrastructure-only, 98 test-only, 14 documented-only, and 3 generated. There are 561 files with direct feature mappings; every remaining file has an explicit non-feature classification and reason rather than an unexplained blank.

## Explicit uncertainties and validation needs

- No registered node or inference path was instantiated; node contracts are static except where an existing test supplies backing.
- Dynamic schema values whose evaluation imports model libraries are retained as source expressions. Runtime defaults can still depend on installed models, extensions, platform, or environment.
- Twenty-one OpenAPI-only operations and 58 OpenAPI component schemas are declarative and may be stale, future-facing, or generated for a different route set.
- Sixty-one hosted proxy paths have an unresolved HTTP method. Hosted service behavior, credentials, billing, quotas, retries, polling timeouts, response drift, and availability require isolated contract fixtures or service-owner evidence.
- GPU/backend selection, dtype fallback, offloading, memory pressure, interruption latency, preview data, cache eviction, database concurrency, subprocess behavior, offline enforcement, and port collision recovery remain runtime validation targets.
- The source snapshot has no nested Git identity. Package version and deterministic file fingerprints are the only attributable baseline.
- Existing tests were cataloged but not executed. There is no runtime pass/fail percentage for the backend in this environment.

## Catalog index

| Artifact | Purpose |
|---|---|
| [`catalogs/backend-reconciliation.json`](catalogs/backend-reconciliation.json) | Baseline, registry totals, and generated-catalog manifest |
| [`catalogs/backend-nodes.csv`](catalogs/backend-nodes.csv) | One row per registered V1/V3 built-in or hosted API node |
| [`catalogs/backend-inactive-nodes.csv`](catalogs/backend-inactive-nodes.csv) | Schema-bearing node classes not present in the active registry |
| [`catalogs/backend-http-routes.csv`](catalogs/backend-http-routes.csv) | Runtime, compatibility-alias, static, and OpenAPI-only HTTP rows |
| [`catalogs/backend-websocket-events.csv`](catalogs/backend-websocket-events.csv) | Text and binary WebSocket contracts |
| [`catalogs/backend-config.csv`](catalogs/backend-config.csv) | CLI, environment, feature flag, search path, settings, and configuration entries |
| [`catalogs/backend-models.csv`](catalogs/backend-models.csv) | Model families, samplers, schedulers, latent formats, dtypes, hardware, memory, and attention modes |
| [`catalogs/backend-external-services.csv`](catalogs/backend-external-services.csv) | Hosted API-node outbound proxy paths and use sites |
| [`catalogs/backend-features.csv`](catalogs/backend-features.csv) | Execution, extension, and security capabilities |
| [`catalogs/backend-formats.csv`](catalogs/backend-formats.csv) | Persisted/wire/media formats, database tables, and migrations |
| [`catalogs/backend-schemas.csv`](catalogs/backend-schemas.csv) | Executable typed schemas and OpenAPI components |
| [`catalogs/backend-source-coverage.csv`](catalogs/backend-source-coverage.csv) | Per-file accounting and source fingerprint data |
| [`catalogs/backend-tests.csv`](catalogs/backend-tests.csv) | Existing backend test-function inventory and feature linkage |
