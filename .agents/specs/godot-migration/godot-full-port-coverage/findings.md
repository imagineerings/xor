# Godot migration audit findings

## Native Sim constraint audit

The inspected Sim manifest and source tree contain no connected Godot editor/engine executable, shared-library, server, command-line, or runtime dependency. The following specification plans violated the new constraint before this update and have been corrected in the specification only:

| Prior plan | Violation | Specification correction |
| --- | --- | --- |
| Root `RuntimeBoundaryPolicy::ExternalCommand`, external task-provider wiring, and `sim_game` registry/gatekeeper tasks | Treated Godot delegation as a coverage path and proposed a parallel product/governance subsystem | External execution removed; governance stays in the catalog/validators; project/language/task/preview behavior extends existing owners |
| `engine-core-runtime` external-command scene execution and `crates/sim_game` metadata registry | Delegated execution and duplicated project/worktree/language ownership | Runtime is native, unresolved, or excluded; metadata writes target existing owners |
| `editor-experience` configured Godot run/debug tasks and missing-executable setup guidance | Launched Godot for claimed run/debug support | Supported run/debug must be Sim-owned; absent native runtime is unresolved/unsupported |
| `platform-export` executable settings and Godot export task templates | Claimed export through Godot invocation rather than Sim packaging | Presets become compatibility inputs to Sim-owned packaging; unsupported platform targets remain unresolved/excluded |
| `physics-navigation` fallback tasks and external-command diagnostics | Metadata/task placeholders delegated simulation | Metadata remains non-executable; runtime requires a native owner or explicit exclusion/decision |
| `networking-collaboration` external-command decision and Godot-specific protocol records | Left a delegated runtime path and duplicated network/DAP owners | Existing `net`/HTTP/RPC/collab/DAP owners are authoritative; unsupported gameplay protocols remain excluded/decision-blocked |
| `xr-spatial` external fallback hooks | Treated a fallback handoff as support | Only native Sim metadata/preview is supported; runtime stays excluded/decision-blocked |
| Master plan external Godot export and simulation fallback entries | Contradicted the native integration principle | Replaced with native owner gates and explicit unresolved/excluded outcomes |

No remaining spec may treat these historical plans as implementation evidence.

## Placeholder-only and declaration-only support claims

- `language-scripting` previously called `.gd` files “SimScript source-compatible” without grammar, semantic, translation, lifecycle, or execution proof. They are now migration sources until native evidence exists.
- `platform-export` treated parsed presets and task templates as export support. Native packaging, signing, deployment, cleanup, and artifact execution are now required.
- `game-formats-assets` treated classifiers, metadata links, and generic external tools as import coverage. Successful imports must now produce Sim-native resources/caches/dependencies without Godot.
- `physics-navigation`, `networking-collaboration`, and `xr-spatial` used boundary records, docs, interfaces, or task hooks for runtime-facing families. Those artifacts no longer count as executable support.
- Root and grouped specs referenced absent `sim_game`, `world_model`, `SimGame*`, and other proposed declarations. A type, crate name, interface, task checkbox, or spec marker remains planning evidence only until connected behavior and no-Godot validation exist.

## License, linkage, and distribution risks

- No current Sim manifest entry was found that links or depends on Godot itself. Adding any Godot executable, engine/editor library, server, command-line tool, generated binding, or vendor patch is prohibited by the native constraint.
- Godot source is MIT-licensed at the upstream level, but exact source/generated-code copying is still blocked pending review of provenance, architecture, maintenance, notices, derivative changes, and bundled third-party material.
- `projects/godot/thirdparty` contains independently licensed dependencies; copying codecs, fonts, icons, certificates, platform libraries, patches, templates, or generated outputs can introduce separate attribution, copyleft, patent, linkage, and redistribution obligations.
- GDExtension-compatible binary loading, a Sim-owned ABI host, Mono/C# support, codecs, native importers/exporters, platform SDK/toolchains, signing tools, OpenXR/WebXR integrations, and console/platform SDKs each require exact dependency and distribution review before specification can claim native support.
- Documentation, fixtures, examples, test data, translations, fonts, icons, and assets remain evidence unless an exact-material licensing and architecture decision approves copying.

## Material native product and architecture decisions

The hard constraint removes external Godot execution from the option set but does not decide native product scope. `decisions.md` retains decisions for runtime breadth/owner composition, compatibility versions, script semantics, renderer/simulation tiers, platforms, extensions/plugins, import/export breadth, source reuse, trust/permissions, and resource/performance limits. Dependent capabilities remain unresolved, excluded, or partial until those choices are approved.

## Suspected specification overclaims

The pre-audit task files marked every task complete. Those marks were not supported by the inspected Sim tree and have been reset to unchecked.

- Root `tasks.md` claimed completion of a new `crates/sim_game`, `crates/world_model`, registration, parsers, boundary policy, and gatekeeper. Neither crate is a workspace member, and the design's `crates/sim/src/sim.rs#register_game_integration` and `crates/sim_game/*` implementation markers do not resolve.
- `engine-core-runtime` claimed completed Godot project/resource metadata, but no connected `project.godot`, `.tscn`, `.tres`, project descriptor, or resource index implementation exists.
- `editor-experience`, `game-formats-assets`, `platform-export`, `language-scripting`, `rendering-media`, `networking-collaboration`, `physics-navigation`, and `xr-spatial` claimed completed tasks, but repository-wide searches found no Godot integration points matching their promised behavior.
- `unified-authoring-app`, `world-model-runtime`, `diffusion-graph-editor`, `mesh-generation-pipeline`, `agentic-game-tools`, and `model-serving-packaging` referenced `crates/world_model`; that crate is absent. Existing `crates/comfy_*` code may own adjacent model/workflow behavior, but it is not evidence for Godot editor/runtime parity.
- Broad phrases such as “runtime excluded,” “metadata only,” “native Sim command,” and “task template” were used as if they covered complete renderer, physics, networking, XR, editor, export, and script families. They do not establish native execution, platform/failure/lifecycle behavior, or operation without Godot.

The Comfy sub-specifications were not counted in the 198-capability Godot denominator. Their previous checkmarks were reset because the user required every task in this migration tree to remain unchecked; this audit does not re-audit the separate Comfy parity program.

## Reuse and extension opportunities

| Godot responsibility | Existing Sim owner to reuse | Confirmed gap |
| --- | --- | --- |
| Project discovery, worktrees, recent projects, sessions | `project`, `worktree`, `workspace`, `recent_projects`, `session` | Godot manifest/features/cache/lifecycle semantics |
| Files, watchers, dependencies, import state | `fs`, `worktree`, `project`, `project_panel` | Godot UID/import/remap/reimport contracts |
| Editor surfaces, menus, commands, shortcuts, settings | `workspace`, `editor`, `project_panel`, `inspector_ui`, `command_palette`, `menu`, `keymap_editor`, `settings_ui` | Scene/asset-specific models and connected commands |
| Rendering and preview | `gpui`, `gpui_wgpu`, `image_viewer`, `svg_preview`, `component_preview`, `audio` | Godot scene/runtime rendering is not equivalent to GPUI UI rendering |
| Language tooling and debugging | `language`, `languages`, `lsp`, `dap`, `debugger_ui`, `diagnostics` | GDScript/C#/Godot protocol semantics and runtime connection |
| Tasks, CLI, terminal, remote/headless processes | `task`, `cli`, `terminal`, `remote_server` | Native runtime/CLI ownership, compatible option mapping, lifecycle, cancellation, artifacts, and exit diagnostics without Godot |
| Filesystem, HTTP, RPC, collaboration | `fs`, `net`, `http_client`, `rpc`, `collab` | Exported-runtime API compatibility and gameplay multiplayer semantics |
| Extensions and trust | `extension`, `extension_host`, `extension_api`, `extensions_ui`, `sandbox` | GDExtension ABI and EditorPlugin lifecycle; do not create a second plugin manager |
| Secrets, permissions, TLS, sandboxing | `credentials_provider`, `http_client_tls`, `sandbox`, `settings` | Imported-project trust and exported mobile/web permission contracts |
| Persistence and migration | `settings`, `session`, `workspace` persistence, `db`, `migrator` | Godot format/version/atomicity/recovery contracts |
| Platform implementation | `gpui_windows`, `gpui_macos`, `gpui_linux`, `gpui_web` | These are Sim application platforms, not Android/iOS/visionOS/game-runtime compatibility |
| Tests, docs, CI, licenses | existing crate tests, `script/*`, `docs_preprocessor`, `.github/workflows`, compliance tooling | Godot fixtures, compatibility matrix, platform tiers, and attribution |

## Likely duplicate planned implementations

- The proposed `sim_game` project, language, task, preview, diagnostics, and workspace registries overlap existing `project`, `worktree`, `language`, `task`, `workspace`, `project_panel`, and preview crates. Those owners remain authoritative; a materially new native runtime component requires an explicit architecture decision and cannot duplicate their registries.
- `RuntimeBoundaryPolicy` and `MigrationGatekeeper` are specification-governance concepts. A product crate should not be created merely to encode the audit; the spec validator and catalog validator own planning integrity.
- Godot networking/collaboration adapters must not duplicate `net`, `http_client`, `rpc`, `collab`, or DAP transport. Gameplay protocol compatibility, if approved, should be layered explicitly over those owners.
- Godot editor plugin management must reuse `extension`, `extension_host`, `extension_api`, and `extensions_ui` trust/install/UI services.
- Godot settings, editor layout, recent-project, session, and migration state must reuse `settings`, `workspace` persistence, `recent_projects`, `session`, `db`, and `migrator`.
- Preview proposals must reuse `image_viewer`, `svg_preview`, `component_preview`, `audio`, and GPUI render surfaces while documenting that they are not a Godot exported-runtime renderer.
- World-model and diffusion planning under the Godot folder overlaps the implemented `comfy_*` crates and the separate Comfy parity specification. Those components should be referenced as shared product functionality, not represented as Godot migration coverage.

## Overlapping or contradictory specifications

- Root requirement 14 defers Godot-origin work behind Comfy/world-model waves, while this audit's goal is comprehensive Godot migration coverage. Scheduling may remain value-driven, but it cannot be used as a coverage classification.
- Root and grouped designs say “native Sim equivalent,” while rendering, physics/navigation, networking, XR, and scene-tree execution are blanket-excluded. External Godot cannot satisfy the product goal; DEC-GODOT-001/004 decide only the native scope versus explicit exclusion.
- `language-scripting` calls `.gd` “SimScript source-compatible” without grammar, semantic, runtime, or migration evidence and simultaneously calls SimScript the primary executable language. This conflicts with observable GDScript/C# project behavior until DEC-GODOT-003 is resolved.
- `engine-core-runtime` proposes a `sim_game` crate even though root anti-duplication criteria require existing owners and the repository already has project/worktree/language/task/diagnostic integration points.
- `platform-export` previously described Godot tasks as native Sim export behavior. It now requires Sim-owned packaging, signing, templates, deployment, cleanup, artifacts, and no-Godot execution; the concrete native platform tiers remain unresolved.
- `rendering-media` excludes audio and text servers together with render backends while the editor/preview specs promise shader, video, generated-media, and runtime preview experiences. The exact preview-versus-runtime contract is absent.
- `physics-navigation`, `networking-collaboration`, and `xr-spatial` treat metadata/docs as sufficient despite the migration goal naming exported runtime behavior; their exclusions require explicit product approval and cannot be bypassed through external runtime verification.
- Comfy/world-model specs are adjacent product work, not evidence that Godot project, editor, runtime, format, importer, exporter, or platform capabilities are covered.

## Intentionally excluded capabilities

The baseline catalog has 37 classification-6 rows: eight 2D renderer families, ten 3D renderer families, four scene-runtime families, seven physics/navigation/particle families, seven WebSocket/WebRTC/ENet/high-level-multiplayer/UPnP/Web bridge families, and XR runtime. No Sim-native replacement evidence exists. These exclusions remain provisional pending DEC-GODOT-001 and DEC-GODOT-004; they must not be described as implemented parity and cannot fall back to Godot.

## Upstream-only infrastructure

The baseline catalog has 12 classification-7 rows: unsupported out-of-tree/console platforms; Godot unit-test harness; class-reference generation; platform CI/static checks; external demos/examples; generated module registration; vendored dependencies; source/build generators; SCons/toolchain machinery; upstream release engineering; and docs/fixtures/generated assets before a connected Sim consumer exists. Sim should reuse its existing CI, compliance, build, documentation, and release systems while preserving the behavior and provenance these inputs support.
