# Requirements: Godot Full Port Coverage

## Problem

The existing Godot migration pack groups broad areas but does not prove source-complete capability coverage, connected native Sim implementation, or leaf-level traceability. This audit freezes the source baseline and makes every independently observable capability reviewable without treating checked tasks, placeholders, external Godot delegation, dependencies, or blanket exclusions as parity.

## Scope

### In scope

- Godot 4.7-stable editor, runtime, format, build, platform, optional-module, test, documentation, and infrastructure behavior present in the frozen source snapshot.
- Existing Sim implementation and every specification under `.agents/specs/godot-migration/`.
- Source-backed classification, native Sim ownership, anti-duplication, no-Godot dependency validation, platform/failure/lifecycle coverage, and implementation planning.

### Out of scope

- Product implementation, dependency installation, external mutation, commits, pushes, and pull requests.
- Choosing unresolved native product scope, compatibility floors, source-copy licensing policy, or materially different native architecture directions.

## Requirements

### Requirement 1: Reproducible audit baseline and catalog

**User story:** As a reviewer, I want the audit bound to reproducible source revisions and exhaustive catalog fields so that coverage claims can be independently checked.

#### Acceptance criteria

1. **1.1** THE audit SHALL record the Sim commit, Godot commit, content-manifest fingerprint, source-file count, source version, working-tree state, submodule state, build targets, feature options, modules, platform roots, and CI roots.
2. **1.2** THE catalog SHALL assign every independently observable capability one stable `GODOT-<DOMAIN>-<NUMBER>` ID and exactly one of the seven requested classifications.
3. **1.3** THE catalog SHALL record every field requested by the audit, including exact source, Sim, requirement, design, task, validation, confidence, decision, native owner, Godot build/runtime dependency, Sim storage/execution/UI/lifecycle path, Godot-compatible boundary, reuse evidence, and no-Godot-installation evidence.
4. **1.4** THE summary SHALL reconcile every catalog row by domain and classification and state the coverage denominator and formula.
5. **1.5** IF evidence is absent or a product, compatibility, licensing, or architecture choice is unresolved, THEN THE audit SHALL record uncertainty and SHALL NOT assume parity or choose a direction.

### Requirement 2: Project manager and lifecycle

**User story:** As a Godot project owner, I want migration coverage for discovery, creation, import, launch, recovery so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **2.1** WHEN a cataloged project manager and lifecycle capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **2.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **2.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `project + workspace + recent_projects` and SHALL NOT add a parallel implementation.
4. **2.4** WHEN parity is claimed, THEN verification SHALL exercise every `PROJ` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 3: Scene, node, resource, and serialization

**User story:** As a Godot project owner, I want migration coverage for runtime object graph and file contracts so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **3.1** WHEN a cataloged scene, node, resource, and serialization capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **3.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **3.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `project + worktree + language` and SHALL NOT add a parallel implementation.
4. **3.4** WHEN parity is claimed, THEN verification SHALL exercise every `SCENE` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 4: Editor workspace and authoring surfaces

**User story:** As a Godot project owner, I want migration coverage for workspaces, docks, inspector, commands, settings so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **4.1** WHEN a cataloged editor workspace and authoring surfaces capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **4.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **4.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `workspace + project_panel + inspector_ui + editor + command_palette` and SHALL NOT add a parallel implementation.
4. **4.4** WHEN parity is claimed, THEN verification SHALL exercise every `EDITOR` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 5: 2D rendering

**User story:** As a Godot project owner, I want migration coverage for canvas scene and renderer so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **5.1** WHEN a cataloged 2d rendering capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **5.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **5.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `gpui + gpui_wgpu + image_viewer` and SHALL NOT add a parallel implementation.
4. **5.4** WHEN parity is claimed, THEN verification SHALL exercise every `R2D` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 6: 3D rendering

**User story:** As a Godot project owner, I want migration coverage for scene renderer, materials, lighting, post-processing so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **6.1** WHEN a cataloged 3d rendering capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **6.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **6.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `gpui_wgpu + component_preview + image_viewer` and SHALL NOT add a parallel implementation.
4. **6.4** WHEN parity is claimed, THEN verification SHALL exercise every `R3D` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 7: UI/control framework and themes

**User story:** As a Godot project owner, I want migration coverage for runtime Control tree and editor UI reuse so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **7.1** WHEN a cataloged ui/control framework and themes capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **7.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **7.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `gpui + ui + theme + ui_input` and SHALL NOT add a parallel implementation.
4. **7.4** WHEN parity is claimed, THEN verification SHALL exercise every `UI` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 8: Input, windowing, display, accessibility, and internationalization

**User story:** As a Godot project owner, I want migration coverage for devices, actions, windows, locale, assistive technology so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **8.1** WHEN a cataloged input, windowing, display, accessibility, and internationalization capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **8.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **8.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `gpui + gpui_platform + keymap_editor + settings` and SHALL NOT add a parallel implementation.
4. **8.4** WHEN parity is claimed, THEN verification SHALL exercise every `INPUT` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 9: Physics, navigation, animation, audio, and particles

**User story:** As a Godot project owner, I want migration coverage for simulation services so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **9.1** WHEN a cataloged physics, navigation, animation, audio, and particles capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **9.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **9.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `task + project metadata; architecture decision required for runtime owner` and SHALL NOT add a parallel implementation.
4. **9.4** WHEN parity is claimed, THEN verification SHALL exercise every `SIM` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 10: Scripting languages and script lifecycle

**User story:** As a Godot project owner, I want migration coverage for GDScript, C#, expression, editor tooling so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **10.1** WHEN a cataloged scripting languages and script lifecycle capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **10.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **10.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `language + lsp + dap + extension_host + task` and SHALL NOT add a parallel implementation.
4. **10.4** WHEN parity is claimed, THEN verification SHALL exercise every `SCRIPT` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 11: Native extensions and editor plugins

**User story:** As a Godot project owner, I want migration coverage for GDExtension ABI and plugin lifecycle so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **11.1** WHEN a cataloged native extensions and editor plugins capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **11.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **11.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `extension + extension_host + extension_api + extensions_ui` and SHALL NOT add a parallel implementation.
4. **11.4** WHEN parity is claimed, THEN verification SHALL exercise every `EXT` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 12: Asset importing, caching, and dependencies

**User story:** As a Godot project owner, I want migration coverage for editor filesystem and import pipeline so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **12.1** WHEN a cataloged asset importing, caching, and dependencies capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **12.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **12.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `worktree + fs + project + image_viewer + svg_preview` and SHALL NOT add a parallel implementation.
4. **12.4** WHEN parity is claimed, THEN verification SHALL exercise every `IMPORT` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 13: Export, packaging, templates, and deployment

**User story:** As a Godot project owner, I want migration coverage for presets, PCK, templates, platform exporters so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **13.1** WHEN a cataloged export, packaging, templates, and deployment capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **13.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **13.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `task + terminal + project + settings` and SHALL NOT add a parallel implementation.
4. **13.4** WHEN parity is claimed, THEN verification SHALL exercise every `EXPORT` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 14: Filesystem, networking, HTTP, multiplayer, and web

**User story:** As a Godot project owner, I want migration coverage for runtime IO and communication so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **14.1** WHEN a cataloged filesystem, networking, http, multiplayer, and web capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **14.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **14.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `fs + net + http_client + rpc + collab + task` and SHALL NOT add a parallel implementation.
4. **14.4** WHEN parity is claimed, THEN verification SHALL exercise every `NET` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 15: Debugger, profiler, logging, diagnostics, and crashes

**User story:** As a Godot project owner, I want migration coverage for editor/runtime observability so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **15.1** WHEN a cataloged debugger, profiler, logging, diagnostics, and crashes capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **15.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **15.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `dap + debugger_ui + diagnostics + zlog + crashes + miniprofiler_ui` and SHALL NOT add a parallel implementation.
4. **15.4** WHEN parity is claimed, THEN verification SHALL exercise every `DEBUG` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 16: CLI, headless, automation, and developer workflows

**User story:** As a Godot project owner, I want migration coverage for main process modes and tooling so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **16.1** WHEN a cataloged cli, headless, automation, and developer workflows capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **16.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **16.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `cli + task + terminal + remote_server` and SHALL NOT add a parallel implementation.
4. **16.4** WHEN parity is claimed, THEN verification SHALL exercise every `CLI` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 17: Authentication, permissions, sandboxing, and security

**User story:** As a Godot project owner, I want migration coverage for trust and resource boundaries so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **17.1** WHEN a cataloged authentication, permissions, sandboxing, and security capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **17.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **17.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `sandbox + credentials_provider + http_client_tls + extension_host + settings` and SHALL NOT add a parallel implementation.
4. **17.4** WHEN parity is claimed, THEN verification SHALL exercise every `SEC` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 18: Persistence, compatibility, migrations, and formats

**User story:** As a Godot project owner, I want migration coverage for durable project/editor/runtime state so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **18.1** WHEN a cataloged persistence, compatibility, migrations, and formats capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **18.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **18.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `settings + session + workspace persistence + db + migrator + fs` and SHALL NOT add a parallel implementation.
4. **18.4** WHEN parity is claimed, THEN verification SHALL exercise every `PERSIST` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 19: Platform-specific behavior

**User story:** As a Godot project owner, I want migration coverage for desktop, mobile, web, XR so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **19.1** WHEN a cataloged platform-specific behavior capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **19.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **19.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `gpui_windows + gpui_macos + gpui_linux + gpui_web + task` and SHALL NOT add a parallel implementation.
4. **19.4** WHEN parity is claimed, THEN verification SHALL exercise every `PLAT` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 20: Tests, examples, docs, localization, build tooling, and CI

**User story:** As a Godot project owner, I want migration coverage for quality and developer infrastructure so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **20.1** WHEN a cataloged tests, examples, docs, localization, build tooling, and ci capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **20.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **20.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `existing crate tests + script tooling + docs preprocessing` and SHALL NOT add a parallel implementation.
4. **20.4** WHEN parity is claimed, THEN verification SHALL exercise every `QA` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 21: Optional modules and build features

**User story:** As a Godot project owner, I want migration coverage for SCons feature composition so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **21.1** WHEN a cataloged optional modules and build features capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **21.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **21.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `Cargo feature owners + task diagnostics + system_specs` and SHALL NOT add a parallel implementation.
4. **21.4** WHEN parity is claimed, THEN verification SHALL exercise every `MOD` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 22: Third-party and upstream infrastructure

**User story:** As a Godot project owner, I want migration coverage for vendor, generators, release engineering so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.

#### Acceptance criteria

1. **22.1** WHEN a cataloged third-party and upstream infrastructure capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.
2. **22.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.
3. **22.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `existing dependency, license, build, CI, and docs tooling` and SHALL NOT add a parallel implementation.
4. **22.4** WHEN parity is claimed, THEN verification SHALL exercise every `UPSTREAM` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.

### Requirement 23: Native Sim implementation gate

**User story:** As a product and distribution owner, I want every supported Godot-origin capability to be owned and executed by Sim so that shipped products remain independent of a Godot installation or runtime dependency.

#### Acceptance criteria

1. **23.1** WHEN a Godot-origin capability is implemented or classified as fully specified, THEN its storage, execution, UI, persistence, cancellation, recovery, and lifecycle paths SHALL be owned by named existing or proposed Sim components.
2. **23.2** THE migration SHALL NOT embed, bundle, invoke, launch, link against, wrap, proxy, communicate with, or depend at build time or runtime on the Godot editor, engine, executable, shared library, server, command-line tool, or hidden Godot instance.
3. **23.3** WHEN Godot-compatible projects, scenes, resources, scripts, APIs, imports, or export settings cross a compatibility boundary, THEN Sim SHALL parse or emit the boundary representation while storing and executing supported behavior as Sim-native data, resources, scenes, artifacts, and runtime state.
4. **23.4** WHEN an existing Sim component owns an adjacent responsibility, THEN the migration SHALL extend that owner and SHALL NOT create a parallel Godot-specific crate, registry, manager, runtime, task source, UI subsystem, persistence layer, network stack, plugin host, renderer, or platform service.
5. **23.5** IF source code, generated code, vendor patches, libraries, bindings, fixtures, assets, or documentation would be copied from Godot, THEN work SHALL remain blocked until a separate licensing and architecture review approves the exact material, license obligations, provenance, maintenance, linkage, and distribution effects.
6. **23.6** WHEN an importer or migration tool reads Godot formats, THEN every successful output SHALL be a Sim-native record, resource, scene, artifact, cache entry, or runtime state, and failure/cancellation SHALL not require Godot to recover.
7. **23.7** WHEN an exported project is validated, THEN it SHALL execute on a machine without Godot installed and its package, process tree, loader resolution, network connections, and runtime dependency manifest SHALL contain no Godot editor, engine, executable, shared library, server, or command-line tool.
8. **23.8** IF a capability cannot currently be implemented through a native Sim owner, THEN it SHALL be classified as unresolved, intentionally excluded with rationale, or requiring a material product/architecture decision and SHALL NOT be treated as covered by a wrapper, interface, placeholder, format declaration, task template, or external delegation.
9. **23.9** WHEN a capability is classified as already implemented or fully specified, THEN its acceptance criteria, leaf task validation, and implementation evidence SHALL prove Sim-owned execution with Godot absent; otherwise the classification SHALL be partial, missing, excluded, upstream-only, or decision-blocked as applicable.
10. **23.10** WHEN this audit is validated, THEN it SHALL report every plan that embeds, wraps, invokes, vendors, links, or delegates to Godot; every Godot-specific abstraction duplicating a Sim owner; every placeholder-only support claim; every license/linkage dependency; and every material native architecture decision without silently selecting a direction.

## Constraints

- Classification records the pre-audit baseline. Newly added closure traceability does not rewrite historical classification.
- Editor behavior and exported-runtime behavior remain distinct.
- Mandatory behavior, optional modules, examples, tests, vendor code, and release infrastructure remain distinct.
- Every implementation task remains unchecked until separately implemented and verified.

## Open questions

See `decisions.md`; those decisions intentionally block native runtime scope, compatibility floors, platform tiers, plugin trust, and source reuse/licensing choices. External Godot execution is not an available architecture direction.
