# Validation: Native Rust/GPUI Comfy parity

## Outcome and oracle boundary

Validation compares identical deterministic fixtures against source behavior and native Sim, but source applications are development-only oracles. Recorded fixtures, normalization code, fingerprints, environments, and numeric tolerances are checked in; release tests do not require source trees, Python, JavaScript extension execution, external Comfy connectivity, accounts, paid services, credentials, or network access.

A result is `observed` only when a recorded runtime probe actually ran. Existing tests may support `test-backed`; executable code may support `code-inferred`; documentation without corroboration remains `documented-only`; unavailable hardware/accounts/dependencies remain explicit uncertainty.

## Deterministic fixture families

- Lossless legacy/current workflow and prompt JSON, malformed graphs, list/lazy/blocker/expansion nodes, unknown fields, legacy plugin IDs, tabs, drafts, conflicts, and embedded PNG/WebP/FLAC metadata.
- Tensor operation matrices over shape, dtype, layout, device, empty/scalar, NaN/infinity, views/copies, gradients, RNG seeds/counters/phases, cancellation, and unsupported capability.
- Safe model-format corpus plus one shape-reduced detector/loader/forward fixture for each of 94 model families, patch/quantization variants, 33 latent formats, and all invalid/partial/mismatch/OOM cases.
- Analytical tiny denoiser trajectories for all 44 samplers and exact scalar sigma arrays for all 9 schedulers, including boundary, NaN, cancellation, and callback ordering.
- Every local/API node row with exact schema and independently applicable success, boundary, list/lazy, validation, cache/change, effect, failure, cancellation, persistence, and recovery cases.
- HTTP, WebSocket, comfy-cli, Desktop IPC/preload/menu/window, settings, flags, formats, plugins, providers, codecs, localization, and source-file closure fixtures.

## Environment and platform matrix

| Dimension | Required coverage | Evidence rule |
| --- | --- | --- |
| Production boundary | Packaged release with no source trees, no Python on PATH, no Node extension host/browser, network disabled | Must pass native readiness and first slice; static inspection alone is insufficient |
| Devices | Certified baseline: deterministic CPU and signed Apple Metal; optional CUDA, ROCm, DirectML, XPU, NPU, MLU, CoreX, and multi-device boundaries | Optional live-hardware rows stay external release gates; CoreX must remain compiled, zero-symbol, typed Unbound until its future specification is delivered |
| Platforms | macOS, Windows, Linux package/data/permission/window/backend/codec behavior | Native branch plus platform test/lab; macOS observation cannot promote Windows/Linux |
| Modes | GPUI local, headless native host, remote client of Sim host, offline, provider/cloud gated | GPUI and API share one native runtime; no external Comfy mode |
| Accessibility | Keyboard-only, semantic tree, screen reader, focus, contrast, zoom, reduced motion | Production application must not default to inaccessible |
| Failure injection | Invalid, empty, loading, partial, cancellation, timeout, OOM, device loss, plugin trap/hang, worker crash, app crash, conflict, permission denial, restart | Terminal state, durable record, resource convergence, and visible recovery are mandatory |

## Numeric and media comparison policy

Exact comparison is required for identifiers, schemas, shapes, strides, dtypes, layouts, integer/boolean values, CPU sigma arrays, versioned CPU RNG where supported, state transitions, errors, filenames, metadata, side effects, and ordering. Floating tolerances are recorded per operation/dtype/backend with absolute/relative/ULP rules and intermediate checkpoints. Final-image similarity cannot conceal operator, model, sampler, scheduler, or RNG substitution. Media comparison records decoded pixels/samples/frames/geometry plus color, orientation, timing, channels, metadata chunks, and container effects.

## Numeric performance and convergence budgets

Release-profile measurements record hardware, driver, OS, power mode, fixture digest, warmup, sample count, median, p95, and peak bytes. On the pinned CI reference CPU, the 512×512 five-node image slice completes in at most 2 seconds and the 32×32 four-step tiny diffusion slice in at most 5 seconds. A 1,000-node graph keeps pointer/key dispatch p95 at or below 8 ms and rendered-frame p95 at or below 16.7 ms during pan/zoom; a 10,000-node stress graph keeps interaction p95 below 50 ms. Worker CPU readiness is at most 2 seconds, a warm certified device worker at most 5 seconds, API/event projection p95 at most 50 ms, and the first preview arrives within 250 ms after a preview-capable checkpoint. Cancellation becomes visibly `cancelling` within 100 ms; CPU/WASM cooperative work terminates within 1 second, provider work within its 5-second adapter deadline, and non-preemptible device work either reaches a fence or the worker is terminated within 10 seconds. Ten seconds after terminal cancellation/crash, live task/file/handle counts return to baseline and accounted memory is within the larger of 1 percent or 64 MiB of baseline. A budget miss is a release failure unless the affected conditional platform row remains explicitly uncertified.

## Certification scope and external release gates

Implementation completion requires deterministic CPU conformance, strict verification of any supplied CPU attestation, the retained signed Apple Metal baseline evidence, and fail-closed unavailable-path validation for every compiled optional adapter. Creating or refreshing the signed CPU attestation requires the approved external PKCS#8 key and is not an implementation task. Live certification for ROCm, DirectML, XPU, NPU, MLU, CUDA, and multi-device modes remains an external release gate that may be claimed only on the exact hardware and driver environment. CoreX ABI, semantic, production-integration, signing, and hardware work is transferred to `.agents/specs/comfy-corex-enablement/`; this pack validates only its compiled zero-symbol structural adapter and canonical typed `Unbound` state. An absent gate is never represented as a pass and does not weaken production fail-closed behavior.

## Scenarios

### VAL-CATALOG-001: Catalog and source closure

- Type: catalog.
- Fixture: Pinned source trees, checksum-locked base snapshot inputs, and every checked-in generator.
- Command/runner: `python3 .agents/specs/comfy-parity/regenerate_all.py --check-twice`.
- Procedure and expected signal: Run `regenerate_all.py --check-twice`, compare hashes, verify the source-snapshot manifest, reconcile registries and every source-file disposition against master feature IDs, and fail on stale output, unstable IDs, orphan, collision, or duplicate IDs without pretending missing base extractors ran.
- Pass artifact: exit status 0 from the exact runner after the source-snapshot manifest matches and both complete regeneration passes produce no changed paths. The checked-in generated outputs and command result are the evidence; this command-only gate emits no separate target JSON artifact.

### VAL-CANCEL-001: Canonical cancellation ownership

- Type: domain/architecture.
- Fixture: The repository-wide Comfy cancellation definition/call-site inventory plus tensor, plugin, headless, transport, and worker adapters.
- Command/runner: `cargo test -p comfy_test_support val_cancel_001`.
- Procedure and expected signal: Require exactly one production cancellation-state definition in comfy_types; exercise monotonic clone visibility, domain-error mappings, first-writer reason preservation, transport shutdown, worker Cancel projection, and the 16 MiB frame/4 MiB event bounds without treating the development oracle token as production.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-cancel-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-DESKTOP-001: Desktop-to-GPUI behavior mapping

- Type: platform/GPUI.
- Fixture: Desktop IPC, menus, windows, choosers, settings, and platform fixtures.
- Command/runner: `cargo test -p comfy_ui --features test-support val_desktop_001`.
- Procedure and expected signal: Exercise mapped native actions on macOS, Windows, and Linux; compare visible states, lifecycle ordering, cancellation, and persisted results.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-desktop-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-DOMAIN-001: Persistence schemas and migrations

- Type: domain.
- Fixture: Versioned DB/settings/workspace fixtures.
- Command/runner: `cargo test -p comfy_runtime val_domain_001`.
- Procedure and expected signal: Round-trip every version, restart at each transition, inject corrupt/partial writes, and preserve unknown fields.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-domain-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-DOMAIN-002: Workflow and embedded-media formats

- Type: domain.
- Fixture: Legacy/current JSON plus PNG, WebP, FLAC, and other carriers.
- Command/runner: `cargo test -p comfy_runtime val_domain_002`.
- Procedure and expected signal: Import, migrate, save, reopen, compare unknown fields and metadata, and test malformed/oversized/non-finite input.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-domain-002.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-WORKFLOW-OWNERSHIP-001: Workflow and metadata authoritative ownership

- Type: architecture/domain/recovery.
- Fixture: Every raw workflow parser, non-finite JSON compatibility helper, embedded-metadata parser/writer, workflow save reducer, and their production call sites.
- Command/runner: `cargo test -p comfy_test_support val_workflow_ownership_001`.
- Procedure and expected signal: Search the entire repository and require WorkflowFormatDocument, normalize_json_non_finite, MetadataDocument, and WorkflowSaveCoordinator to be the sole owners of their declared behavior; reject the superseded WorkflowDocument and PublicationState definitions, direct coordinator-state mutation, repeated carrier parsing/writing, and copied non-finite token logic; prove graph, prompt, storage, PNG, asset, and metadata adapters preserve source bytes, token coercion, validation, save/conflict transitions, and no-execution import semantics; require a byte-stable zero-failure/zero-skip artifact twice.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-workflow-ownership-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-DOMAIN-003: Graph command semantics

- Type: domain.
- Fixture: Deterministic graph command sequences.
- Command/runner: `cargo test -p comfy_runtime val_domain_003`.
- Procedure and expected signal: Compare command result, undo/redo, link typing, groups, subgraphs, serialization, and invalid operations.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-domain-003.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-DOMAIN-004: Native execution reducers

- Type: domain.
- Fixture: Prompt, attempt, queue, history, cache, event, and cancellation fixtures.
- Command/runner: `cargo test -p comfy_runtime val_domain_004`.
- Procedure and expected signal: Explore legal and illegal transitions, concurrency interleavings, retry identity, late events, and restart reconciliation.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-domain-004.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-DOMAIN-005: Settings, flags, and localization

- Type: domain.
- Fixture: All settings, defaults, flags, locales, and precedence layers.
- Command/runner: `cargo test -p comfy_runtime val_domain_005`.
- Procedure and expected signal: Verify validation, precedence, external change, restart, unknown-value preservation, and English fallback.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-domain-005.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-DOMAIN-006: GPUI ownership and task lifetime

- Type: domain/GPUI.
- Fixture: Entity graph and controlled executor fixtures.
- Command/runner: `cargo test -p comfy_runtime val_domain_006`.
- Procedure and expected signal: Detect nested updates, leaked tasks, dropped required tasks, stale updates, silent errors, and incorrect profile ownership.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-domain-006.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-DOMAIN-007: Templates, App Mode, and sharing

- Type: domain.
- Fixture: Local/bundled/provider/plugin templates and publication fixtures.
- Command/runner: `cargo test -p comfy_runtime val_domain_007`.
- Procedure and expected signal: Verify provenance, isolation, permissions, missing dependencies, cancellation, conflict, and restart behavior.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-domain-007.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-DOMAIN-008: Paths, archives, permissions, and content trust

- Type: security.
- Fixture: Traversal, symlink, archive-bomb, hostile metadata, and permission corpus.
- Command/runner: `cargo test -p comfy_runtime val_domain_008`.
- Procedure and expected signal: Fuzz bounded parsers and prove no root escape, script execution, secret leak, or partial destructive mutation.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-domain-008.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-RUNTIME-PERSISTENCE-001: Native runtime persistence foundation

- Type: domain/persistence.
- Fixture: Versioned native attempt envelopes, inactive legacy migration records, and every historical Comfy runtime DB prefix.
- Command/runner: `cargo test -p comfy_runtime val_runtime_persistence_001`.
- Procedure and expected signal: Apply every migration prefix to empty and nonempty stores; require superseded profile, workspace, and generic-mapping rows to be quarantined losslessly; round-trip typed profile-scoped attempts and inactive migrations across restart; preserve nested unknown fields; redact legacy secrets recursively; and reject invalid updates before overwrite.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-runtime-persistence-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-RUNTIME-SETTINGS-001: Native runtime profile settings foundation

- Type: domain/settings.
- Fixture: Registered Sim defaults plus merged native runtime profile settings.
- Command/runner: `cargo test -p comfy_runtime val_runtime_settings_001`.
- Procedure and expected signal: Verify canonical profile identity, typed device/memory/API/plugin/provider policy, Sim precedence reuse, partial-parse error retention, inactive future values, exact unknown-field round trips, duplicate/version/bind rejection, and fail-closed mapping into production initialization.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-runtime-settings-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-RUNTIME-TRUST-001: Native runtime trust foundation

- Type: domain/security/architecture.
- Fixture: Canonical permission, signature, provider, secret, API exposure, navigation, FFI, artifact-root, archive-parser, and restricted-pickle ownership fixtures.
- Command/runner: `cargo test -p comfy_runtime val_runtime_trust_001`.
- Procedure and expected signal: Verify exact requested-capability sealing, cryptographic signature evidence without caller booleans, closed/profile-scoped providers, opaque secret identifiers, safe remote exposure and navigation, registry-issued callable and dependency-only FFI certification, rejection of dependency contracts from callable lookup or authorization, ArtifactRoot-only generic path validation, focused model-archive parsing, and one versioned restricted-pickle policy; retain deterministic source/ownership digests and zero network or external-process use.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-runtime-trust-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-FOUNDATION-001: Native workspace foundation

- Type: build/domain.
- Fixture: Locked Cargo metadata, native crate manifests, feature forwarding, generated module manifests, and production reverse dependencies.
- Command/runner: `cargo test -p comfy_test_support val_foundation_001`.
- Procedure and expected signal: Check every foundation crate and worker binary under the lockfile, compile each isolated accelerator forwarding feature, verify deterministic generated module manifests, reject undeclared downstream manifest dependencies, and prove production reverse dependencies exclude development-only oracle/source launchers without claiming packaged release closure.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-foundation-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-E2E-001: Superseded external path release guard

- Type: release E2E.
- Fixture: Packaged Sim with legacy connection state.
- Command/runner: `cargo test -p comfy_test_support val_e2e_001`.
- Procedure and expected signal: Prove the former external-server slice is absent: no Comfy connection UI/request occurs, data is preserved, and native migration is offered.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-e2e-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-E2E-002: Native worker lifecycle

- Type: E2E.
- Fixture: Packaged Sim with deterministic CPU backend plus every compiled signed accelerator selection.
- Command/runner: `cargo test -p comfy_test_support val_e2e_002`.
- Procedure and expected signal: Start, ready, stop, restart, crash, recover, and quit without Python, source trees, network, or a public protocol loopback; for each accelerator require the exact signed-package-to-certified-session-to-semantic-backend chain, typed unavailable before Ready when host evidence is absent, and no CPU retry or relabeling.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-e2e-002.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-PROTOCOL-001: Private native worker protocol

- Type: protocol/security.
- Fixture: Every versioned worker envelope, backend-capability DTO, fatal diagnostic, lifecycle transition, and output proposal.
- Command/runner: `cargo test --locked -p comfy_types worker_protocol; cargo test --locked -p comfy_worker --test ipc_framing`.
- Procedure and expected signal: Round-trip every current frame, preserve pinned legacy discriminants, reject version skew, malformed lengths, opaque extensions, oversized events and diagnostics, and require typed path-free backend unavailability before dispatch.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-protocol-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-LEGACY-ENGINE-001: Read-only legacy engine migration

- Type: architecture/domain/security.
- Fixture: Bounded legacy installation evidence plus every production Comfy lifecycle definition and call site.
- Command/runner: `cargo test --locked -p comfy_test_support --test no_python_engine val_legacy_engine_001`.
- Procedure and expected signal: Require one legacy-installation migration owner; reject oversized, malformed, secret-bearing, mutable, active, filesystem-probing, process-launching, network-connecting, or independently persisted evidence; prove every requested Python/Git/Comfy lifecycle action is refused, safe settings projection delegates to NativeRuntimeProfile, every reusable field names its canonical owner and requires explicit acceptance, RuntimeSupervisor remains the only Comfy process owner, and the zero-failure artifact is byte-stable twice.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-legacy-engine-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-E2E-003: Rust/WASM plugin compatibility

- Type: E2E/security.
- Fixture: Signed, unsigned, old/new WIT, legacy-ID, trap, hang, and denied-grant plugins.
- Command/runner: `cargo test -p comfy_test_support val_e2e_003`.
- Procedure and expected signal: Verify explicit ports, mapping order, non-destructive open, permission isolation, resource bounds, cancellation, and placeholders.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-e2e-003.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-E2E-004: Native updates and rollback

- Type: E2E/recovery.
- Fixture: Backend, codec, model, registry, plugin, and application update fixtures.
- Command/runner: `cargo test -p comfy_test_support val_e2e_004`.
- Procedure and expected signal: Pause, resume, cancel, corrupt, crash, validate, commit, roll back, and restart from every journal stage.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-e2e-004.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-E2E-005: Provider, secret, and cloud isolation

- Type: E2E/security.
- Fixture: Local fake providers and disabled/offline profiles.
- Command/runner: `cargo test -p comfy_test_support val_e2e_005`.
- Procedure and expected signal: Verify auth expiry, secret redaction, consent, cost confirmation, timeout reconciliation, and zero requests while disabled/offline.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-e2e-005.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-001: Keyboard-only graph authoring

- Type: GPUI interaction/accessibility.
- Fixture: Representative and large graphs.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_001`.
- Procedure and expected signal: Create, search, connect, edit, move, group, run, inspect, undo, and save without a pointer; assert focus and semantic announcements.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-002: Pointer and gesture graph editing

- Type: GPUI interaction.
- Fixture: Node, link, group, reroute, minimap, and viewport fixtures.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_002`.
- Procedure and expected signal: Exercise click, multi-select, drag, drop, pan, zoom, link insertion/reconnect, cancellation, and boundary hit testing.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-002.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-003: Clipboard and focus transitions

- Type: GPUI interaction.
- Fixture: Internal, JSON, media, file, and hostile clipboard payloads.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_003`.
- Procedure and expected signal: Copy/cut/paste across windows and fields, restore focus, preserve unknown data, and reject unsafe payloads.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-003.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-004: Workflow tabs and storage UI

- Type: GPUI interaction/persistence.
- Fixture: Dirty local/provider workflows and conflicts.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_004`.
- Procedure and expected signal: Exercise create/open/import/save/save-as/close/reopen/autosave/external change/crash with confirmations and focus restoration.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-004.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-005: Queue, progress, preview, error, and retry UI

- Type: GPUI interaction.
- Fixture: Deterministic profile-scoped execution snapshots, events, acknowledgements, and catalog dispositions.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_005`.
- Procedure and expected signal: Reconcile all 119 queue feature rows, 9 execution-placement commands, 17 job/run menus, and 25 execution-relevant component surfaces; exercise the production-registered dock panel and graph/status/error projections for empty, loading, partial, stale, unavailable, queued, running, cancelling, succeeded, failed, cancelled, interrupted, unknown, and provider states; prove acknowledgement-before-mutation, active-profile synchronization, cancellation-safe destructive confirmation, typed output recovery/removal through the durable canonical projection, structured error copy and ExternalNavigationPolicy-gated navigation, retry identity, accessible focus/progress, cross-profile and stale-sequence rejection, and canonical reduction of a large progress/preview stream with bounded rendering and final-state preservation.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-005.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-006: Node library and missing dependency UI

- Type: GPUI interaction.
- Fixture: Complete, filtered, deprecated, missing-node/model/media/plugin fixtures.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_006`.
- Procedure and expected signal: Verify search, creation, replacement, preservation, remediation, keyboard navigation, drag/drop, and feature gates.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-006.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-007: Native media editors and viewers

- Type: GPUI interaction/visual.
- Fixture: Image, HDR, mask, audio, video, 3D, latent, text, JSON, and unknown outputs.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_007`.
- Procedure and expected signal: Verify load/edit/undo/save/error/cancel/external-change behavior plus focus, accessibility, and deterministic renders.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-007.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-008: Settings, onboarding, help, and themes

- Type: GPUI interaction/visual.
- Fixture: All settings and presentation states.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_008`.
- Procedure and expected signal: Verify defaults, validation, search, keyboard access, localization, contrast, restart-required state, dismissal, and persistence.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-008.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-009: Native operations, logs, and recovery UI

- Type: GPUI interaction.
- Fixture: Worker/update/download/crash operation streams.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_009`.
- Procedure and expected signal: Verify progress, cancel, diagnostics, bounded logs, copy/search/export, popout, relaunch, and restart recovery.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-009.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-010: Window, chooser, menu, and OS lifecycle

- Type: GPUI/platform.
- Fixture: Platform matrix and destructive-state fixtures.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_010`.
- Procedure and expected signal: Verify menu state, shortcuts, chooser filters/cancel, focus, close guards, navigation policy, notifications, and relaunch order.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-010.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-011: Accessibility and localization audit

- Type: accessibility.
- Fixture: Every route, panel, dialog, popover, menu, graph control, and editor.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_011`.
- Procedure and expected signal: Run semantic-tree, screen-reader, keyboard-only, focus-order, contrast, reduced-motion, zoom, and English-fallback checks.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-011.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-012: Graph shell command and placement foundation

- Type: GPUI interaction/catalog.
- Fixture: Every frontend command, keybinding, menu, and component-surface row plus the native graph workspace.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_012`.
- Procedure and expected signal: Reconcile stable command identities, labels, availability, exact generated native placement, and later executable owners with no heuristic fallback; prove every menu and component surface retains an explicit parity-matrix `place:` or `defer:` decision naming that same owner; prove forward and reverse trace closure covers exactly the four source catalogs; load the Comfy keymap only in the graph context; dispatch the already-owned graph commands through real GPUI actions; exercise visible later-owned feedback; and reject unknown or unavailable commands without a silent no-op.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-012.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-013: Production accessibility bootstrap

- Type: GPUI/accessibility.
- Fixture: Production Sim application construction, Comfy initialization, keymap ordering, and static menu registration.
- Command/runner: `cargo test -p sim --features test-support val_gpui_013`.
- Procedure and expected signal: Prove production application construction never disables or environment-gates GPUI accessibility; construct that path on a test/headless platform where platform thread rules permit, otherwise verify the compiled helper and exact production source boundary; verify idempotent Comfy initialization, the scoped built-in keymap load order before user overrides, and registered non-placeholder menu actions. Native graph semantics remain exercised by VAL-GPUI-012 and the later whole-application VAL-GPUI-011 audit.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-013.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-014: Native graph context menu closure

- Type: GPUI interaction/catalog.
- Fixture: Exactly 55 actionable graph/canvas/node/group/selection/reroute/slot/subgraph rows plus 8 consumed graph-context infrastructure prerequisites.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_014`.
- Procedure and expected signal: Open the actual assigned GPUI surfaces through pointer and keyboard paths; reconcile every generated source condition, item kind, feature binding, visible availability, initial focus, accessible label/role/checked/disabled/expanded state, canonical dispatch outcome, undo boundary, destructive confirmation, and cancellation result. Require all eight typed infrastructure prerequisites at their real production registry, adapter, conversion, merge, dropdown, and renderer call sites; digest and reconcile the exact parity-matrix row for all 63 identities; and fail on any literal pass shortcut, deferred prerequisite, fallback, duplicate, failed row, or skip.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-014.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-GPUI-015: Native declarative frontend extension compatibility

- Type: GPUI interaction/plugin/catalog.
- Fixture: All 59 frontend extension rows, every installed signed UI declaration, and the 17-case compatibility fixture matrix.
- Command/runner: `cargo test -p comfy_ui --features test-support val_gpui_015`.
- Procedure and expected signal: Reconcile exact generated classifications and typed read-only surfaces; project only live signature-verified ComponentHost inventory; preserve exact bounded payloads and identities in accessible placeholders for unknown, malformed, or duplicate contributions; surface inventory failure without stale state; prove SDK, host, Sim adapter, and GPUI owners remain disjoint; and reject JavaScript, DOM, LiteGraph, browser, Python, source-runtime, capability-grant, action-dispatch, settings-persistence, or parallel lifecycle paths. Require a byte-stable zero-failure/zero-skip artifact twice.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-gpui-015.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-HTTP-001: Native HTTP route contracts

- Type: protocol.
- Fixture: Every backend-http-routes row.
- Command/runner: `cargo test -p comfy_api val_http_001`.
- Procedure and expected signal: Replay valid, empty, malformed, unauthorized, forbidden, not-found, conflict, range, oversized, timeout, cancel, and ambiguous mutation cases against Rust handlers.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-http-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-WS-001: Native WebSocket contracts

- Type: protocol.
- Fixture: Every backend-websocket-events row.
- Command/runner: `cargo test -p comfy_api val_ws_001`.
- Procedure and expected signal: Check JSON/binary framing, ordering, fragmentation, duplicate/stale events, reconnect, previews, cancellation, unknown fields, and host shutdown; after a server close frame, require peer acknowledgement, a bounded timeout, or a recognized post-close peer teardown so concurrent ping/data frames cannot turn clean shutdown into a reset.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-ws-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-NODE-001: Per-node schema closure

- Type: node contract.
- Fixture: Every registered and inactive node row.
- Command/runner: `cargo test -p comfy_nodes val_node_001`.
- Procedure and expected signal: Compare exact identifiers, display/category, every input/output field, flags, defaults, constraints, list/lazy/output-node status, availability, and object-info projection.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-node-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-NODE-002: Per-node native behavior closure

- Type: node execution.
- Fixture: A deterministic fixture for every active/conditional node row.
- Command/runner: `cargo test -p comfy_nodes val_node_002`.
- Procedure and expected signal: Check success, boundaries, validation, list/lazy mapping, cache/change, effects, cancellation, error, unavailable dependency, persistence, and recovery per node; representative-only evidence fails.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-node-002.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-RECOVERY-001: Database and settings crash recovery

- Type: failure injection.
- Fixture: Write-ahead and migration failpoints.
- Command/runner: `cargo test -p comfy_test_support val_recovery_001`.
- Procedure and expected signal: Terminate at every persistence phase and verify atomicity, backup, repair, unknown-field retention, and visible state.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-recovery-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-RECOVERY-002: Workflow save and external-change conflicts

- Type: failure injection.
- Fixture: Dirty files and provider revisions.
- Command/runner: `cargo test -p comfy_runtime val_recovery_002`.
- Procedure and expected signal: Race save/autosave/external writes/crash and verify compare, reload, keep, save-copy, and no silent overwrite.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-recovery-002.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-RECOVERY-003: Native worker crash

- Type: failure injection.
- Fixture: Kill worker at every node, tensor, model, sampler, preview, and output phase under the deterministic CPU backend and every compiled signed accelerator selection.
- Command/runner: `cargo test -p comfy_test_support val_recovery_003`.
- Procedure and expected signal: Verify GPUI survives, attempt interrupts, handles revoke, partial outputs do not commit, bounded restart applies, retry is explicit, and every accelerator replacement repeats package verification, host observation, registry certification, session construction, and readiness without retaining stale resources or falling back to CPU.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-recovery-003.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-RECOVERY-004: Ambiguous native API mutation

- Type: failure injection/protocol.
- Fixture: Drop persistence/response completion around enabled canonical native mutations and probe unavailable mutation families.
- Command/runner: `cargo test -p comfy_api val_recovery_004`.
- Procedure and expected signal: Reconcile enabled mutations from canonical command receipts before retry, prove no duplicate prompt, and prove unavailable output-upload, userdata-delete, and payment routes fail before canonical side effects without synthetic substitutes.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-recovery-004.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-RECOVERY-005: Filesystem and asset recovery

- Type: failure injection/architecture.
- Fixture: Missing/renamed/permission-denied/corrupt inputs and outputs plus the entire-repository root, index, and output-publication ownership surface.
- Command/runner: `cargo test -p comfy_test_support val_recovery_005`.
- Procedure and expected signal: Preserve references, update the canonical ArtifactIndex, reject unsafe paths once through ArtifactRoot, offer verified recovery, avoid partial commits, and prove AssetService is only an enrichment adapter while OutputCommitter alone performs final publication.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-recovery-005.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-RECOVERY-006: Settings and locale external changes

- Type: failure injection.
- Fixture: Concurrent file/DB/policy changes.
- Command/runner: `cargo test -p comfy_test_support val_recovery_006`.
- Procedure and expected signal: Verify precedence, conflict presentation, watcher coalescing, valid-state preservation, and restart convergence.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-recovery-006.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-RECOVERY-007: Native download and update journal

- Type: failure injection.
- Fixture: Model/plugin/backend/codec/registry update stages.
- Command/runner: `cargo test -p comfy_test_support val_recovery_007`.
- Procedure and expected signal: Cancel or crash at every stage, then resume, discard, repair, roll back, and verify integrity without claiming false readiness.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-recovery-007.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-RECOVERY-008: Task cancellation and resource convergence

- Type: failure injection/performance.
- Fixture: Controlled tasks, plugins, providers, codecs, and device fences.
- Command/runner: `cargo test -p comfy_test_support val_recovery_008`.
- Procedure and expected signal: Cancel at every await/fence and prove handles, memory, files, channels, and UI state converge within documented bounds.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-recovery-008.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-RECOVERY-009: Orphan native worker recovery

- Type: failure injection/platform.
- Fixture: Crash parent before/during worker shutdown.
- Command/runner: `cargo test -p comfy_test_support val_recovery_009`.
- Procedure and expected signal: On restart identify only Sim-owned workers, refuse foreign processes, terminate or recover safely, and prevent restart loops.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-recovery-009.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-NATIVE-BOUNDARY-001: Native-only release boundary

- Type: release/security.
- Fixture: Release dependency graph, package manifest, binary strings, runtime trace, and isolated host.
- Command/runner: `cargo test -p comfy_test_support val_native_boundary_001`.
- Procedure and expected signal: Fail on production Comfy/Python/Node-extension/browser/external-Comfy paths; run ready and first slice without network, PATH Python, or source trees.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-native-boundary-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-NODE-REGISTRY-001: Read-side native node descriptor registry

- Type: node/architecture.
- Fixture: Every registered and inactive node catalog row, generated built-in descriptor IDs, object-info projection, and early image/diffusion slice membership.
- Command/runner: `cargo test -p comfy_nodes val_node_registry_001`.
- Procedure and expected signal: Round-trip the exact 789 registered and 12 inactive rows, preserve every catalog field and non-executable status, reject malformed/duplicate catalogs, require deterministic generated descriptor membership and exact early slices, and prove the read-side registry exposes no mutable plugin registration or executable-dispatch authority.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-node-registry-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-TENSOR-001: Tensor operation matrix

- Type: tensor/domain.
- Fixture: Every cataloged operation across supported dtype/layout/device combinations.
- Command/runner: `cargo test -p comfy_tensor val_tensor_001`.
- Procedure and expected signal: Compare shapes, strides, values, promotions, copies/views, empty/scalar, NaN/inf/rounding, errors, fallback, cancellation, and determinism; reject unknown typed-reference targets, cross-leaf owner/module substitutions, unsealed compiled records, placeholder or malformed structured semantics, missing or mismatched evidence, and symlinked evidence components.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-tensor-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-NUMERIC-FORMATS-001: Native numeric formats and attention

- Type: tensor/model/device/architecture.
- Fixture: Nine catalog dtypes, PyTorch-compatible promotion, four quantization contracts, four attention contracts, canonical ownership mappings, and checked-in source-derived fixtures.
- Command/runner: `cargo test --locked -p comfy_model --test numeric_formats val_numeric_formats_001`.
- Procedure and expected signal: Verify exact dtype identity/storage/codec boundaries, round-to-nearest-even promotion and conversion behavior, NaN/inf and typed unsupported cases, deterministic INT8/MXFP8/NVFP4 quantize-dequantize plus mixed metadata v1, native SDP and split attention values/masks/layout/workspace/cancellation, and explicit optimized-backend fallback. Require one DType, quantization, and attention owner with checked worker/plugin/backend adapters and no second tensor, allocator, cache, planner, cancellation, or production Python/JavaScript path.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-numeric-formats-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-AUTOGRAD-001: Native autograd

- Type: tensor/domain.
- Fixture: Analytical functions, custom VJPs, training nodes, optimizers, gradient-dependent samplers, and the canonical Task 102 QuantLinear fixture bound by exact path and SHA-256.
- Command/runner: `cargo test -p comfy_tensor val_autograd_001; cargo test --locked -p comfy_test_support --test autograd_breadth`.
- Procedure and expected signal: Compare forward, gradients, saved tensors, detach/no-grad, checkpoint recompute, mixed-precision scaling, updates, errors, and cancellation; execute a recorded derivative graph and second backward for every custom function whose fixture declares analytical higher-order support; prove every first-order-only or once-differentiable policy rejects before its VJP, recomputation, or publication; require the aggregate comfy_test_support runner to reject QuantLinear path, digest, schema, owner, deletion, rename, duplication, or unused-delegation mutations and execute all 22 strict cases through the canonical comfy_model adapter.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-autograd-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-RNG-001: Versioned RNG streams

- Type: tensor/domain.
- Fixture: Seed/counter/device/node/phase fixtures.
- Command/runner: `cargo test -p comfy_tensor val_rng_001`.
- Procedure and expected signal: Compare exact CPU sequences where supported, distribution checkpoints elsewhere, stream independence, retry identity, stochastic rounding, and cancellation.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-rng-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-DEVICE-001: Native device baseline and fail-closed optional backends

- Type: platform/device.
- Fixture: Deterministic CPU conformance, the supplied certified Apple Metal baseline artifact, every optional D27 adapter on non-target or missing-runtime hosts, and external lab evidence only where it is actually supplied.
- Command/runner: `cargo test --locked -p comfy_tensor --all-features val_device_001`.
- Procedure and expected signal: Compile, check, test, and lint every feature boundary. CPU must execute the complete deterministic operation, dtype, layout, transfer, memory, event, cancellation, and error matrix. Any supplied CPU attestation must pass strict canonical parsing and signature verification against the independently configured trust anchor; implementation or device drift leaves the external signed-attestation gate unclaimed but never skips current CPU conformance. Apple Metal retains its signed live-hardware baseline artifact. Other accelerator live observations remain external release-certification gates. CoreX must compile as the zero-symbol structural adapter, reject every certificate projection, expose no loader or kernel, and report canonical typed Unbound on every host until the separate future specification is completed. No unavailable host, fake harness, package receipt, feature flag, or compiled adapter may be relabeled as certified or available.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-device-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row. The Apple Metal baseline retains its signed artifact under `catalogs/native-device-certification/`; a supplied CPU artifact must verify cryptographically but may remain explicitly stale as an external release gate; optional accelerator artifacts are accepted only as external target-lab evidence.

### VAL-MEMORY-001: Memory planning and OOM

- Type: device/performance/recovery/architecture.
- Fixture: Deterministic canonical-allocator snapshots, all five catalog memory modes, synthetic multi-device allocation traces, named fences, injected OOM/device pressure, and the packaged native image worker.
- Command/runner: `cargo test --locked -p comfy_tensor val_memory_001; cargo test --locked -p comfy_worker --test memory_conformance val_memory_001; cargo test --locked -p comfy_worker --test ipc_framing packaged_worker_reports_preflight_oom_without_dispatch_or_restart`.
- Procedure and expected signal: Check checked reservation classes and durable-baseline accounting; exact typed catalog modes; transactional LRU eviction; mmap, pinned, and pageable offload; preferred placement, peer copy, and host staging; bounded monotonic retry; named-fence cancellation and late-value rejection; device-loss revocation; and steady-state convergence. Prove the attempt policy reads the canonical allocator snapshot without allocating storage or duplicating the model cache/cancellation owner. A rejected live preflight must emit one actionable failure with no graph progress or output proposal, leave the worker backend-ready, and shut down without recovery or restart.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-memory-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-MODEL-FORMAT-001: Safe model formats

- Type: model/security.
- Fixture: Valid, truncated, corrupt, hostile, oversized, sharded, mmap, safetensors, restricted PyTorch, GGUF, config, and tokenizer files.
- Command/runner: `cargo test -p comfy_model val_model_format_001`.
- Procedure and expected signal: Compare tensors and metadata, bound resources, reject executable pickle, preserve errors, cancel safely, and detect external changes.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-model-format-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-CLIP-001: Native tokenizer and CLIP closure

- Type: model/tensor/architecture.
- Fixture: Every generated tokenizer, prompt-weighting, text-encoder, vision-encoder, detector, loader, and projection contract row.
- Command/runner: `cargo test --locked -p comfy_model val_clip_001`.
- Procedure and expected signal: Independently verify pinned source and symbol digests; execute one source-derived valid and invalid fixture per row; cover architecture and artifact binding, masks, layers, pooling, dtype/device, cancellation, OOM, workspace convergence, and production call-site ownership; publish the versioned cumulative exact-task/exact-contract schema, where a truthful partial artifact claims only its passed rows; require zero failures and zero skips for every claimed task result.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-clip-001.json` using schema version 1 with validation and environment identity, truthful partial or passed overall status, aggregate summary, current sole-producer path and SHA-256, current per-task implementation paths and SHA-256 values, and unique exact contract records binding contract ID, owning task ID, pinned source and symbol SHA-256 values, passed status, and non-empty unique case IDs. Every claimed task result has zero failures and skips; partial artifacts claim only their exact passed rows.

### VAL-PATCH-001: Native PatchGraph contract closure

- Type: model/tensor/patching.
- Fixture: Every generated PatchGraph payload, semantic, and family-equation contract row.
- Command/runner: `cargo test --locked -p comfy_model val_patch_001`.
- Procedure and expected signal: Independently verify pinned source and symbol digests; execute every generated patch contract through the canonical PatchGraph and caller ExecutionContext; cover ordering, dtype, device, cancellation, OOM rollback, workspace convergence, and authoritative ownership; publish the versioned cumulative exact-task/exact-contract schema, where a truthful partial artifact claims only its passed rows; require zero failures and zero skips for every claimed task result.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-patch-001.json` using schema version 1 with validation and environment identity, truthful partial or passed overall status, aggregate summary, current sole-producer path and SHA-256, current per-task implementation paths and SHA-256 values, and unique exact contract records binding contract ID, owning task ID, pinned source and symbol SHA-256 values, passed status, and non-empty unique case IDs. Every claimed task result has zero failures and skips; partial artifacts claim only their exact passed rows.

### VAL-CONTROLNET-001: Native ControlNet contract closure

- Type: model/tensor/conditioning.
- Fixture: Every generated ControlNet and T2I adapter contract row.
- Command/runner: `cargo test --locked -p comfy_model controlnet::tests`.
- Procedure and expected signal: Independently verify pinned source and symbol digests; execute every generated ControlNet contract through the canonical checked chain; cover strength, hint preprocessing, batch projection, fixed-slot merge ordering, VAE and latent delegation, identity, dtype/device, cancellation, OOM rollback, workspace convergence, and authoritative ownership; publish the versioned cumulative exact-task/exact-contract schema with zero failures and zero skips.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-controlnet-001.json` using schema version 1 with validation and environment identity, truthful partial or passed overall status, aggregate summary, current sole-producer path and SHA-256, current per-task implementation paths and SHA-256 values, and unique exact contract records binding contract ID, owning task ID, pinned source and symbol SHA-256 values, passed status, and non-empty unique case IDs. Every claimed task result has zero failures and skips; partial artifacts claim only their exact passed rows.

### VAL-CONDITIONING-001: Native conditioning integration closure

- Type: model/sampler/runtime/worker.
- Fixture: Every generated conditioning-value and guidance contract plus their production native-diffusion integration.
- Command/runner: `CARGO_INCREMENTAL=0 cargo test --locked -p comfy_test_support --test native_conditioning_integration val_conditioning_001`.
- Procedure and expected signal: Independently verify pinned source and symbol digests and execute every generated conditioning and guidance contract through the canonical prebound typed runtime bundle. Require caller-cancellable aggregate cache-identity discovery with zero model loads or private workspace; exact provider, bundle, and handle binding for model/tokenizer, CLIP, VAE identity and execution, conditioning PatchGraph/model execution, and ControlNet execution; mutation-based cache separation and stale warm-provider rejection before execution; structural Cancelled versus ResourceExhausted preservation through conditioning, CLIP, model, VAE, and ControlNet wrappers; and zero tensor, output, or proposal publication plus workspace convergence on either failure. Publish the versioned cumulative exact-task/exact-contract schema with zero failures and zero skips.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-conditioning-001.json` using schema version 1 with validation and environment identity, truthful partial or passed overall status, aggregate summary, current sole-producer path and SHA-256, current per-task implementation paths and SHA-256 values, and unique exact contract records binding contract ID, owning task ID, pinned source and symbol SHA-256 values, passed status, and non-empty unique case IDs. Every claimed task result has zero failures and skips; partial artifacts claim only their exact passed rows.

### VAL-PATCH-ADAPTER-001: Native patch loading and merge adapter closure

- Type: model/tensor/patching/architecture.
- Fixture: The exact 14 source-fingerprinted model-patcher, LoRA-loader, key-discovery, prefetch, and Model/CLIP merge rows.
- Command/runner: `cargo test --locked -p comfy_model --test patch_adapters`.
- Procedure and expected signal: Independently verify every pinned source and selected-symbol digest; execute one source-derived valid and invalid fixture per row; cover checked many-alias and sliced key bindings, load precedence and diagnostics, immutable PatchGraph composition and per-key projection, all seven merge formulas and exclusions, recursive aligned prefetch, dense and quantized replacement, caller context/cancellation/OOM, and failure atomicity. Search the repository and prove patches delegates family recognition to weight_adapter, ordering/equations/digests to PatchGraph, codecs/identity/materialization to quantization, and allocation/device/cancellation/publication to their canonical owners; require a byte-stable zero-failure/zero-skip artifact twice.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-patch-adapter-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-VAE-001: Native VAE architecture closure

- Type: model/tensor/architecture.
- Fixture: Every generated VAE selector, profile, image, video, audio, structured-output, tiling, detector, and loader contract row.
- Command/runner: `cargo test --locked -p comfy_model val_vae_001`.
- Procedure and expected signal: Independently verify pinned source and symbol digests; execute one source-derived valid and invalid fixture per row; cover geometry, dtype/device, ModelStore binding, tile execution, cancellation, OOM/retry delegation, workspace convergence, and production call-site ownership; publish the versioned cumulative exact-task/exact-contract schema, where a truthful partial artifact claims only its passed rows; require zero failures and zero skips for every claimed task result.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-vae-001.json` using schema version 1 with validation and environment identity, truthful partial or passed overall status, aggregate summary, current sole-producer path and SHA-256, current per-task implementation paths and SHA-256 values, and unique exact contract records binding contract ID, owning task ID, pinned source and symbol SHA-256 values, passed status, and non-empty unique case IDs. Every claimed task result has zero failures and skips; partial artifacts claim only their exact passed rows.

### VAL-WEIGHT-ADAPTER-001: Native weight-adapter runtime closure

- Type: model/tensor/autograd/architecture.
- Fixture: The exact source-fingerprinted weight-adapter registry, trainable-base, bypass, Adapter, and Diff catalog rows.
- Command/runner: `cargo test --locked -p comfy_model --test weight_adapter_runtime`.
- Procedure and expected signal: Independently verify every pinned source and selected-symbol digest; execute one source-derived valid and invalid fixture per row; cover source-compatible initialization, caller-owned RNG commit, additive and transform forwards, analytical reverse traversal through the canonical AutogradTape, saved-value mutation witnesses and release, linear and convolution bypass geometry, static PatchGraph projection, quantized materialization delegation, dtype/device/layout rejection without CPU fallback, caller-authorized workspace, cancellation, OOM/retry delegation, and no partial value or gradient publication. Search the complete repository and prove comfy_model::weight_adapter owns only registry and runtime planning while PatchGraph, quantization, TensorBackend, workspace authority, CancellationToken, worker retry, AutogradTape, GradientStore, persistence, and final transactions retain sole authority; require a byte-stable zero-failure/zero-skip artifact twice.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-weight-adapter-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-MODEL-REGISTRY-001: Read-side native model descriptor registry

- Type: model/architecture.
- Fixture: Every backend-models catalog row and the exact model-family inventory subset.
- Command/runner: `cargo test -p comfy_model val_model_registry_001`.
- Procedure and expected signal: Round-trip all 211 catalog rows and 94 model-family rows with exact identities, source fields, statuses, ambiguous-identifier candidates, malformed/duplicate rejection, deterministic ordering, and zero executable, parser, artifact-index, cache, or dispatch ownership.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-model-registry-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-MODEL-FAMILY-FOUNDATION-001: Authoritative model-family foundation

- Type: model/architecture.
- Fixture: Canonical family identity, source projection, registration, profile, state-dictionary transaction, build, forward-program, patch, memory, and ownership fixtures.
- Command/runner: `cargo test -p comfy_model val_model_family_foundation_001`.
- Procedure and expected signal: Require the source-fingerprinted 94-row development projection to be exact and byte-stable without importing ComfyUI; prove immutable data-only plan selectors and the native domain reject invalid identities, ordinals, profiles, source bounds, dimension add/multiply/exact-divide errors, staged-reference cycles, undeclared or missing components and required keys, key collisions, incomplete or overlapping split/assembly, and unsupported tensor transformations; prove split/narrow/concat/transpose/permute/reshape/expand/round/constant/arange mechanics delegate to comfy_tensor; prove every checked state-dictionary plan stages then atomically commits or rolls back on error/cancellation; and require ModelStore, ArtifactIndex, PatchGraph, MemoryPlanner, DType, and CancellationToken to retain their existing ownership.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-model-family-foundation-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-MODEL-DETECTION-001: Native parsed-model detection integration

- Type: model/architecture.
- Fixture: Parsed safetensors, restricted PyTorch, GGUF, diffusers, prefix-selection, ambiguous, malformed, cancellation, and cache fixtures.
- Command/runner: `cargo test -p comfy_model val_model_detection_001`.
- Procedure and expected signal: Build bounded immutable detection facts from LoadedModel tensor metadata and parser-owned format metadata without rereading or reparsing artifacts; port the source configuration and prefix-selection semantics into checked Rust selectors; resolve only through ModelFamilyRegistry; prove model layout and state-plan selection are key-derived and cannot be selected or overridden by caller metadata; for the source-declared weight-compatible WAN21 CausalAR and WAN21 FlowRVS rows, require a matching base-WAN tensor signature followed respectively by the exact parsed boolean `transformer.causal_ar=true` or exact bounded `transformer.model_type=flow_rvs` selector, and reject missing, false or different, malformed, injected, or unrelated metadata; reject unknown dtype, overflow, malformed dimensions, no-match and ambiguous-match cases with typed errors; and prove ModelStore remains the sole parsed-model cache while model_family remains the sole family-resolution owner.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-model-detection-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-MODEL-FAMILY-ROW-001: One native model-family row

- Type: model.
- Fixture: The assigned catalog row, source projection, registration module, deterministic fixture, and named shape-reduced checkpoints.
- Command/runner: `cargo test -p comfy_model val_model_family_row_001`.
- Procedure and expected signal: Require exact feature/name/source-ordinal/provenance identity; execute the assigned row through parsed-model detection, profile selection, transactional component mapping, checked build, every named checkpoint, patching, supported dtype/device cases, memory/OOM, cancellation, malformed/partial/ambiguous inputs, and ownership assertions without claiming aggregate breadth.
- Pass artifact: exit status 0 plus one deterministic artifact per executed fixture under `target/comfy-parity/val-model-family-row-001/`, each containing exact source/fixture/provenance digests, environment/backend identity, per-case results, and zero failures or skips.

### VAL-MODEL-FAMILY-001: All model families

- Type: model.
- Fixture: Shape-reduced fixture for each of exactly 94 family rows.
- Command/runner: `cargo test -p comfy_model val_model_family_001`.
- Procedure and expected signal: Require exact catalog/source/module/registration/test/fixture/provenance identity closure with zero skips; execute every row through native parsed-model detection, configuration/profile selection, transactional state-dictionary mapping, checked build, named layer checkpoints, dtype/device matrix, partial/mismatch errors, patches, cancellation, and OOM.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-model-family-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-SAMPLING-FOUNDATION-001: Authoritative sampler and scheduler foundation

- Type: sampling/architecture.
- Fixture: Canonical sampler, scheduler, sampling-profile, noise-phase, registry, plan, observed-step/final-commit and bounded adaptive-attempt transactions, callback, and runtime-adapter fixtures.
- Command/runner: `cargo test -p comfy_sampler val_sampling_foundation_001`.
- Procedure and expected signal: Verify stable schema-versioned identities, exact source order/defaults, duplicate rejection, generated source/test/fixture closure, one model-sampling sigma/time profile, checked denoise slicing, failure-atomic fixed steps, callback-before-intermediate ordering, adaptive attempt/evaluation limits, accepted/rejected counters and latent history, completion tolerance, RNG-phase independence and retry replay, typed cancellation/non-finite/OOM/device-loss errors, and checked runtime/model adapters with no second queue, persistence, security, publication, tensor, RNG, workspace, trace, callback, or cancellation owner.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-sampling-foundation-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-SAMPLER-001: All sampler trajectories

- Type: sampling.
- Fixture: Analytical tiny denoiser for each of 44 sampler IDs.
- Command/runner: `cargo test -p comfy_sampler val_sampler_001`.
- Procedure and expected signal: Compare every intermediate latent, evaluation/callback order, noise, boundaries, cancellation point, NaN/extreme sigma, and exact errors.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-sampler-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-SCHEDULER-001: All scheduler sigma arrays

- Type: sampling.
- Fixture: Scalar fixtures for each of 9 scheduler IDs.
- Command/runner: `cargo test -p comfy_sampler val_scheduler_001`.
- Procedure and expected signal: Compare exact arrays/defaults, denoise/start/end, zero/one/extreme steps, invalid values, dtype, and device-independent behavior.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-scheduler-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-LATENT-001: All latent formats

- Type: model/tensor.
- Fixture: Every one of 33 latent format rows.
- Command/runner: `cargo test -p comfy_model val_latent_001`.
- Procedure and expected signal: Compare channel/shape, scale/shift, empty construction, encode/decode, dtype/device, serialization, and invalid shape behavior.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-latent-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-PLUGIN-001: Rust/WIT plugin API and sandbox

- Type: plugin/security.
- Fixture: First-party Rust and compiled WIT fixtures covering canonical type IDs; every port field; required, optional, empty, singular, and list ports for scalar/tensor/artifact/model values; every capability; mappings; signatures; traps; hangs; cancellation; and invalid handles.
- Command/runner: `cargo test -p comfy_plugin_host val_plugin_001`.
- Procedure and expected signal: Compile Rust and WIT fixtures against the same SDK and require identical manifest/type/port/value/error projection. Exercise input presence/length, indexed reads/takes, push-plus-finish outputs, ownership/use-after-take, wrong cardinality/type, empty lists, absent optionals, and terminal revocation. For filesystem, network/provider, secret, clock, randomness, model, transactional output, sanitized log, declarative UI, and route calls, test allowed and denied grants, request/response bounds, quota, timeout/cancel, rollback, redaction, and no late side effect; then verify version negotiation, deterministic legacy resolution, diagnostics, and workflow preservation.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-plugin-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-PLUGIN-HOST-001: Extension-owned Comfy component host

- Type: plugin/architecture/security.
- Fixture: ExtensionStore fixed-pair component inventory, ComponentRuntime, signed ComponentHost lifecycle, sealed NativeNodeRegistry adapter, and catalog/API projections.
- Command/runner: `cargo test -p comfy_test_support val_plugin_host_001`.
- Procedure and expected signal: Install, update, restart, invoke, and uninstall signed no-WASI component fixtures through the production lifecycle adapter; reject unsafe identifiers, missing pairs, symlinks, oversized or changing files, invalid signatures, unauthorized grants, host-ceiling violations, cross-manifest invocation, WASI imports, traps, cancellation, quotas, and stale handles; prove failed inventory revokes stale state, failed verified replacement is atomic and visible to reload callers, ExtensionStore alone validates lifecycle identity/inventory and ComponentRuntime alone owns Comfy no-WASI engine/cache/epoch state while generic WASI, development CLI, and language engines have no Comfy call path, the invocation host admits no side effect after a known preflight failure, the runtime registry alone owns executable bindings, and API metadata exactly projects the canonical catalog plus compiled binding.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-plugin-host-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-WORKER-PLUGIN-001: Production worker plugin bridge

- Type: plugin/worker/security/recovery.
- Fixture: Content-addressed verified component deployment, private worker IPC, canonical capability-service broker, checked signed presentation projection, OutputCommitter proposals, and GPUI/API/headless startup paths.
- Command/runner: `CARGO_INCREMENTAL=0 cargo test -p comfy_test_support --test plugin_e2e val_worker_plugin_001 -- --nocapture`.
- Procedure and expected signal: Run the real worker-process fixture twice and require identical retained artifact bytes. Activate verified components in the private worker without a host path or ambient authority; derive worker registry state from the same signed component snapshot on desktop, API, headless, and worker startup; preserve exact signed component display/category/output-port metadata through the atomic RuntimeNodePresentation adapter and require the API to fail closed when that checked projection is absent; map every capability request through PluginCapabilityBroker into the canonical runtime owners and every output proposal through OutputCommitter; test deterministic repeat execution, install/update/removal, pre-dispatch and blocking cancellation, trap, actual process loss plus restart/redeployment, stale generation, denial before credential/provider actuators, proposal rollback, duplicate-publication rejection, and exact-once publication with zero Python, JavaScript, WASI, external-server, worker final-commit, or test-only production service paths.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-worker-plugin-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-NATIVE-API-001: Native API and CLI closure

- Type: protocol/CLI.
- Fixture: Every HTTP/WS and comfy-cli catalog row.
- Command/runner: `cargo test -p comfy_api val_native_api_001`.
- Procedure and expected signal: Serve/execute through Rust services, compare schemas/events/errors/lifecycle, and assert no proxy, Python, source tree, or alternate execution path.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-native-api-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-OWNERSHIP-DOMAIN-001: Authorization, cancellation, and backend ownership

- Type: architecture/security/protocol.
- Fixture: Every cancellation, runtime permission, asset/API/plugin/provider authorization, signed-manifest, asset-reference, execution-output-operation, external-navigation, backend capability, binding-status, and worker-negotiation definition and production call site.
- Command/runner: `cargo test -p comfy_test_support val_ownership_domain_001`.
- Procedure and expected signal: Require the canonical cancellation token, PermissionPolicy, PluginTrustPolicy, ProviderPolicy, ExternalNavigationPolicy, AssetIdentity/AssetRoots reference mapping, ExecutionPresentationOwner output-operation projection, and BackendCapabilityMatrix to be the only decision or mutation owners in scope; prove checked ABI/wire/asset/API/GPUI/backend adapters preserve exact semantics; reject self-grants, untyped capability decisions, duplicate host trust, repeated `sim-asset` parsing, UI-owned output persistence, direct Comfy external opens, and caller-supplied verification booleans before quota, allocation, navigation, deletion, or effects; and require a byte-stable zero-failure/zero-skip artifact twice.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-ownership-domain-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-OWNERSHIP-001: Foundational ownership and adapter semantics

- Type: architecture/security/recovery.
- Fixture: The generated authoritative-ownership catalog, every competing definition and production call site, and deterministic adapter/failure-injection fixtures.
- Command/runner: `cargo test -p comfy_test_support val_ownership_001`.
- Procedure and expected signal: Search the entire repository and require exactly one authoritative owner for each foundational behavior; prove checked DTO/ABI mappings preserve semantics; reject production self-grants, parallel queue or persistence state, repeated path/security validation, duplicate cancellation state, transitive host paths in worker payloads, and worker/plugin final commits. For tensor workspace security, require one non-cloneable BackendWorkspaceAuthority implementation for every exact backend, one ScratchReservation bind site, no stateful backend-specific authority, no public zero reservation constructor, and no backend authorizer. Only WorkerSession may consume a post-preflight graph-attempt PlannedWorkspaceAuthorization; WorkerBackendSession may use the same paired authority solely for its bounded pre-Ready internal readiness transaction. Verify only the canonical owner performs each security check, state transition, persistence write, or final publication; require a byte-stable zero-failure/zero-skip artifact twice.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-ownership-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-NATIVE-E2E-001: Native image execution slice

- Type: E2E.
- Fixture: LoadImage -> ImageScale -> ImageInvert -> PreviewImage -> SaveImage.
- Command/runner: `cargo test -p comfy_test_support val_native_e2e_001`.
- Procedure and expected signal: Run CPU tensors in worker; compare validation, progress, transactional metadata output, cache hit/invalidation, deterministic cancel, worker kill/recovery, GPUI open, and isolated native boundary.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-native-e2e-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-NATIVE-E2E-002: Native tiny diffusion slice

- Type: E2E/model.
- Fixture: The generated `sd15-tiny-v1` contract: SD15 COMFY-MODEL-0117 plus the six exact node IDs, SD1 tokenizer/token IDs, reduced f32 CLIP/UNet/VAE topology, SD15 latent COMFY-MODEL-0045, Euler COMFY-MODEL-0179, normal COMFY-MODEL-0209, four steps, fixed seed, complete state-key manifest, and every named intermediate.
- Command/runner: `cargo test -p comfy_test_support val_native_e2e_002`.
- Procedure and expected signal: Validate the fixture contract and provenance, then compare family detection and prefix/key mapping, tokenizer IDs, CLIP conditioning, exact sigmas, RNG/noise, all denoiser evaluations and Euler latent steps, VAE pixels, PNG/metadata, cache, cancellation at every evaluation, OOM plan, worker crash/restart, GPUI interaction, and isolated native boundary. The debug-build worker-response deadline is only a deadlock watchdog and does not replace or weaken the separately measured five-second release-profile performance gate. A family, tokenizer, equation, key, sampler, scheduler, latent substitution, or release-budget miss fails.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-native-e2e-002.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-MEDIA-001: Native media and metadata

- Type: media/domain.
- Fixture: All cataloged codecs, carriers, outputs, previews, malformed and hostile fixtures.
- Command/runner: `cargo test -p comfy_media val_media_001`.
- Procedure and expected signal: Compare pixels/samples/frames/3D data, timing, color, orientation, chunks, filenames, limits, cancellation, unavailable codec, FFI packaging, and no subprocess.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-media-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-METADATA-001: Embedded workflow metadata carriers

- Type: media/domain.
- Fixture: Every cataloged metadata carrier plus malformed, hostile, oversized, disabled-metadata, and unknown-chunk fixtures.
- Command/runner: `cargo test -p comfy_media val_metadata_001`.
- Procedure and expected signal: Extract and round-trip prompt/workflow metadata and permitted unknown chunks with bounded parsing, exact carrier priority and disable-metadata behavior, content detection, truncation coverage, and no execution on import; codec pixel/sample/frame/geometry closure remains in VAL-MEDIA-001.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-metadata-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-DOCS-001: Documentation evidence reconciliation

- Type: catalog.
- Fixture: docs, embedded-docs, navigation, redirects, localization, node docs, OpenAPI, and tooling catalogs.
- Command/runner: `python3 .agents/specs/comfy-parity/generate_documentation_catalogs.py`.
- Procedure and expected signal: Verify file closure and deltas, require executable corroboration for stronger evidence, retain documented-only claims, and reproduce recorded link/test/translation results.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-docs-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-CLI-001: comfy-cli contract reconciliation

- Type: CLI.
- Fixture: Every command, flag, schema, event, error, config, format, lifecycle, test, and source row.
- Command/runner: `cargo test -p sim --features test-support comfy_cli_contract`.
- Procedure and expected signal: Compare native/migration/defer mapping, help/schema output, invalid input, offline, cancellation, interrupted operation, and source-file closure without running Python in production.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-cli-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-NODE-CLOSURE-001: Native node implementation closure

- Type: node/catalog.
- Fixture: All local and API-node rows plus generated native descriptors.
- Command/runner: `cargo test -p comfy_nodes val_node_closure_001`.
- Procedure and expected signal: Reconcile implementation/provider/placeholder status and per-node schema/behavior results with zero unexplained rows; fail any representative-only equivalence claim.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-node-closure-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

### VAL-COMFY-BUILD-001: Opt-in Comfy product boundary

- Type: build/package.
- Fixture: The default and Comfy-enabled Sim feature, dependency, runtime-surface, asset, and macOS/Linux/Windows package graphs.
- Command/runner: `cargo test -p comfy_test_support val_comfy_build_001`.
- Procedure and expected signal: Build and test default Sim without Comfy, build CPU and selected accelerator Comfy modes, prove the default normal dependency tree contains no comfy_* package, verify Comfy CLI/UI/runtime/settings surfaces are cfg-absent, and compare default versus explicit-Comfy package plans with deterministic zero-failure evidence.
- Pass artifact: exit status 0 plus `target/comfy-parity/val-comfy-build-001.json` containing fixture digests, environment/backend identity, per-case results, and no unexplained skipped row.

## Validation execution and gates

Implementation tasks run targeted crate/domain/GPUI tests and `./script/clippy`. Closure runs every generator twice, every applicable unit/protocol/GPUI/visual/E2E/persistence/failure/accessibility/platform/security/performance scenario available for the CPU/Apple Metal baseline, fail-closed compile and unavailable-path checks for optional backends, the package boundary scan, registry/source-file reconciliation, forward and reverse traceability, independent completeness and implementation-readiness audits, and:

```sh
python3 .agents/skills/coding/scripts/validate_spec.py .agents/specs/comfy-parity --require-complete
```

A missing runtime, account, provider, codec, dependency, signing key, or optional device does not become a pass. It remains a named external release gate or uncertainty with the affected feature IDs, planned lab, safe default, and consequence. Such external gates do not block implementation completion, but no optional backend status changes to `equivalent` or `certified` until the exact row's target evidence exists. CoreX remains typed Unbound regardless of local files until the separate future specification completes every proprietary admission gate.
