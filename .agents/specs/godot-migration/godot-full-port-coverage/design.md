# Design: Godot Full Port Coverage

## Overview

The audit uses a frozen source manifest, one master capability catalog, and domain owners that point into existing Sim crates. The catalog is the granularity authority; requirements define cross-cutting success/failure/reuse/evidence and native-ownership contracts, design elements bind each domain to its smallest existing owners, and one leaf task per capability prevents broad labels, wrappers, or Godot delegation from hiding materially different behavior.

## Existing context

The inspected Sim revision has mature project, worktree, editor, GPUI, language/LSP/DAP, task, filesystem, HTTP/RPC, collaboration, diagnostics, settings, persistence, sandbox, extension, media, and platform crates. It has no `sim_game` or `world_model` workspace members and no connected `project.godot`, `.tscn`, `.tres`, GDScript, GDExtension, Godot import, or Godot export implementation. Existing checked migration tasks are therefore planning history, not implementation evidence.

## Design decisions

### D-BASELINE: Frozen evidence and historical classification

- Responsibility: Reproduce the exact Godot and Sim sources, preserve baseline classification, and generate reconciled counts.
- Integration: `baseline.md`, `catalogs/master-coverage.csv`, `coverage-summary.md`, and `verify_snapshot.py`.
- Rationale: A source snapshot without nested Git metadata must be verified against the official tag rather than assigned an inferred SHA.

### D-NATIVE: Native Sim ownership and no-Godot dependency gate

- Responsibility: Require every supported Godot-origin capability to name the existing or proposed Sim owner, Sim-native storage/execution/UI/lifecycle path, compatibility boundary, reuse evidence, build/runtime dependency status, and a validation that runs with Godot absent.
- Integration: Existing Sim owners named by each domain, `catalogs/master-coverage.csv`, `findings.md`, `decisions.md`, `validate_audit.py`, owner-spec acceptance criteria, and leaf-task validation metadata.
- Runtime boundary: Godot is a behavioral and format reference only. Sim must not embed, bundle, invoke, launch, link, wrap, proxy, or communicate with Godot. Imported formats terminate at Sim-native records/resources; exports package Sim-owned execution.
- Source boundary: Godot source, generated code, vendor patches, fixtures, assets, and docs remain evidence unless exact copying is separately approved after licensing and architecture review.
- Classification gate: Classifications 1 and 3 require acceptance criteria and connected evidence proving Sim-owned execution in a hermetic no-Godot environment. A wrapper, task template, file declaration, type, interface, stub, placeholder, disabled path, or external delegation cannot satisfy the gate.
- Decision handling: Capabilities without a viable native owner remain unresolved, intentionally excluded, upstream-only, or decision-blocked; the audit never selects between materially different native product/architecture directions.

### D-PROJ: Project manager and lifecycle ownership

- Responsibility: Own the `PROJ` catalog rows for discovery, creation, import, launch, recovery, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `project + workspace + recent_projects`. Proposed focused writes are `crates/project/src/project.rs, crates/workspace/src/workspace.rs, crates/recent_projects/src/recent_projects.rs`.
- Rationale: Extend project + workspace + recent_projects at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-SCENE: Scene, node, resource, and serialization ownership

- Responsibility: Own the `SCENE` catalog rows for runtime object graph and file contracts, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `project + worktree + language`. Proposed focused writes are `crates/project/src/project.rs, crates/worktree/src/worktree.rs, crates/language/src/language_registry.rs`.
- Rationale: Extend project + worktree + language at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-EDITOR: Editor workspace and authoring surfaces ownership

- Responsibility: Own the `EDITOR` catalog rows for workspaces, docks, inspector, commands, settings, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `workspace + project_panel + inspector_ui + editor + command_palette`. Proposed focused writes are `crates/workspace/src/workspace.rs, crates/project_panel/src/project_panel.rs, crates/inspector_ui/src/inspector_ui.rs`.
- Rationale: Extend workspace + project_panel + inspector_ui + editor + command_palette at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-R2D: 2D rendering ownership

- Responsibility: Own the `R2D` catalog rows for canvas scene and renderer, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `gpui + gpui_wgpu + image_viewer`. Proposed focused writes are `crates/gpui/src/element.rs, crates/gpui_wgpu/src/wgpu_renderer.rs, crates/image_viewer/src/image_viewer.rs`.
- Rationale: Extend gpui + gpui_wgpu + image_viewer at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-R3D: 3D rendering ownership

- Responsibility: Own the `R3D` catalog rows for scene renderer, materials, lighting, post-processing, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `gpui_wgpu + component_preview + image_viewer`. Proposed focused writes are `crates/gpui_wgpu/src/wgpu_renderer.rs, crates/component_preview/src/component_preview.rs, crates/image_viewer/src/image_viewer.rs`.
- Rationale: Extend gpui_wgpu + component_preview + image_viewer at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-UI: UI/control framework and themes ownership

- Responsibility: Own the `UI` catalog rows for runtime Control tree and editor UI reuse, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `gpui + ui + theme + ui_input`. Proposed focused writes are `crates/ui/src/ui.rs, crates/theme/src/theme.rs, crates/ui_input/src/ui_input.rs`.
- Rationale: Extend gpui + ui + theme + ui_input at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-INPUT: Input, windowing, display, accessibility, and internationalization ownership

- Responsibility: Own the `INPUT` catalog rows for devices, actions, windows, locale, assistive technology, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `gpui + gpui_platform + keymap_editor + settings`. Proposed focused writes are `crates/gpui/src/platform.rs, crates/gpui_platform/src/gpui_platform.rs, crates/settings/src/settings.rs`.
- Rationale: Extend gpui + gpui_platform + keymap_editor + settings at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-SIM: Physics, navigation, animation, audio, and particles ownership

- Responsibility: Own the `SIM` catalog rows for simulation services, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `task + project metadata; architecture decision required for runtime owner`. Proposed focused writes are `crates/project/src/project.rs, crates/task/src/task.rs, crates/audio/src/audio.rs, crates/media/src/media.rs`.
- Rationale: Extend task + project metadata; architecture decision required for runtime owner at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-SCRIPT: Scripting languages and script lifecycle ownership

- Responsibility: Own the `SCRIPT` catalog rows for GDScript, C#, expression, editor tooling, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `language + lsp + dap + extension_host + task`. Proposed focused writes are `crates/languages/src/lib.rs, crates/language/src/language_registry.rs, crates/lsp/src/lsp.rs, crates/dap/src/dap.rs`.
- Rationale: Extend language + lsp + dap + extension_host + task at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-EXT: Native extensions and editor plugins ownership

- Responsibility: Own the `EXT` catalog rows for GDExtension ABI and plugin lifecycle, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `extension + extension_host + extension_api + extensions_ui`. Proposed focused writes are `crates/extension_host/src/extension_host.rs, crates/extension_api/src/extension_api.rs, crates/extensions_ui/src/extensions_ui.rs`.
- Rationale: Extend extension + extension_host + extension_api + extensions_ui at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-IMPORT: Asset importing, caching, and dependencies ownership

- Responsibility: Own the `IMPORT` catalog rows for editor filesystem and import pipeline, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `worktree + fs + project + image_viewer + svg_preview`. Proposed focused writes are `crates/worktree/src/worktree.rs, crates/project/src/project.rs, crates/image_viewer/src/image_viewer.rs`.
- Rationale: Extend worktree + fs + project + image_viewer + svg_preview at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-EXPORT: Export, packaging, templates, and deployment ownership

- Responsibility: Own the `EXPORT` catalog rows for presets, PCK, templates, platform exporters, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `task + terminal + project + settings`. Proposed focused writes are `crates/task/src/task.rs, crates/project/src/project.rs, crates/settings/src/settings.rs`.
- Rationale: Extend task + terminal + project + settings at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-NET: Filesystem, networking, HTTP, multiplayer, and web ownership

- Responsibility: Own the `NET` catalog rows for runtime IO and communication, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `fs + net + http_client + rpc + collab + task`. Proposed focused writes are `crates/fs/src/fs.rs, crates/net/src/net.rs, crates/http_client/src/http_client.rs, crates/rpc/src/rpc.rs, crates/collab/src/lib.rs`.
- Rationale: Extend fs + net + http_client + rpc + collab + task at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-DEBUG: Debugger, profiler, logging, diagnostics, and crashes ownership

- Responsibility: Own the `DEBUG` catalog rows for editor/runtime observability, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `dap + debugger_ui + diagnostics + zlog + crashes + miniprofiler_ui`. Proposed focused writes are `crates/dap/src/dap.rs, crates/debugger_ui/src/debugger_ui.rs, crates/diagnostics/src/diagnostics.rs, crates/crashes/src/crashes.rs`.
- Rationale: Extend dap + debugger_ui + diagnostics + zlog + crashes + miniprofiler_ui at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-CLI: CLI, headless, automation, and developer workflows ownership

- Responsibility: Own the `CLI` catalog rows for main process modes and tooling, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `cli + task + terminal + remote_server`. Proposed focused writes are `crates/cli/src/cli.rs, crates/task/src/task.rs, crates/remote_server/src/main.rs`.
- Rationale: Extend cli + task + terminal + remote_server at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-SEC: Authentication, permissions, sandboxing, and security ownership

- Responsibility: Own the `SEC` catalog rows for trust and resource boundaries, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `sandbox + credentials_provider + http_client_tls + extension_host + settings`. Proposed focused writes are `crates/sandbox/src/sandbox.rs, crates/credentials_provider/src/credentials_provider.rs, crates/extension_host/src/extension_host.rs`.
- Rationale: Extend sandbox + credentials_provider + http_client_tls + extension_host + settings at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-PERSIST: Persistence, compatibility, migrations, and formats ownership

- Responsibility: Own the `PERSIST` catalog rows for durable project/editor/runtime state, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `settings + session + workspace persistence + db + migrator + fs`. Proposed focused writes are `crates/settings/src/settings.rs, crates/session/src/session.rs, crates/workspace/src/persistence/model.rs, crates/migrator/src/migrator.rs`.
- Rationale: Extend settings + session + workspace persistence + db + migrator + fs at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-PLAT: Platform-specific behavior ownership

- Responsibility: Own the `PLAT` catalog rows for desktop, mobile, web, XR, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `gpui_windows + gpui_macos + gpui_linux + gpui_web + task`. Proposed focused writes are `crates/gpui_platform/src/gpui_platform.rs, crates/task/src/task.rs`.
- Rationale: Extend gpui_windows + gpui_macos + gpui_linux + gpui_web + task at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-QA: Tests, examples, docs, localization, build tooling, and CI ownership

- Responsibility: Own the `QA` catalog rows for quality and developer infrastructure, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `existing crate tests + script tooling + docs preprocessing`. Proposed focused writes are `crates/project/tests/integration/project_tests.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv, .github/workflows/run_tests.yml`.
- Rationale: Extend existing crate tests + script tooling + docs preprocessing at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-MOD: Optional modules and build features ownership

- Responsibility: Own the `MOD` catalog rows for SCons feature composition, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `Cargo feature owners + task diagnostics + system_specs`. Proposed focused writes are `crates/system_specs/src/system_specs.rs, crates/task/src/task.rs, Cargo.toml`.
- Rationale: Extend Cargo feature owners + task diagnostics + system_specs at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

### D-UPSTREAM: Third-party and upstream infrastructure ownership

- Responsibility: Own the `UPSTREAM` catalog rows for vendor, generators, release engineering, including success, failure, persistence, lifecycle, mode, and platform outcomes.
- Integration: Reuse or extend `existing dependency, license, build, CI, and docs tooling`. Proposed focused writes are `script/check-licenses, script/generate-licenses, tooling/compliance/src/lib.rs`.
- Rationale: Extend existing dependency, license, build, CI, and docs tooling at its existing integration points; do not fork parallel project, UI, network, security, or persistence services.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D-BASELINE | Recompute commit, manifest, build/module/platform inventory |
| 1.2 | D-BASELINE | Validate IDs and seven-value classification enum |
| 1.3 | D-BASELINE | Validate required nonempty catalog columns and exact trace IDs |
| 1.4 | D-BASELINE | Reconcile generated summary counts to CSV rows |
| 1.5 | D-BASELINE | Review uncertainty and decision registers for silent choices |
| 2.1 | D-PROJ | Catalog scenarios for every PROJ row |
| 2.2 | D-PROJ | Catalog scenarios for every PROJ row |
| 2.3 | D-PROJ | Catalog scenarios for every PROJ row |
| 2.4 | D-PROJ | cargo test -p project -p workspace -p recent_projects godot |
| 3.1 | D-SCENE | Catalog scenarios for every SCENE row |
| 3.2 | D-SCENE | Catalog scenarios for every SCENE row |
| 3.3 | D-SCENE | Catalog scenarios for every SCENE row |
| 3.4 | D-SCENE | cargo test -p project -p worktree -p language godot_scene |
| 4.1 | D-EDITOR | Catalog scenarios for every EDITOR row |
| 4.2 | D-EDITOR | Catalog scenarios for every EDITOR row |
| 4.3 | D-EDITOR | Catalog scenarios for every EDITOR row |
| 4.4 | D-EDITOR | cargo test -p workspace -p project_panel -p inspector_ui godot |
| 5.1 | D-R2D | Catalog scenarios for every R2D row |
| 5.2 | D-R2D | Catalog scenarios for every R2D row |
| 5.3 | D-R2D | Catalog scenarios for every R2D row |
| 5.4 | D-R2D | cargo test -p gpui -p gpui_wgpu -p image_viewer godot_canvas |
| 6.1 | D-R3D | Catalog scenarios for every R3D row |
| 6.2 | D-R3D | Catalog scenarios for every R3D row |
| 6.3 | D-R3D | Catalog scenarios for every R3D row |
| 6.4 | D-R3D | cargo test -p gpui_wgpu -p component_preview godot_3d |
| 7.1 | D-UI | Catalog scenarios for every UI row |
| 7.2 | D-UI | Catalog scenarios for every UI row |
| 7.3 | D-UI | Catalog scenarios for every UI row |
| 7.4 | D-UI | cargo test -p ui -p theme -p ui_input godot_control |
| 8.1 | D-INPUT | Catalog scenarios for every INPUT row |
| 8.2 | D-INPUT | Catalog scenarios for every INPUT row |
| 8.3 | D-INPUT | Catalog scenarios for every INPUT row |
| 8.4 | D-INPUT | cargo test -p gpui -p gpui_platform -p keymap_editor godot_input |
| 9.1 | D-SIM | Catalog scenarios for every SIM row |
| 9.2 | D-SIM | Catalog scenarios for every SIM row |
| 9.3 | D-SIM | Catalog scenarios for every SIM row |
| 9.4 | D-SIM | cargo test -p project -p task -p audio godot_simulation |
| 10.1 | D-SCRIPT | Catalog scenarios for every SCRIPT row |
| 10.2 | D-SCRIPT | Catalog scenarios for every SCRIPT row |
| 10.3 | D-SCRIPT | Catalog scenarios for every SCRIPT row |
| 10.4 | D-SCRIPT | cargo test -p language -p languages -p lsp -p dap godot |
| 11.1 | D-EXT | Catalog scenarios for every EXT row |
| 11.2 | D-EXT | Catalog scenarios for every EXT row |
| 11.3 | D-EXT | Catalog scenarios for every EXT row |
| 11.4 | D-EXT | cargo test -p extension -p extension_host -p extensions_ui godot |
| 12.1 | D-IMPORT | Catalog scenarios for every IMPORT row |
| 12.2 | D-IMPORT | Catalog scenarios for every IMPORT row |
| 12.3 | D-IMPORT | Catalog scenarios for every IMPORT row |
| 12.4 | D-IMPORT | cargo test -p worktree -p project -p image_viewer godot_import |
| 13.1 | D-EXPORT | Catalog scenarios for every EXPORT row |
| 13.2 | D-EXPORT | Catalog scenarios for every EXPORT row |
| 13.3 | D-EXPORT | Catalog scenarios for every EXPORT row |
| 13.4 | D-EXPORT | cargo test -p task -p project -p settings godot_export |
| 14.1 | D-NET | Catalog scenarios for every NET row |
| 14.2 | D-NET | Catalog scenarios for every NET row |
| 14.3 | D-NET | Catalog scenarios for every NET row |
| 14.4 | D-NET | cargo test -p fs -p net -p http_client -p rpc godot |
| 15.1 | D-DEBUG | Catalog scenarios for every DEBUG row |
| 15.2 | D-DEBUG | Catalog scenarios for every DEBUG row |
| 15.3 | D-DEBUG | Catalog scenarios for every DEBUG row |
| 15.4 | D-DEBUG | cargo test -p dap -p debugger_ui -p diagnostics -p crashes godot |
| 16.1 | D-CLI | Catalog scenarios for every CLI row |
| 16.2 | D-CLI | Catalog scenarios for every CLI row |
| 16.3 | D-CLI | Catalog scenarios for every CLI row |
| 16.4 | D-CLI | cargo test -p cli -p task -p remote_server godot |
| 17.1 | D-SEC | Catalog scenarios for every SEC row |
| 17.2 | D-SEC | Catalog scenarios for every SEC row |
| 17.3 | D-SEC | Catalog scenarios for every SEC row |
| 17.4 | D-SEC | cargo test -p sandbox -p credentials_provider -p extension_host godot_security |
| 18.1 | D-PERSIST | Catalog scenarios for every PERSIST row |
| 18.2 | D-PERSIST | Catalog scenarios for every PERSIST row |
| 18.3 | D-PERSIST | Catalog scenarios for every PERSIST row |
| 18.4 | D-PERSIST | cargo test -p settings -p session -p workspace -p migrator godot_persistence |
| 19.1 | D-PLAT | Catalog scenarios for every PLAT row |
| 19.2 | D-PLAT | Catalog scenarios for every PLAT row |
| 19.3 | D-PLAT | Catalog scenarios for every PLAT row |
| 19.4 | D-PLAT | cargo test -p gpui_platform -p task godot_platform |
| 20.1 | D-QA | Catalog scenarios for every QA row |
| 20.2 | D-QA | Catalog scenarios for every QA row |
| 20.3 | D-QA | Catalog scenarios for every QA row |
| 20.4 | D-QA | cargo test -p project godot_compat && ./script/clippy |
| 21.1 | D-MOD | Catalog scenarios for every MOD row |
| 21.2 | D-MOD | Catalog scenarios for every MOD row |
| 21.3 | D-MOD | Catalog scenarios for every MOD row |
| 21.4 | D-MOD | cargo test -p system_specs -p task godot_features |
| 22.1 | D-UPSTREAM | Catalog scenarios for every UPSTREAM row |
| 22.2 | D-UPSTREAM | Catalog scenarios for every UPSTREAM row |
| 22.3 | D-UPSTREAM | Catalog scenarios for every UPSTREAM row |
| 22.4 | D-UPSTREAM | ./script/check-licenses && cargo test -p compliance godot |
| 23.1 | D-NATIVE | Catalog native-owner/dependency/path/boundary fields, violation report, and hermetic no-Godot validation |
| 23.2 | D-NATIVE | Catalog native-owner/dependency/path/boundary fields, violation report, and hermetic no-Godot validation |
| 23.3 | D-NATIVE | Catalog native-owner/dependency/path/boundary fields, violation report, and hermetic no-Godot validation |
| 23.4 | D-NATIVE | Catalog native-owner/dependency/path/boundary fields, violation report, and hermetic no-Godot validation |
| 23.5 | D-NATIVE | Catalog native-owner/dependency/path/boundary fields, violation report, and hermetic no-Godot validation |
| 23.6 | D-NATIVE | Catalog native-owner/dependency/path/boundary fields, violation report, and hermetic no-Godot validation |
| 23.7 | D-NATIVE | Catalog native-owner/dependency/path/boundary fields, violation report, and hermetic no-Godot validation |
| 23.8 | D-NATIVE | Catalog native-owner/dependency/path/boundary fields, violation report, and hermetic no-Godot validation |
| 23.9 | D-NATIVE | Catalog native-owner/dependency/path/boundary fields, violation report, and hermetic no-Godot validation |
| 23.10 | D-NATIVE | Catalog native-owner/dependency/path/boundary fields, violation report, and hermetic no-Godot validation |

## Error handling and recovery

A missing path, symbol, criterion, design element, task, classification, native owner, Godot dependency declaration, Sim-native path, compatibility boundary, reuse evidence, no-Godot validation, confidence, or question is a catalog error. A source or architecture uncertainty is not converted into an implementation claim: it remains an open decision with dependent capability tasks blocked by review. Any proposed Godot process, library, server, command, wrapper, proxy, hidden instance, or shipped vendor/linkage dependency is a blocking native-ownership violation.

## Testing strategy

- Recompute the local Godot Git blob manifest and compare it with the official tag tree.
- Validate CSV schema, IDs, enum values, nonempty fields, unique capabilities, counts, and requirement/design/task references.
- Validate that every catalog row declares no Godot build/runtime dependency and records Sim-native storage, execution, UI, lifecycle, compatibility boundary, reuse evidence, and a hermetic no-Godot scenario.
- Scan all migration specs for plans that invoke, embed, wrap, link, vendor, or delegate execution to Godot and for Godot-specific abstractions that duplicate existing Sim owners.
- Run the feature-spec validator for this pack and every modified migration pack.
- During implementation, execute each catalog row's focused command plus scenario tests for success, failure, persistence, lifecycle, permissions, limits, cancellation, relevant platforms, package contents, linked dependencies, process trees, and operation with Godot absent.
