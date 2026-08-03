# Requirements: Comfy behavioral parity in Sim

## Problem

Sim has no Comfy-specific implementation. Users cannot currently open and edit
Comfy workflows, execute their graphs with native tensors and model kernels,
inspect outputs, or rely on Comfy's protocol, node, extension, desktop, and
persistence contracts. Generic Sim workspace and GPUI primitives are useful
implementation infrastructure, but they are not observable parity.

The target is an idiomatic Rust/GPUI integration whose observable behavior is
compatible with the source snapshots fixed in [baseline.md](baseline.md). The
machine-readable feature catalog is normative: each `feature_id` is an
independently addressable contract and carries its source evidence, detailed
variants, Sim gap, acceptance signal, and planned validation.

## Actors

- **Workflow author:** creates, edits, imports, saves, and runs workflows.
- **Operator:** configures native compute backends, devices, model roots,
  plugins, updates, diagnostics, workers, and recovery.
- **Plugin author:** supplies versioned Rust source plugins or WASM Component
  Model plugins with explicit ports, permissions, legacy identifiers, node
  schemas, declarative UI contributions, and serialized workflow data.
- **Remote or cloud user:** uses Sim's native protocol host or an explicitly
  configured provider whose authentication, entitlements, and storage differ
  from the local runtime.
- **Automation client:** uses Comfy's HTTP and WebSocket contracts directly.
- **Sim administrator:** configures trust, native API exposure, storage,
  secrets, telemetry, worker isolation, plugin grants, and device policy.

## Scope

In scope are all source features represented by `catalogs/features.csv`,
including active, conditional, platform-specific, experimental,
developer-only, cloud/paid, deprecated/dead, infrastructure-only, and uncertain
entries. Active and conditional source features require an implementation task
or an explicit, evidence-backed defer decision. Other availability classes
remain visible and traceable even when their parity decision is defer,
compatibility-only, or inventory-only.

The following constraints do not remove source features from the inventory:

- Production Sim SHALL implement execution, nodes, tensors, autograd, model
  loading, samplers, schedulers, devices, memory management, caching, media,
  cancellation, and recovery in native Rust components. It SHALL NOT launch,
  manage, bundle, connect to, or depend on a Python ComfyUI process.
- ComfyUI may be launched only by development/test conformance tooling. That
  oracle is a reverse development dependency and is absent from production
  dependency graphs, settings, menus, binaries, packages, and runtime paths.
- Python custom nodes, JavaScript web extensions, DOM widgets, and LiteGraph
  imperative hooks SHALL NOT execute in production Sim. Compatibility is by a
  versioned Rust/WASM plugin API, explicit port descriptors, deterministic
  legacy identifier mappings, and lossless unresolved placeholders.
- Vue, Electron, Python, and LiteGraph architecture need not be copied.
- Pixel identity is not required unless appearance changes discoverability,
  state communication, interaction, media interpretation, or accessibility.
- This specification does not authorize source modification, dependency
  installation, account use, paid calls, model downloads, commits, pushes, or
  pull requests.

## Requirements

### Requirement 1: Evidence-addressable feature contracts

The implementation and its tests must preserve a stable link from each source
capability to an observable Sim decision.

#### Acceptance criteria

1. THE specification SHALL assign one stable `COMFY-*` identifier to every independently testable source capability and SHALL never reuse an identifier for a different capability.
2. THE `catalogs/features.csv` row for each feature SHALL record product, domain, name, source classification, availability, evidence level, confidence, source symbol, relevant test or documentation, actor, trigger, conditions, success behavior, failure and recovery behavior, persistence or compatibility effects, Sim status, gap, requirement criterion, design decision, task, and validation identifier.
3. WHEN source evidence is contradictory or cannot be materialized THEN THE catalog SHALL retain the feature as `unverified` or `uncertain` with the contradiction or missing prerequisite stated.
4. WHEN a feature is deprecated, unreachable, disabled, developer-only, experimental, platform-specific, or cloud/paid THEN THE catalog SHALL preserve that classification and SHALL state whether Sim implements, isolates, defers, or only recognizes it.
5. WHEN source registries change in a future baseline THEN THE catalog generator SHALL report added, removed, and changed contracts without renumbering surviving feature IDs.

### Requirement 2: Superseded external ComfyUI connection modes

The former external, managed, and bundled ComfyUI connection architecture is
withdrawn. These durable criterion IDs now record its explicit supersession and
the migration behavior needed for data created by earlier planning baselines.

#### Acceptance criteria

1. THE production application SHALL NOT create or expose a managed, bundled, adopted, or external ComfyUI execution profile; this criterion supersedes the former connection-mode requirement and maps usable configuration into a native runtime profile under Requirement 33.
2. THE production application SHALL NOT negotiate object-info, routes, extension requirements, or execution capability from a ComfyUI server; the compiled native registry, signed plugin manifests, and native service capabilities SHALL be authoritative.
3. WHEN imported state contains a former ComfyUI connection profile THEN Sim SHALL preserve it as an inactive migration record, explain why it cannot execute, and offer a non-destructive conversion of model roots, API-host policy, plugin mappings, and workflow state.
4. WHEN legacy connectivity states or credentials are encountered THEN Sim SHALL neither reconnect nor transmit them, SHALL redact or remove secret material through an explicit migration, and SHALL retain an auditable migration result.
5. WHEN a user switches native runtime profiles or windows THEN workflow, queue, history, output, model, secret, plugin, device, and memory state SHALL remain scoped to the selected native profile.

### Requirement 3: Superseded Python engine installation and lifecycle

Python, pip, Git custom-node environments, and ComfyUI server lifecycle are
source behaviors to inventory, not production dependencies to reproduce.

#### Acceptance criteria

1. THE production runtime SHALL NOT probe for or require Python, pip, a ComfyUI checkout, Git custom-node repositories, or a ComfyUI port before native execution can become ready.
2. WHEN Sim installs or updates a native backend, codec, model artifact, compatibility registry, or plugin THEN it SHALL expose deterministic stages, byte and item progress, logs, cancellation, supported pause or resume, integrity checks, and a recoverable operation record.
3. WHEN native execution starts THEN Sim SHALL launch only Sim-owned Rust workers, negotiate the private worker protocol and backend capability, capture sanitized diagnostics, and reach native readiness without opening a ComfyUI URL.
4. WHEN stop, restart, shutdown, or application quit is requested THEN Sim SHALL prevent new work, propagate cancellation, wait for worker fences within a bound, terminate only owned Rust workers when needed, and journal the final state.
5. IF backend initialization, worker readiness, health, or shutdown fails THEN Sim SHALL surface the exact stage and retained logs, remove only proven temporary state, and offer retry, diagnostics, backend change, repair, or safe reset.
6. WHEN legacy Python installation metadata is imported THEN Sim SHALL treat it as read-only migration evidence and SHALL never launch, update, delete, or reconfigure that installation.

### Requirement 4: Native prompt validation and graph execution semantics

Automation clients and workflow authors require Comfy-compatible graph
semantics implemented by Sim's native compiler, planner, and executor.

#### Acceptance criteria

1. WHEN a prompt is submitted THEN the native compiler SHALL preserve node identifiers, class types, input values, link tuples, hidden inputs, output-node reachability, and client or prompt identifiers exactly as defined by the baseline protocol.
2. WHEN validating a graph THEN Sim SHALL report the same missing node, missing required input, type or value constraint, invalid link, unreachable output, and custom validation failures with node-addressable details.
3. WHEN execution starts THEN dependencies SHALL run in a valid topological order while honoring lazy inputs, list inputs and outputs, output nodes, execution blocking, node expansion, and asynchronous node contracts.
4. WHEN a node produces UI output, data output, multiple list results, partial output, or no output THEN downstream execution and client-visible results SHALL match the cataloged node and executor contract.
5. IF execution raises an exception or structured execution blocker THEN Sim SHALL preserve the prompt and executed-node context, stop the affected execution according to source semantics, and expose a retryable, copyable error without presenting success.
6. WHEN multiple prompts exist THEN queue position, priority or front insertion, prompt numbering, per-client routing, and completion order SHALL match the native compatibility contract.

### Requirement 5: Caching, change detection, interruption, and recovery

Incremental execution must not reuse stale results or lose user intent.

#### Acceptance criteria

1. WHEN a prompt is repeated THEN cache reuse SHALL follow the selected Comfy cache mode, node input signatures, `IS_CHANGED` behavior, node identity, lazy dependency use, and list mapping semantics.
2. WHEN a node, widget, model artifact, input file, plugin manifest or module digest, native registry version, backend, dtype policy, or actually demanded lazy input changes THEN Sim SHALL invalidate exactly the affected cached work.
3. WHEN a user interrupts or cancels queued or running work THEN Sim SHALL cancel the native task tree, distinguish queued cancellation from running fence completion, prevent output commit, and ignore late worker results from the cancelled attempt.
4. IF the native worker is lost during execution THEN Sim SHALL mark the attempt interrupted, reconcile committed outputs and durable cache entries from the journal, and SHALL NOT duplicate a prompt without explicit user intent.
5. WHEN retry is selected THEN Sim SHALL state whether it reuses the original prompt, regenerates the current workflow prompt, or resumes an explicitly restartable native provider task, and SHALL create an auditable new attempt identity where the source does.
6. WHEN Sim or a native worker restarts THEN recoverable queue, history, workflow, cache, and output references SHALL reconcile from durable sources while ephemeral progress and in-flight device work are marked interrupted.

### Requirement 6: Built-in node registry fidelity

Every registered built-in node is a public workflow and extension contract.

#### Acceptance criteria

1. FOR EVERY row in `catalogs/backend-nodes.csv`, Sim SHALL recognize the exact registered identifier, display name, category, description, deprecation or experimental status, and search aliases exposed by the source registry.
2. FOR EVERY cataloged node input, Sim SHALL preserve ordering, socket type, required or optional or hidden status, default, allowed values, minimum, maximum, step, rounding, multiline, force-input, raw-link, tooltip, lazy, list, autogrow, and serialization behavior that the node declares.
3. FOR EVERY cataloged node output, Sim SHALL preserve output type, name, list behavior, tooltip, node-output status, and downstream connection compatibility.
4. WHEN a versioned Rust/WASM plugin intentionally replaces or aliases a built-in node THEN Sim SHALL apply only an explicit signed or user-approved mapping, retain the built-in and plugin versions, and preserve unsupported fields losslessly.
5. WHEN a node is unavailable, disabled, deprecated, experimental, API-backed, or missing a dependency THEN Sim SHALL keep the serialized node, show its availability reason, and prevent an invalid prompt without deleting connections or widgets.
6. THE node contract suite SHALL parameterize schema and execution validation, prompt serialization, output decoding, list mapping, lazy use, change detection, cache behavior, blocking, cancellation, and error behavior for every active registered node row.

### Requirement 7: Native models, formats, sampling, devices, and memory

All cataloged inference and training behavior is a native Sim responsibility.

#### Acceptance criteria

1. FOR EVERY cataloged model family, loader, directory, checkpoint or tensor format, LoRA, VAE, CLIP, ControlNet, upscale, merge, conditioning, latent, image, mask, audio, video, and 3D capability, Sim SHALL use a native descriptor, safe parser, implementation, or explicit unavailable state without narrowing valid serialized names or paths.
2. FOR EVERY cataloged sampler and scheduler identifier, Sim SHALL implement the source algorithm in Rust, serialize the exact identifier, preserve seed and noise controls, and SHALL NOT substitute a merely similar algorithm.
3. WHEN CPU, CUDA, ROCm, Apple Metal, DirectML, Intel XPU, Huawei NPU, Cambricon MLU, Iluvatar CoreX, or multi-device execution is selected THEN Sim SHALL validate an exact native backend capability matrix, display effective device and memory state, and retain portable unknown values without crashing.
4. WHEN dtype, attention, layout, memory mode, offload, mmap, pinned-memory, preview, deterministic, quantization, or performance policy is configured THEN Sim SHALL validate mutual exclusions and report the effective native configuration.
5. IF a model, auxiliary file, accelerator library, codec, or remote API dependency is unavailable THEN Sim SHALL identify the exact dependency and affected nodes, preserve the workflow, and offer only evidence-backed remediation.
6. WHILE offline THE native runtime and cached local models SHALL remain usable without source trees or Python, while API nodes, compatibility-registry lookups, remote models, and cloud actions SHALL fail closed with actionable status.

### Requirement 8: Native HTTP protocol compatibility host

Automation clients and the native UI must share one versioned HTTP contract.

#### Acceptance criteria

1. FOR EVERY row in `catalogs/backend-http-routes.csv`, the protocol layer SHALL model method, legacy and current path, path and query parameters, headers, body schema, response schema, content type, status codes, streaming behavior, permissions, feature gate, and filesystem or queue side effects.
2. WHEN an automation client calls Sim THEN native request decoding and response encoding SHALL preserve permitted unknown fields and SHALL reject malformed required fields with a route-addressable error.
3. WHEN the optional native compatibility host is enabled THEN every enabled route SHALL be served by Sim's Rust services and SHALL match the source status, content type, cache headers, range behavior, upload limits, filename safety, and error body observed for that baseline without proxying to ComfyUI.
4. IF a route is conditional, internal, manager-owned, asset-gated, multi-user-gated, deprecated, or custom-node-provided THEN capability negotiation SHALL expose that condition rather than treating a 404 as an empty success.
5. WHEN an HTTP client times out or loses a mutation response THEN Sim SHALL reconcile the native operation by idempotency key or durable attempt identity before accepting a retry.
6. THE route contract suite SHALL replay cataloged valid, empty, malformed, unauthorized, forbidden, not-found, conflict, oversized, cancelled, and unavailable-dependency cases.

### Requirement 9: WebSocket protocol and preview compatibility

Automation clients depend on an ordered projection of the native event bus.

#### Acceptance criteria

1. FOR EVERY row in `catalogs/backend-websocket-events.csv`, Sim SHALL decode the event identifier, JSON or binary framing, payload fields, target client behavior, ordering constraints, and version or feature conditions.
2. WHEN native status, execution-start, execution-cached, executing, progress, executed, success, interruption, error, or cataloged job transitions occur THEN Sim SHALL emit only legal source-compatible events for the matching client and prompt.
3. WHEN a binary preview frame arrives THEN Sim SHALL validate its event header, image format, optional metadata, length, and prompt or node association before display.
4. IF an event is unknown or has additional fields THEN Sim SHALL retain diagnostic visibility, ignore it safely unless negotiated, and continue processing later valid frames.
5. WHEN an automation socket reconnects THEN Sim SHALL use the applicable client or session identity, project current native status, reconcile queue and history, and discard duplicate or stale events deterministically.
6. THE WebSocket contract suite SHALL test fragmented delivery, coalesced events, out-of-order and duplicate events, malformed JSON, malformed binary frames, disconnects, reconnects, and native-host shutdown.

### Requirement 10: Queue, jobs, history, and output records

Users need consistent control and inspection across legacy prompts and newer
job APIs.

#### Acceptance criteria

1. WHEN queue or job data loads THEN Sim SHALL distinguish running, pending, completed, failed, cancelled, interrupted, unknown, and source-compatible provider states with stable prompt or job identities.
2. WHEN the user queues, reorders where supported, deletes, clears, cancels, interrupts, retries, or reloads a workflow from a record THEN the native UI SHALL dispatch the matching typed runtime command and update only after native acknowledgement or reconciliation; the HTTP host SHALL project the same operation for automation clients.
3. WHEN history is empty, loading, partially available, paginated, filtered, stale, or unavailable THEN the panel SHALL render a distinct state and preserve the user's current selection where valid.
4. WHEN outputs include images, animated images, audio, video, 3D, text, files, or unknown extension output THEN Sim SHALL retain node association, ordering, metadata, URLs, subfolder and type fields, and download or view actions.
5. IF a referenced output is missing, expired, forbidden, externally deleted, or has an unsupported media type THEN the record SHALL remain inspectable and expose recovery or removal without hiding the history entry.
6. WHEN queue and history change concurrently with user actions THEN Sim SHALL merge by native prompt, attempt, and event-sequence identity, not by list position.

### Requirement 11: Files, assets, metadata, and filesystem effects

Local and remote filesystems have different trust and persistence boundaries.

#### Acceptance criteria

1. WHEN importing through GPUI or uploading through the native API host an input, mask, workflow media, model, or other cataloged asset THEN Sim SHALL enforce the source field names, overwrite and duplicate behavior, typed namespace and subfolder rules, size limits, filename normalization, and permission checks.
2. WHEN viewing or downloading input, output, or temporary content THEN Sim SHALL preserve content type, disposition, preview or channel parameters, range behavior, and safe path containment.
3. WHEN an output node writes a file THEN filename prefixing, collision numbering, output or temp directory choice, metadata embedding, sidecar behavior, and history reference SHALL match that node's contract.
4. IF metadata saving is disabled THEN prompt and workflow metadata SHALL be omitted only from formats and nodes governed by that flag, without dropping unrelated file metadata.
5. WHEN the asset subsystem scans, hashes, tags, marks missing, restores, or synchronizes files THEN operations SHALL be capability-gated, cancellable where source behavior permits, and represented as durable asset state.
6. WHEN a local file changes or disappears outside Sim THEN open workflows, model choices, media widgets, output viewers, and indexes SHALL show conflict, missing, or reload state before destructive replacement.

### Requirement 12: Superseded Python and JavaScript extension execution

The former proposal to delegate Python custom nodes and JavaScript web
extensions to ComfyUI or a browser is withdrawn. These criteria retain the
negative production boundary and migration obligations for those source
contracts; Requirement 39 defines the replacement plugin API.

#### Acceptance criteria

1. THE production application SHALL NOT import, discover, execute, or launch Python custom-node modules, Python package environments, or custom-node subprocesses.
2. THE production application SHALL NOT execute JavaScript, TypeScript, DOM widgets, LiteGraph imperative hooks, arbitrary web directories, or Node-based extension hosts for Comfy compatibility.
3. WHEN a workflow references a Python or JavaScript extension identifier THEN Sim SHALL retain its node, fields, widgets, links, and extension data; resolve it only through the deterministic Rust/WASM legacy mapping rules; and otherwise show an exact non-destructive placeholder.
4. WHEN legacy `extra_model_paths` or custom-node model-directory declarations are imported THEN Sim SHALL map approved paths into native typed artifact roots, reject unsafe paths, refresh affected choices and diagnostics, and preserve serialized relative and unknown paths.
5. WHEN API nodes are enabled THEN native provider plugins SHALL isolate secrets, requests, polling, cancellation, cost or credit surfaces, external uploads, provider errors, and per-profile grants.
6. WHEN API nodes are disabled or Sim is offline THEN no native provider plugin request SHALL be initiated and affected nodes SHALL explain the disabled provider, missing grant, secret, entitlement, or connectivity condition without logging sensitive data.

### Requirement 13: Native graph node, link, and widget editing

Workflow authors need GPUI-native editing with Comfy serialization semantics.

#### Acceptance criteria

1. WHEN a user creates a node by library, search, link release, paste, template, drag-and-drop, replacement, or extension action THEN Sim SHALL instantiate the selected registered type with cataloged defaults and one undoable transaction.
2. WHEN a user starts, completes, moves, copies, reconnects, or removes a link THEN Sim SHALL enforce socket compatibility, multiple-connection rules, virtual or union types, dynamic slots, reroutes, snapping, and invalid-drop feedback.
3. FOR EVERY compiled built-in or versioned plugin widget schema, Sim SHALL provide an operable native widget or an exact non-destructive placeholder, preserving prompt serialization separately from workflow serialization.
4. WHEN widget input is empty, invalid, clamped, rounded, dynamic, converted to an input, restored to a widget, or externally updated THEN visible value, serialized value, prompt value, and validation state SHALL remain consistent.
5. WHEN a node is muted, bypassed, pinned, collapsed, resized, renamed, colored, disabled, or selected THEN graph rendering, prompt generation, workflow serialization, and undo or redo SHALL apply the source semantics.
6. IF a node definition changes while the workflow is open THEN Sim SHALL reconcile slots and widgets losslessly, surface conflicts, and require confirmation before discarding unmapped values or links.

### Requirement 14: Graph selection, layout, groups, reroutes, and subgraphs

Complex workflows require deterministic multi-entity operations.

#### Acceptance criteria

1. WHEN the user clicks, shift-clicks, box-selects, selects all, or changes selection through a panel THEN node, group, reroute, and link selection SHALL match the active graph and expose consistent properties.
2. WHEN selected entities move, align, distribute, arrange, duplicate, group, ungroup, collapse, expand, pin, mute, bypass, or delete THEN Sim SHALL commit one reversible command with stable ordering and no partial mutation.
3. WHEN the canvas pans, zooms, fits, centers, follows a dragged link, uses a minimap, or restores a viewport THEN coordinate conversion and persisted scale or offset SHALL remain deterministic across display scale factors.
4. WHEN reroutes are inserted, moved, branched, floated, reparented, or migrated from legacy nodes THEN connectivity, parent IDs, floating type, and deletion behavior SHALL match the applicable workflow schema.
5. WHEN a selection is converted to a subgraph or a subgraph is opened, nested, duplicated, published, unpacked, or removed THEN definitions, instance IDs, exposed inputs, outputs, widgets, navigation breadcrumbs, and nested viewport state SHALL remain valid.
6. IF a dynamic subgraph input or extension behavior is unsupported THEN Sim SHALL preserve the serialized data and identify the limitation without flattening or deleting the subgraph.

### Requirement 15: Commands, gestures, clipboard, focus, and accessibility

Every interaction path must remain discoverable and keyboard operable.

#### Acceptance criteria

1. FOR EVERY row in `catalogs/frontend-commands.csv`, Sim SHALL expose the command in the applicable key context, with cataloged default keybindings, enablement, focus transition, undo boundary, and visible result.
2. FOR EVERY cataloged mouse, wheel, pen, drag-and-drop, context-menu, and modifier gesture, Sim SHALL implement the same functional result or an explicitly documented accessible alternative when platform conventions conflict.
3. WHEN copying or pasting nodes, links, images, workflows, API prompts, text, or files THEN Sim SHALL validate clipboard types, remap identifiers without collisions, preserve supported metadata, and report rejected data without clearing the current graph.
4. WHEN a menu, dialog, popover, picker, editor, graph, widget, or panel opens and closes THEN focus SHALL move to the intended control, remain trapped only where modal, restore to a valid prior control, and never disappear behind the canvas.
5. WHEN using only a keyboard THEN all creation, connection, editing, execution, queue, history, settings, error, and destructive-confirmation workflows SHALL be operable with visible focus.
6. FOR EVERY supported production build, Sim SHALL enable the accessibility platform for Comfy surfaces without requiring `SIM_EXPERIMENTAL_A11Y=1`, and graph entities and controls SHALL expose names, roles, values, state, relationships, errors, progress, and live announcements without relying only on color or pointer hover.

### Requirement 16: Workflow lifecycle, persistence, and schema compatibility

Workflow files are durable compatibility artifacts, not merely editor state.

#### Acceptance criteria

1. WHEN creating, opening, importing, duplicating, renaming, saving, saving as, exporting, closing, or deleting a workflow THEN tabs, dirty state, recent state, local/provider persistence, local files, and destructive confirmation SHALL follow the selected storage provider's contract.
2. WHEN multiple workflow tabs or windows are open THEN each SHALL retain its own document identity, graph navigation, viewport, undo history, execution association, and save target across focus changes.
3. WHEN reading workflow schema 0.4, schema 1, an API-format prompt, a cataloged legacy format, or a future passthrough-compatible version THEN Sim SHALL preserve unknown fields and choose validation or migration by explicit version evidence.
4. WHEN writing schema 0.4 or schema 1 THEN node IDs, graph state, links, reroutes, groups, models, subgraph definitions, widget values, properties, config, extra fields, and renderer metadata SHALL match the selected schema.
5. IF validation or migration fails THEN Sim SHALL keep the original bytes, identify the path and reason, avoid partial replacement, and allow read-only inspection or opening with preserved unsupported data.
6. WHEN autosave, draft persistence, provider save, external file change, restart, crash, or conflict occurs THEN Sim SHALL expose which version is authoritative and offer reload, keep, compare, or save-copy recovery as applicable.

### Requirement 17: Embedded workflows, templates, App Mode, and sharing

Comfy workflows move through media, templates, applications, and cloud stores.

#### Acceptance criteria

1. FOR EVERY row in `catalogs/cross-formats.csv`, Sim SHALL extract cataloged workflow and prompt metadata keys from PNG, WebP, AVIF, SVG, FLAC, MP3, Ogg or Opus, WebM, MP4, MOV, M4V, GLB, latent or safetensors, and JSON containers with the same malformed-data fallback.
2. WHEN media contains a UI workflow, API prompt, both, neither, legacy parameter text, uppercase metadata keys, non-finite Python JSON values, or oversized metadata THEN Sim SHALL select, convert, reject, or upload according to the cataloged behavior and SHALL not execute on import.
3. WHEN exporting through a metadata-capable output path THEN the selected prompt and workflow representation SHALL be embedded only where the source node or format contract supports it and SHALL round-trip through the shared fixture suite.
4. WHEN loading local, bundled, approved-provider, URL, cloud, or Rust/WASM plugin templates THEN Sim SHALL preserve template provenance, thumbnail and model requirements, missing-node handling, and user changes as a new workflow identity.
5. WHEN App Mode or a published application hides graph editing THEN configured inputs, outputs, execution, loading, validation, error, and restore behavior SHALL remain functional and reversible to the permitted editing mode.
6. WHEN sharing, publishing, or cloud-saving is unavailable, unauthenticated, forbidden, conflicted, or partially uploaded THEN Sim SHALL preserve the local workflow and expose retry or copy without claiming publication.

### Requirement 18: Execution UI, previews, errors, and retry

Runtime state must be visible at graph, node, queue, and application levels.

#### Acceptance criteria

1. WHEN execution is queued or running THEN action bars, queue panels, node indicators, progress bars, status icons, browser or window status, and cancellation controls SHALL derive from one prompt state.
2. WHEN node progress or previews arrive THEN Sim SHALL associate them with the correct profile, prompt, node, execution attempt, frame, and output while throttling rendering without dropping final state.
3. WHEN execution succeeds THEN output nodes, history, viewers, badges, notifications, and workflow status SHALL update once, even if success and history reconciliation arrive separately.
4. WHEN validation, node execution, transport, provider, decoding, filesystem, or permission errors occur THEN Sim SHALL show a concise summary, expandable structured details, affected graph entities, copy action, and evidence-backed recovery.
5. WHEN a user navigates from an error or history item THEN Sim SHALL select and reveal the relevant node or output without destroying the previous selection or graph viewport history.
6. WHEN cancellation, retry, dismissal, or a new run follows an error THEN stale highlights and previews SHALL clear only for the superseded attempt, while durable error and history records remain inspectable.

### Requirement 19: Node library, missing dependencies, assets, editors, and viewers

Specialized content must remain operable rather than degrading into opaque
files.

#### Acceptance criteria

1. WHEN the node library loads THEN category tree, search, aliases, recents or favorites where supported, node descriptions, deprecation, experimental state, provider cost, and availability SHALL reflect the selected registry.
2. WHEN nodes, models, media, or widget resources are missing THEN Sim SHALL identify each reference, distinguish not installed from not indexed or not permitted, propose catalog-backed replacement or installation, and require confirmation before rewriting a workflow.
3. WHEN assets are browsed, searched, filtered, sorted, previewed, selected, uploaded, downloaded, renamed, tagged, marked missing, restored, or deleted THEN Sim SHALL preserve native/provider capability, pagination, metadata, and destructive confirmation semantics.
4. WHEN mask, crop, painter, bounding-box, audio-recording, text, seed, color, model, or other specialized widget editing is invoked THEN it SHALL round-trip the node's serialized and prompt value and support cancel without mutation.
5. WHEN image, animated image, HDR, audio, video, 3D, latent, JSON, or unknown output opens THEN the viewer SHALL expose applicable playback, frame, channel, metadata, zoom, download, and error behavior.
6. IF a codec, renderer, GPU path, browser API, or native service is unavailable THEN Sim SHALL retain the asset, expose the missing capability, and offer an external-open or download path where permitted.

### Requirement 20: Application shell and user-visible surfaces

All source surfaces must have an explicit native placement or defer decision.

#### Acceptance criteria

1. FOR EVERY cataloged page, view, route, tab, panel, sidebar, toolbar, action bar, dialog, popover, menu, context menu, notification, toast, status indicator, and error surface, `parity-matrix.md` SHALL name its Sim workspace item, dock panel, modal, popover, status surface, compatibility host, or defer decision.
2. WHEN a surface is empty, loading, partially loaded, stale, unavailable, unauthorized, forbidden, conflicted, or in error THEN Sim SHALL render a distinct state with only valid actions enabled.
3. WHEN a destructive action affects workflow, history, queue, model, installation, snapshot, asset, secret, or account data THEN Sim SHALL identify scope, require the cataloged confirmation, and retain state when cancelled.
4. WHEN a notification or toast represents progress, success, warning, error, update, external change, or recovery THEN duplicate events SHALL coalesce by identity and important failures SHALL remain discoverable after auto-dismiss.
5. WHEN panels or workspace items move, resize, hide, reopen, split, or restore after restart THEN focus, selected profile, active document, scroll, and persisted layout SHALL remain valid.
6. WHEN a feature is unavailable by distribution or flag THEN its surface SHALL either be absent as in the source or visibly gated; it SHALL never offer an action that predictably fails without explanation.

### Requirement 21: Settings, themes, palettes, localization, onboarding, and help

Configuration must be typed, scoped, migratable, and visible.

#### Acceptance criteria

1. FOR EVERY row in `catalogs/frontend-settings.csv` and `catalogs/desktop-settings.csv`, Sim SHALL model key, type, default, enum or bounds, scope, persistence store, availability, restart requirement, side effects, and migration or legacy aliases.
2. WHEN settings are changed through UI, JSON, runtime profile, approved provider, workspace, platform, release-channel, or policy layers THEN Sim SHALL resolve precedence deterministically and show validation errors without discarding valid settings.
3. WHEN theme, palette, link style, grid, canvas, node, density, renderer, preview, menu, sidebar, or experimental settings change THEN affected entities SHALL update at the source-observable time and persist at the documented scope.
4. WHEN locale changes THEN cataloged user-facing strings, node definitions, dates, numbers, plural forms, keyboard labels, errors, and accessibility names SHALL use the selected locale with a deterministic English fallback.
5. WHEN onboarding, tours, release notes, help, documentation, surveys, update prompts, or first-run choices are shown THEN dismissal, completion, version gating, external navigation, and restart persistence SHALL match the cataloged state.
6. IF a setting or feature flag is unknown to an older Sim build THEN its persisted value SHALL remain lossless and inactive rather than being deleted.

### Requirement 22: Authentication, secrets, billing, telemetry, tasks, and feature flags

Remote and paid behavior crosses account and privacy boundaries.

#### Acceptance criteria

1. WHEN authentication is required THEN sign-in, callback, token refresh, sign-out, session expiry, multi-user identity, and unauthorized recovery SHALL be scoped to the selected profile and SHALL never expose tokens in logs, workflows, clipboard defaults, or telemetry.
2. WHEN secrets are created, selected, updated, missing, rejected, or deleted THEN Sim SHALL use an OS-appropriate secret store or an explicitly encrypted fallback and SHALL expose only secret identifiers to workflows and extensions.
3. WHEN credits, billing, pricing, provider cost, cloud entitlement, or paid limits are available THEN Sim SHALL display provider-authoritative values, require confirmation where the source does, and reconcile ambiguous submissions before retry.
4. WHEN telemetry or surveys are disabled, unsupported, offline, or denied THEN Sim SHALL send no associated event; when enabled, cataloged event names and required fields SHALL exclude prompt, workflow, path, secret, and media content unless separately consented.
5. WHEN asynchronous native-provider or cloud tasks run THEN status, progress, cancellation, completion, failure, and retry SHALL remain associated with the initiating profile and visible after the initiating dialog closes.
6. WHEN build, native registry, provider remote-config, entitlement, experiment, or user feature flags change THEN Sim SHALL recompute affected availability, preserve unknown values, and record the source of the effective flag.

### Requirement 23: Frontend and LiteGraph legacy compatibility

Native GPUI cannot silently claim compatibility with JavaScript extension hooks.

#### Acceptance criteria

1. FOR EVERY row in `catalogs/frontend-extensions.csv`, the design SHALL classify the hook as declarative Rust/WASM contribution, legacy identifier mapping, lossless placeholder, documented-only claim, or deliberate defer; no classification SHALL imply JavaScript execution.
2. WHEN a Rust/WASM plugin contributes nodes, widgets, commands, menus, settings, sidebars, routes, lifecycle events, declarative drawing, serialization callbacks, or network services THEN the native host SHALL expose only versioned APIs and explicitly granted permissions.
3. WHEN a workflow requires a web extension, DOM widget, or imperative LiteGraph hook unavailable in native GPUI THEN Sim SHALL retain a non-destructive placeholder with the exact missing extension identity, data, and mapped replacement choices; it SHALL NOT hand execution to an external or embedded browser.
4. IF a Rust/WASM hook traps, hangs, mutates invalid graph state, exceeds resources, requests a forbidden capability, or becomes incompatible THEN Sim SHALL isolate the failure, preserve the workflow, expose diagnostics, revoke affected handles, and keep native editing responsive.
5. WHEN extension-owned fields, widget values, node properties, callbacks, graph versions, or serialized payloads are unknown THEN native load-save SHALL preserve them losslessly.
6. THE extension fixture suite SHALL cover legacy and V3 identifiers, DOM widgets, web directories, commands, menus, settings, routes, serialization callbacks, explicit port mappings, signature and permission denial, trap, hang, cancellation, resource exhaustion, and unresolved placeholders without executing Python or JavaScript.

### Requirement 24: Native runtime installation, adoption, profiles, and onboarding

Desktop parity maps its visible lifecycle to native runtime profiles, workers,
artifacts, plugins, and platform services rather than a Python server.

#### Acceptance criteria

1. WHEN Sim first enables Comfy capability THEN it SHALL reproduce applicable cataloged welcome, terms or privacy, content-root, backend or GPU, legacy migration, progress, native readiness, and recovery decisions for the current platform.
2. WHEN probing existing data THEN Sim SHALL distinguish native runtime profiles, model roots, workflow/output stores, Rust/WASM plugin stores, legacy Comfy installations, incomplete operations, busy resources, incompatible formats, and unsafe paths without modifying them.
3. WHEN adopting or migrating data THEN Sim SHALL preview moved, copied, linked, retained, and conflicting models, plugins, settings, users, workflows, outputs, snapshots, and inactive legacy Python metadata before mutation.
4. WHEN multiple runtime profiles exist THEN Sim SHALL maintain stable identities, names, runtime/schema versions, content roots, ownership, active device group, native API port policy, windows, and per-profile settings.
5. WHEN a new window, popout, or runtime-profile switch occurs THEN Sim SHALL bind it to the intended profile and SHALL not leak queue, worker log, output, model handle, plugin grant, or secret state from another profile.
6. IF installation or migration is cancelled, interrupted, denied, or fails THEN Sim SHALL retain a resumable or safely removable operation record and SHALL not mark the installation ready.

### Requirement 25: Desktop updates, snapshots, rollback, and downloads

Updates must be recoverable across Sim, native backends, model artifacts,
compatibility registries, and Rust/WASM plugins.

#### Acceptance criteria

1. WHEN checking for updates THEN Sim SHALL distinguish the application, native compute backends, codecs, model artifacts, compatibility registry, first-party components, and Rust/WASM plugin versions and channels; legacy ComfyUI/frontend/manager/custom-node versions remain migration evidence only.
2. WHEN downloading an application, backend, codec, model, plugin, registry, or archive THEN Sim SHALL expose source, destination, total and transferred bytes where known, speed, stage, pause or resume support, cancellation, checksum or signature result, and retained partial state.
3. BEFORE a mutating native runtime or plugin update, Sim SHALL create or select the cataloged snapshot boundary and record application/runtime/schema versions, backend manifests, plugin digests and grants, model references, and user-data exclusions.
4. WHEN applying an update THEN Sim SHALL quiesce affected processes, preserve logs, perform atomic or staged replacement, verify readiness and version, and commit success only after health validation.
5. IF update validation fails or the next launch crashes THEN Sim SHALL offer rollback, repair, diagnostics, or retained old version according to available snapshot evidence.
6. WHEN application auto-update requires restart or relaunch THEN window, workflow, installation operation, and unsaved-state handling SHALL use the platform-specific cataloged confirmation and recovery path.

### Requirement 26: Native workers, health, ports, logs, terminal, and recovery

Operators need transparent managed-process behavior.

#### Acceptance criteria

1. WHEN Sim invokes a native worker, signed helper, archive tool, health probe, or platform helper THEN it SHALL record sanitized executable, arguments, working directory, environment source, ownership, start time, progress, exit, and captured output; Python and Git custom-node subprocesses are forbidden.
2. WHEN the optional native API host selects a port THEN Sim SHALL respect configured and detected ports, bind loopback by default, handle probe-to-bind races, and report the effective native URL without treating a port as an execution prerequisite.
3. WHEN native worker health or readiness changes THEN Sim SHALL distinguish process alive, private IPC reachable, backend ready, model index ready, API host ready, degraded, cancelling, hung, exited, and protocol-incompatible states.
4. WHEN logs or the integrated terminal open THEN historical and live stdout or stderr, levels, timestamps, copy, clear, search, download, popout, and bounded retention SHALL follow the cataloged surface.
5. WHEN a native worker or approved helper hangs or exits unexpectedly THEN Sim SHALL capture exit context, mark in-flight operations interrupted, discard uncommitted output, apply bounded automatic recovery policy, and avoid a restart loop.
6. WHEN the app itself crashes or is force-closed THEN the next launch SHALL detect only Sim-owned orphan workers and operation journals, offer terminate, restart, resume eligible operations, repair, or dismiss, and preserve user data.

### Requirement 27: Desktop IPC, preload, menus, windows, and OS integration

Every native bridge contract needs a Rust ownership decision.

#### Acceptance criteria

1. FOR EVERY row in `catalogs/desktop-ipc.csv`, Sim SHALL map channel, direction, arguments, return or event schema, error behavior, permissions, lifecycle, platform, and preload exposure to a Rust service, GPUI action, window event, compatibility bridge, or defer decision.
2. FOR EVERY native menu and title-bar action, Sim SHALL preserve enablement, checked state, role, shortcut, target window or instance, platform placement, and visible side effect.
3. WHEN a file, folder, install location, executable, export target, or adoption source chooser opens THEN initial location, filters, multi-selection, cancellation, permissions, and returned path rules SHALL match the cataloged platform behavior.
4. WHEN windows create, focus, blur, hide, show, minimize, maximize, full-screen, close, navigate, open external links, or spawn popouts THEN lifecycle events and destructive guards SHALL run once in the correct order.
5. WHEN navigation targets local frontend, engine, authentication callback, allowed external URL, download, or untrusted origin THEN Sim SHALL apply the cataloged allow, external-open, deny, or permission decision.
6. WHEN OS integration registers protocols, notifications, recent items, startup behavior, taskbar or dock state, power events, or relaunch arguments THEN behavior SHALL remain platform-gated and reversible.

### Requirement 28: Security, permissions, remote access, and packaging

Process, network, filesystem, extension, and account trust boundaries must be
explicit.

#### Acceptance criteria

1. WHEN binding the native compatibility host THEN loopback SHALL be the default; remote listen, TLS, CORS, reverse-proxy trust, and public exposure SHALL require explicit configuration with the effective risk shown.
2. WHEN resolving any user, workflow, plugin, route, archive, provider-supplied, or legacy path THEN Sim SHALL prevent traversal and unsafe links and SHALL enforce the owning runtime profile's typed roots.
3. WHEN installing or running Rust/WASM plugin code THEN Sim SHALL show source, signature, digest, requested permissions, and provenance; isolate downloads and extraction; prevent archive escape; and distinguish trusted, restricted, blocked, and unknown code.
4. WHEN remote content supplies HTML, SVG, URLs, media metadata, model metadata, logs, or error text THEN Sim SHALL treat it as untrusted and SHALL not execute script or open external navigation without policy.
5. WHEN packaging for Windows, macOS, or Linux THEN data, cache, log, temporary, application, update, backend, codec, model, plugin, and snapshot locations SHALL use platform conventions and support paths with spaces and non-ASCII characters; packages SHALL contain no required Python or ComfyUI runtime.
6. WHEN an operation lacks filesystem, network, account, entitlement, administrator, accessibility, media, or process permission THEN Sim SHALL fail before partial mutation where possible and expose the exact permission and recovery action.

### Requirement 29: Idiomatic GPUI ownership, persistence, and error propagation

The native implementation must fit Sim's existing state and concurrency model.

#### Acceptance criteria

1. THE graph editor SHALL be a serializable workspace item; queue, history, node library, assets, and native runtime operations SHALL use dock panels where persistent visibility is useful; confirmations and bounded editors SHALL use modals or popovers.
2. THE active runtime profile, registry, workflow document, graph selection, execution attempts, outputs, device/model state, and operation journals SHALL have explicit GPUI entity or application-service ownership with no nested entity update and no hidden global mutable graph state.
3. WHEN foreground entities start provider network, parsing, hashing, scanning, worker, tensor, model, or media work THEN background tasks or the native worker SHALL own expensive work, foreground updates SHALL be fallible, and stored task handles or cancellation tokens SHALL match the intended lifetime.
4. WHEN an asynchronous operation fails THEN the error SHALL propagate to a visible entity and durable operation or history state; no fallible operation SHALL be silently discarded.
5. WHEN workspace or application state serializes THEN runtime-profile identity, open workflow items, panel layout, selected tab, graph navigation and viewport, safe drafts, plugin mappings, and recoverable operation references SHALL use versioned DB or settings models with migrations.
6. WHEN settings or external files change THEN observed entities SHALL update through Sim's settings and filesystem watchers, with conflict handling before overwriting dirty state.

### Requirement 30: Backward compatibility, deprecation, feature flags, and uncertainty

Compatibility decisions must remain visible as the source and target evolve.

#### Acceptance criteria

1. WHEN loading a workflow from any cataloged legacy or current schema THEN Sim SHALL preserve unknown nodes, fields, widgets, links, model paths, seeds, metadata, output references, and extension data even when it cannot execute them.
2. WHEN a source behavior is deprecated or dead THEN Sim SHALL recognize it for import or migration where evidence requires, SHALL avoid enabling it by default, and SHALL identify its replacement or retained limitation.
3. WHEN an experimental, developer-only, cloud/paid, platform-specific, or feature-flagged capability is disabled THEN Sim SHALL preserve its persisted state and expose the same or stricter gate without silently promoting it to active.
4. WHEN source and Sim behavior conflict THEN `parity-matrix.md` SHALL name the conflicting observable contract, chosen compatibility boundary, migration, and validation rather than labeling it partial without detail.
5. WHEN a product, legal, security, distribution-size, or unavailable-service decision blocks implementation THEN the feature SHALL remain traced as deferred with owner, decision needed, safe assumed default, and consequence if the assumption changes.
6. WHEN a workflow, native API client, plugin manifest, or compatibility registry is newer than Sim THEN unknown routes, events, port/schema fields, workflow fields, and flags SHALL be preserved or safely ignored; when it is older, unavailable capabilities SHALL remain disabled without data loss.

### Requirement 31: Performance, bounded resources, and responsiveness

Large workflows and long executions must not block the GPUI foreground thread.

#### Acceptance criteria

1. WHEN loading or saving the deterministic large-workflow fixture THEN parsing, validation, migration, layout indexing, and serialization SHALL run off the foreground thread where work exceeds a frame budget and SHALL preserve byte-stable unknown fields.
2. WHILE panning, zooming, selecting, linking, moving, or editing a large graph, GPUI input handling SHALL remain responsive and SHALL virtualize or defer offscreen work without changing hit testing or serialization.
3. WHEN high-frequency progress, preview, log, filesystem, asset, or settings events arrive THEN Sim SHALL coalesce rendering while preserving ordered terminal state and diagnostic counts.
4. WHEN caches for node definitions, thumbnails, outputs, metadata, models, routes, or media reach configured bounds THEN eviction SHALL not remove durable workflow or history data and SHALL be observable through diagnostics.
5. WHEN downloads, hashes, scans, model lists, metadata extraction, or background layout are cancelled THEN resource use SHALL converge to idle and temporary files or tasks SHALL follow the cataloged recovery policy.
6. THE validation plan SHALL record baseline and target budgets for native worker startup, device initialization, model indexing and loading, first execution, workflow load and save, graph interaction, queue acknowledgement, event-to-indicator latency, sampler steps, offload, memory growth, cancellation convergence, and crash recovery.

### Requirement 32: Deterministic validation and coverage gates

Parity claims require side-by-side evidence and registry reconciliation.

#### Acceptance criteria

1. THE development-only conformance suite SHALL run identical deterministic workflows, API prompts, node schemas, tensors, model fixtures, sampler trajectories, media, plugin mappings, failures, and persisted-state fixtures against the source baseline and native Sim and SHALL compare normalized observable results with recorded oracle provenance.
2. THE suite SHALL include domain unit, schema and protocol contract, GPUI interaction, visual state, end-to-end, persistence and restart, failure injection, accessibility, keyboard-only, platform, security, and performance checks applicable to each feature.
3. BEFORE a baseline is accepted, registered nodes, tensor/operator calls, model families, samplers, schedulers, devices, formats, HTTP routes, WebSocket messages, CLI commands and flags, desktop IPC channels, preload APIs, frontend commands, keybindings, menu actions, settings, feature flags, schemas, migrations, persisted formats, and tests SHALL reconcile to catalog totals with zero unexplained deltas.
4. BEFORE a baseline is accepted, every production source file and relevant test across ComfyUI, Frontend, Desktop, comfy-cli, docs, embedded-docs, and Sim SHALL map to feature IDs or an explicit infrastructure, test-only, generated, translated mirror, deprecated/dead, asset, documentation, or out-of-scope classification with a reason.
5. BEFORE a parity decision is accepted, every active or conditional feature SHALL map forward to a criterion, design decision or component, executable implementation task or explicitly non-completion-blocking external release-certification gate, and validation scenario, and every criterion, design decision, task, and external gate SHALL trace back to source evidence; proprietary CoreX enablement SHALL map to the separate `comfy-corex-enablement` specification rather than an executable task in this pack.
6. THE orphan audit SHALL search for unlisted handlers, node and operator registrations, commands, flags, environment keys, routes, settings, localization keys, telemetry events, platform branches, APIs, IPC methods, tests, schemas, generated registrations, documentation claims, and persistence keys and SHALL retain unresolved findings as uncertainty.
7. WHEN source runtime, hardware, account, or provider behavior cannot be exercised THEN runtime-validation metrics SHALL count the feature as not observed even if code, tests, or documentation provide other evidence; documentation alone SHALL never be promoted to executable evidence.
8. THE specification pack SHALL pass `python3 .agents/skills/coding/scripts/validate_spec.py .agents/specs/comfy-parity --require-complete` before handoff.

### Requirement 33: Production-native execution boundary

The release architecture must make the native-only constraint mechanically
verifiable rather than a convention.

#### Acceptance criteria

1. THE production Cargo dependency graph, packaged files, settings, menus, CLI, and runtime code paths SHALL contain no required ComfyUI checkout, Python interpreter, Python package, Node extension host, browser handoff, or outbound ComfyUI connection.
2. WHEN native Comfy capability becomes ready THEN Sim SHALL do so with the network disabled, no Python executable on `PATH`, and all source application directories absent.
3. THE GPUI application SHALL communicate with execution through typed Rust commands and private versioned worker IPC, never through the public Comfy HTTP or WebSocket compatibility surface.
4. THE optional native HTTP/WebSocket host and headless CLI SHALL invoke the same Rust runtime services as GPUI and SHALL NOT forward requests to another Comfy server.
5. WHEN legacy configuration asks Sim to launch or connect to ComfyUI THEN Sim SHALL refuse safely, preserve migratable data, and identify the native replacement without starting a process or network request.
6. THE release gate SHALL fail if binary/package inspection, reverse-dependency analysis, or an isolated runtime trace detects a production Python, JavaScript-extension, ComfyUI-process, or external-Comfy dependency.

### Requirement 34: Tensor operations, layouts, autograd, and random number generation

Native execution requires an explicit backend-neutral tensor contract.

#### Acceptance criteria

1. FOR EVERY tensor-inventory row, Sim SHALL first resolve its canonical target, overload/call signature, and callable-versus-namespace/type/value classification without guessing; FOR EVERY resolved callable operation, Sim SHALL define shape, broadcasting, dtype promotion, accumulation dtype, stride/layout, view-versus-copy, empty/scalar behavior, numerical edge behavior, supported devices, determinism, and structured errors, while reference-only rows SHALL become typed facade contracts rather than executable kernels.
2. THE public node, model, persistence, IPC, and plugin contracts SHALL use Sim-owned tensor descriptors and opaque handles and SHALL NOT expose a third-party framework's Rust types or raw device pointers.
3. WHEN a cataloged training node, custom operation, or gradient-dependent sampler executes THEN native reverse-mode autograd SHALL reproduce forward values, gradients or VJPs, detach/no-grad behavior, saved tensors, checkpoint recomputation, scaling, optimizer updates, and cancellation.
4. WHEN an operation is unsupported for a dtype, layout, or device THEN Sim SHALL return a typed capability error or an explicitly declared deterministic fallback; it SHALL NOT silently change precision, layout, device, or algorithm.
5. WHEN stochastic behavior executes THEN a versioned RNG stream identified by algorithm, seed, counter, device, attempt, node, and phase SHALL produce reproducible values without reading or mutating global process RNG.
6. WHEN tensor work is cancelled, faults, overflows, produces NaN/infinity, or loses its device THEN all owned storage and tape state SHALL converge to a diagnosable terminal state without panics or committed partial outputs.

### Requirement 35: Native device backends and memory management

Device names are compatibility promises only after backend-specific behavior is
certified.

#### Acceptance criteria

1. FOR CPU, CUDA, ROCm, Apple Metal, DirectML, Intel XPU, Huawei NPU, Cambricon MLU, Iluvatar CoreX, and cataloged multi-device modes, Sim SHALL publish and enforce an operation, dtype, layout, memory, determinism, and fallback capability matrix; CPU and Apple Metal SHALL be the certified baseline backends; EVERY other non-CPU backend SHALL remain optional and SHALL advertise support only after its adapter crate, binding strategy, ABI/SDK floor, required symbols and struct layouts, targets, discovery order, build/link behavior, package/license/signing policy, unsafe owner, and typed unavailable result are pinned and its applicable release-certification gate passes; Huawei NPU SHALL authorize `libascendcl.so` only from its nonempty reviewed callable surface and SHALL treat the exact `libruntime.so` image only as an explicit `required_by = ascendcl` dependency contract that can never satisfy callable lookup or callable authorization; Iluvatar CoreX SHALL compile as a zero-symbol fail-closed adapter that reports canonical typed `Unbound` until the separate `comfy-corex-enablement` specification lawfully supplies and validates the proprietary IXRT/IXBLAS inputs; deterministic CPU conformance and verification of any supplied CPU attestation against the independently configured trust anchor SHALL be implementation gates, while creating or refreshing a signed CPU hardware attestation SHALL remain an external release-certification gate; Intel XPU execution SHALL use only an owned retained-certification session over the exact reviewed Level Zero and oneDNN calls, SHALL keep production unavailable without canonical certification, and SHALL NOT use host arithmetic or a test harness as a production fallback; ONE serialized dependency task SHALL own all adapter dependency sections and Cargo.lock, after which parallel backend tasks SHALL run `--locked` and SHALL NOT mutate dependencies.
2. THE native worker SHALL own device contexts, streams, events, allocators, tensor/model lifetimes, and device-loss recovery separately from GPUI rendering devices.
3. WHEN planning an attempt THEN the memory planner SHALL reserve weights, activations, attention workspaces, previews, cache entries, staging buffers, and output encoding before dispatch where sizes are knowable.
4. WHEN pressure rises THEN model/tensor LRU eviction, lazy mmap weights, pinned staging, layer/group offload, copy-on-write patches, multi-device placement, and peer-copy fallback SHALL follow the selected cataloged policy and expose effective placement.
5. IF allocation or execution reports OOM THEN Sim SHALL discard uncommitted work, apply only bounded lower-memory retries, record each plan and failure, and end with an actionable error rather than a restart loop.
6. WHEN cancellation reaches non-preemptible device work THEN the attempt SHALL remain `cancelling` until its fence completes, late values SHALL be discarded, and memory accounting SHALL return to the documented steady-state bound.

### Requirement 36: Native model artifacts, formats, families, and patches

Every model-family row requires a safe native discovery and execution path.

#### Acceptance criteria

1. THE native artifact index SHALL discover canonical model roots and approved imported paths, normalize relative paths without traversal, track canonical path/size/mtime/hash identity, watch external changes, and retain missing artifacts as diagnosable references.
2. FOR safetensors, weights-only PyTorch archives, GGUF, and every cataloged weight/config/tokenizer format, Sim SHALL use bounded native parsers; it SHALL never execute pickle reducers or arbitrary model-supplied code.
3. FOR EVERY cataloged model family, Sim SHALL provide a native descriptor, state-dict detector, configuration and tokenizer rules, shape-reduced fixture, weight mapping, forward checkpoints, supported dtype/device matrix, and exact invalid/partial/mismatch errors.
4. WHEN loading is lazy, mmap-backed, quantized, offloaded, sharded, or multi-device THEN artifact lifetime, data integrity, cancellation, progress, memory accounting, and retry behavior SHALL remain observable and deterministic.
5. FOR LoRA, LoHa, LoKr, OFT, ControlNet, VAE, CLIP, upscaling, merges, quantization, and model patches, Sim SHALL implement ordered copy-on-write patch graphs and include artifact and patch identity in cache keys.
6. IF a model format, architecture, auxiliary file, backend, or codec is unavailable THEN Sim SHALL preserve the workflow and exact artifact reference, identify the missing native contract, and offer only verified remediation.

### Requirement 37: Native samplers, schedulers, latents, and conditioning

Algorithm identifiers may not be implemented as aliases to approximate methods.

#### Acceptance criteria

1. FOR EVERY one of the 44 cataloged sampler identifiers, Sim SHALL implement the exact update equations, callback ordering, ancestral/noise behavior, boundaries, and errors in Rust and compare every intermediate step against deterministic fixtures.
2. FOR EVERY one of the 9 cataloged scheduler identifiers, Sim SHALL reproduce exact sigma arrays, defaults, denoise/start/end rules, zero-step behavior, and invalid parameter errors.
3. FOR EVERY one of the 33 cataloged latent formats, Sim SHALL preserve channel layout, scale/shift constants, empty-latent construction, device/dtype behavior, encode/decode boundaries, and serialized identifiers.
4. WHEN seeds, noise controls, leftover noise, variation, stochastic rounding, or sampler-specific randomness are used THEN Sim SHALL allocate independent versioned RNG phases and reproduce cataloged CPU reference tensors where exact comparison is supported.
5. WHEN conditioning, guidance, regions, masks, hooks, ControlNet, CLIP, or model patches alter denoising THEN Sim SHALL preserve ordering, broadcasting, batching, list/lazy behavior, and cache identity at every step.
6. WHEN a sampler encounters cancellation, NaN/infinity, extreme sigmas, invalid schedules, device loss, or OOM THEN it SHALL stop at a declared safe point, commit no partial output, and expose the exact step and recoverability.

### Requirement 38: Native worker, executor, cache, cancellation, and recovery

The graph executor and tensor computation graph are separate native subsystems.

#### Acceptance criteria

1. THE prompt compiler SHALL produce a typed demand-driven DAG that validates exact node schemas, links, values, hidden inputs, output reachability, list mapping, lazy demand, blockers, expansion, asynchronous nodes, and UI outputs.
2. THE execution supervisor SHALL schedule dependency-ready node tasks, preserve queue priority and per-client identity, bound concurrency by device and effect class, and emit a monotonic event sequence.
3. THE cache key SHALL include implementation/version, normalized declared inputs, actually demanded dependency identities, artifacts and patches, backend/device/dtype policy, plugin digest/API, feature configuration, and compatibility change token.
4. SIDE-EFFECTING nodes SHALL use prepare/commit transactions; cancellation, failure, worker loss, or application crash before commit SHALL leave no successful history record or partial final output.
5. THE recovery journal SHALL record immutable prompt and attempt identity, durable cache/output commits, cancellation state, worker identity, and restart eligibility; arbitrary in-flight kernels SHALL be marked interrupted rather than resumed.
6. WHEN a worker crashes or becomes incompatible THEN Sim SHALL isolate the fault from GPUI, revoke handles, reconcile durable state, apply bounded restart policy, and allow an explicit new attempt without duplicating effects.

### Requirement 39: Versioned Rust and WASM plugin APIs

Rust source traits and the WASM Component Model replace Python and JavaScript
extension execution.

#### Acceptance criteria

1. FIRST-PARTY or curated native plugins SHALL implement a versioned source-level Rust trait and be statically linked or packaged as signed Sim components; Sim SHALL NOT promise a stable Rust dynamic-library ABI.
2. THIRD-PARTY plugins SHALL use a versioned WIT Component Model API whose manifest declares plugin/API versions, digest/signature/provenance, node IDs and versions, explicit ports, legacy identifiers, permissions, determinism/cache/effect policy, and declarative UI contributions.
3. EVERY plugin port SHALL have a stable port ID, direction, canonical `namespace:name@major` type registered to one value family and evolution rule, scalar/list cardinality, required/optional/hidden/lazy status, default, serialization, and accepted legacy names; the API SHALL distinguish absent optional values from present empty lists and SHALL define bounded indexed input access, one-time resource transfer, output push/finish, zero/one/list validation, and terminal handle revocation without positional inference.
4. LEGACY identifier resolution SHALL apply workflow-pinned, user-approved, signed-registry, unique-installed, then unresolved-placeholder precedence; opening SHALL use a non-destructive projection and saving SHALL rewrite only after explicit acceptance with mapping provenance.
5. WASM SHALL receive invocation-scoped opaque handles and separately granted bounded filesystem, network/provider, secret, clock, randomness, model, transactional output, sanitized logging, declarative UI-state, and route capabilities with typed request/response/error contracts, quotas, cancellation, rollback, and no raw path, pointer, ambient credential, socket, or host-process authority.
6. WHEN a plugin traps, hangs, exhausts fuel/memory/table/instances/channels/output quota, violates a capability, is cancelled, returns invalid cardinality/type/data, or loses a response/transaction THEN the host SHALL terminate or revoke that invocation, abort uncommitted output and route effects, preserve unaffected work, expose sanitized diagnostics, and remain responsive.

### Requirement 40: Native API host and comfy-cli compatibility

Automation compatibility is a Rust service surface over the native runtime.

#### Acceptance criteria

1. FOR EVERY cataloged ComfyUI HTTP route and WebSocket event selected for parity, Sim SHALL provide a native handler or explicit compatibility response using the same queue, history, artifact, execution, and plugin services as GPUI.
2. FOR EVERY reachable comfy-cli command, option, argument, schema, event, error code, configuration key, format, and lifecycle state, Sim SHALL map it to a native CLI/API behavior, an architecture-conflicting migration response, or a documented explicit defer.
3. WHEN running headless THEN `sim comfy serve` and native CLI commands SHALL initialize the same Rust runtime and worker protocol without starting GPUI, Python, Node, Electron, a browser, or another Comfy server.
4. WHEN exposing the native host remotely THEN loopback SHALL remain default and authentication, authorization, TLS, CORS, rate/size limits, path safety, and provider/plugin route grants SHALL apply before side effects.
5. WHEN CLI/API schemas contain unknown or newer fields THEN Sim SHALL preserve permitted data, reject malformed required fields, and report version negotiation without silently discarding commands or workflow content.
6. THE CLI/API contract suite SHALL cover happy, empty, invalid, offline, unauthorized, timeout, cancellation, retry, interrupted download, worker restart, and ambiguous mutation recovery paths.

### Requirement 41: Native media, metadata, previews, and outputs

Media compatibility must not depend on an FFmpeg command or source server.

#### Acceptance criteria

1. THE native codec registry SHALL identify image, animated image, HDR, mask, depth, audio, video, preview, and 3D formats by bounded content parsing rather than trusting extensions alone.
2. WHEN loading or saving PNG, WebP, FLAC, or another cataloged metadata carrier THEN Sim SHALL preserve prompt/workflow metadata, unknown permitted chunks, color/orientation/timing/channel data, and the source disable-metadata behavior.
3. WHEN producing previews or final outputs THEN scaling, format, quality, alpha/channel rules, frame/sample timing, filenames, collision numbering, temporary/final namespace, progress, and transactional commit SHALL match the node contract.
4. NATIVE codec libraries accessed through reviewed Rust FFI MAY be packaged, but required media paths SHALL NOT launch FFmpeg or another command-line transcoder; licensing, signing, platform support, and unavailable-codec errors SHALL be explicit.
5. WHEN media is malformed, hostile, oversized, truncated, externally changed, permission-denied, unsupported, cancelled, or the worker crashes THEN Sim SHALL bound resources, commit no partial final asset, preserve inspectable references, and expose recovery.
6. WHEN specialized GPUI mask, crop, paint, bounding-box, image, HDR, audio, video, or 3D surfaces edit media THEN keyboard, pointer, focus, accessibility, undo, serialization, and external-change behavior SHALL operate on the same native asset identity.

### Requirement 42: Development-only Comfy conformance oracle

The source products are evidence and test oracles, never production services.

#### Acceptance criteria

1. COMFY source launchers, adapters, normalization code, and live comparison harnesses SHALL reside only in development/test support targets that production crates do not depend on directly or transitively.
2. WHEN an oracle fixture is recorded THEN it SHALL include source product, declared version or tree fingerprint, command/configuration, input hashes, platform/device/dependency state, normalized outputs, tolerance policy, and any unresolved nondeterminism.
3. WHEN documentation conflicts with executable code or tests THEN executable evidence SHALL control conformance expectations and the documented claim SHALL remain separately classified rather than merged.
4. WHEN an oracle cannot run because of dependencies, hardware, credentials, paid services, or platform THEN the affected feature SHALL remain not observed and SHALL use checked-in test/code evidence or explicit uncertainty without fabricated output.
5. ORACLE comparisons SHALL inspect schemas, state transitions, tensor shape/dtype/layout, operator outputs and gradients, model checkpoints, sigma arrays, noise, sampler trajectories, media metadata, side effects, errors, cancellation, and recovery rather than final-image similarity alone.
6. THE release test suite SHALL consume recorded fixtures without requiring source trees, Python, JavaScript extension execution, external Comfy connectivity, accounts, credentials, or network access.

### Requirement 43: comfy-cli, docs, and embedded-docs evidence closure

The added repositories must be versioned, covered, and classified without
turning prose into executable evidence.

#### Acceptance criteria

1. THE baseline SHALL record deterministic versions or source-tree fingerprints for `comfy-cli`, `docs`, and `embedded-docs`, their nested instructions, fingerprint exclusions, declared tool/runtime constraints, and any version skew with consuming products.
2. EVERY comfy-cli production command, callback, alias, option, argument, schema, event, error code, environment/configuration key, OpenAPI mapping, format, lifecycle state, test, and source file SHALL reconcile to a stable catalog row or explicit disposition.
3. EVERY docs and embedded-docs source file SHALL be classified as authoritative English content, translated/generated mirror, node documentation, executable tooling/test, schema/configuration, asset, staging, infrastructure, deprecated/dead, or documented-only claim with a reason.
4. WHEN documentation names a command, route, node, format, lifecycle, extension contract, or capability THEN the inventory SHALL link corroborating executable/test evidence or retain it as `documented-only` or `unverified`; wording alone SHALL never produce `observed`, `test-backed`, or `code-inferred` evidence.
5. WHEN docs navigation, redirects, localization, generated node identifiers, OpenAPI paths, package pins, or embedded/backend registries disagree THEN the exact delta and availability SHALL remain in reconciliation catalogs and the parity matrix.
6. THE source-coverage gate SHALL account for every file and relevant test in all three added repositories and SHALL retain runtime/test failures, translation issues, missing dependencies, and uncorroborated routes as explicit evidence limitations.

### Requirement 44: Native built-in and API-node implementation closure

Schema recognition is not node execution parity.

#### Acceptance criteria

1. FOR EVERY active or conditional row in `catalogs/backend-nodes.csv`, Sim SHALL provide a native Rust implementation or a native provider-backed implementation whose identifier, ports, widgets, validation, list/lazy behavior, output-node/effect status, cache/change contract, cancellation, and errors are independently testable.
2. FOR EVERY inactive, deprecated, dead, experimental, unavailable-dependency, or unverified node row, Sim SHALL preserve serialized identity and data, reproduce the source gate, and SHALL NOT silently activate or discard it.
3. NODE implementations SHALL exchange typed native values and opaque handles for tensors, models, CLIP, VAE, ControlNet, conditioning, latents, images, masks, audio, video, 3D, primitives, lists, provider tasks, and preserved unknown values.
4. EVERY node-family implementation task SHALL own disjoint source and fixture paths; central registry generation SHALL occur in a later serialized task from checked-in descriptors so same-wave tasks do not conflict.
5. THE node closure report SHALL reconcile registered local and API-node totals, native implementations, explicit provider implementations, gated placeholders, per-node schema tests, per-node behavior tests, and unresolved evidence with zero unexplained rows.
6. NO node SHALL be marked equivalent from representative family testing alone; its exact catalog row SHALL pass schema, success, boundary, validation, failure, cancellation, cache/change, persistence, and side-effect checks applicable to that node.

## Non-blocking assumptions

- The recommended production architecture is a Sim-owned native Rust control
  plane with one isolated Sim-owned Rust compute worker per selected device
  group. Worker isolation does not make the runtime external.
- A Sim-owned backend-neutral tensor API is the compatibility boundary. A
  native Rust compute ecosystem may be used behind it only after exact
  operator, autograd, dtype, device, memory, and packaging validation.
- The first end-to-end slice is `LoadImage -> ImageScale -> ImageInvert ->
  PreviewImage -> SaveImage` on the native CPU backend. The next slice is the
  generated `sd15-tiny-v1` contract: SD15 `COMFY-MODEL-0117`, the full SD1
  tokenizer, pinned reduced CLIP/UNet/VAE dimensions and key map, SD15 latent,
  Euler, normal scheduler, fixed seed, and named intermediate checkpoints.
- Cloud and paid implementation depends on approved service contracts,
  credentials handling, entitlements, and product policy. The safe default is
  disabled with lossless workflow preservation.

## Explicit planning exclusions

- No Comfy or Sim implementation is performed by this specification task.
- No source application, dependency lockfile, model, custom node, or user data
  is modified.
- No live account, paid provider, update server, registry mutation, or remote
  installation is used for validation without separate authorization.
- Architecture-private equivalence is not required; observable contracts and
  compatibility boundaries are required.
