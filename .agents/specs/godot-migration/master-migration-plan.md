# Baymax + Godot + World Model Game Engine Migration Plan

## Purpose

Identify the specs, features, and functionality present in `projects/godot/` and `projects/world-model/`, then plan their migration into Baymax without duplicating Baymax runtime infrastructure.

The target product is a cross-platform application for creating 2D and 3D games from a unified Baymax + Godot-inspired interface. Godot contributes project structure, authoring concepts, scene/resource formats, scripting workflows, and export expectations. `projects/world-model` contributes the world foundation model harness used as the generative game engine: prompt and image conditioning, camera/action control, fast video inference, persistent serving, distributed GPU execution, diffusion graph pipelines, and generated 3D mesh workflows.

This is not a wholesale port of the classic Godot runtime. Baymax should build a world-model engine harness around existing Baymax UI, agent, task, media, and project infrastructure, while preserving compatibility paths for Godot-like projects and assets.

## Methodology

For each Godot and world-model feature area, assess:

- **Already exists in Baymax**: no migration needed; document the correspondence.
- **Partially exists**: fill only the concrete gap.
- **New**: create a focused Baymax feature/spec.
- **Do not migrate**: feature is a full game-engine runtime concern or duplicates Baymax platform/rendering/runtime architecture.

Duplication rule: prefer existing Baymax crates and extension points before adding new crates. Do not port Godot windowing, rendering, input, text shaping, physics, audio, networking, XR, or platform stacks directly when Baymax already has its own equivalent. Where real-time world-model simulation replaces classic engine behavior, add explicit world-model harness components rather than copying Godot subsystems.

## Execution Gates

| Gate | Name | Required Before | Exit Criteria |
|---|---|---|---|
| G0 | Spec consistency | Any code task | Every grouped spec has `requirements.md`, `design.md`, and `tasks.md`; task items include `_Requirements:` and `_writes:` |
| G1 | Boundary policy | Godot runtime-adjacent work | Excluded Godot runtime areas are encoded and tested |
| G2 | Shared Godot metadata | Godot project, docs, asset, editor, and task integrations | Project descriptors, diagnostics, source references, fixture attribution, and parse status primitives exist |
| G3 | World-model foundations | World-model runtime, graph, mesh, agent, generated media, and authoring app work | Request, control, worker, graph, mesh, artifact, and provenance primitives exist |
| G4 | Worker safety | Python, GPU, remote, or persistent model execution | Serving diagnostics report setup problems without silent downloads |
| G5 | Graph safety | Diffusion graph execution | Graph validator rejects missing inputs, incompatible ports, and cycles |
| G6 | Provenance | Importing generated videos, meshes, textures, or exports | Artifact store preserves prompt, graph node, model settings, controls, source assets, and output paths |
| G7 | Dependency review | New vendored, native, heavy model, media, or mesh dependency | Review records license, maintenance, security, binary-size, and platform impact |

## Dependency Waves

| Wave | Focus | Specs / Tasks | Depends On |
|---|---|---|---|
| W0 | Planning validation | Umbrella tasks 2, 13; all spec docs | None |
| W1 | Shared foundations | Umbrella tasks 1, 3, 5, 6, 8, 9, 10, 11, 12; `build-test-docs/` tasks 1, 3, 4 | G0 |
| W2 | Godot compatibility substrate | `engine-core-runtime/`, `language-scripting/`, `game-formats-assets/` metadata tasks, `build-test-docs/` docs/compat tasks | G1, G2 |
| W3 | World-model serving substrate | `world-model-runtime/`, `model-serving-packaging/`, generated-media routing in `rendering-media/` | G3, G4 |
| W4 | Authoring and graph UX | `diffusion-graph-editor/`, `unified-authoring-app/`, editor affordances, agent graph tools | G2, G3, G5 |
| W5 | Generation outputs and asset pipelines | `mesh-generation-pipeline/`, generated mesh asset integration, previews, export routing | G3, G5, G6 |
| W6 | External execution hardening | Godot run/export/debug, world-model persistent/remote execution, XR/physics external task fallbacks | G4, G6, G7 |

## Feature Inventory

| Feature Area | Source | Baymax Equivalent | Decision | Location |
|---|---|---|---|---|
| Product goal and unified app | Godot editor + world model workflows | `crates/workspace`, `crates/project`, `crates/ui`, `crates/baymax_apps` | Add unified game authoring app | `unified-authoring-app/` |
| Engine core and runtime metadata | `projects/godot/core`, scene/resource model | Rust std, `crates/project`, `crates/worktree`, `crates/fs` | Metadata/indexing only; no runtime port | `engine-core-runtime/` |
| Editor experience | Godot editor affordances | command palette, project panel, task/debugger UI | Add Godot-aware Baymax affordances | `editor-experience/` |
| Rendering and media | Godot render/media/shader systems; generated video | GPUI/wgpu, media, image viewer | Preview and shader metadata only; generated media routing | `rendering-media/` |
| Platform and export | Godot platform/export templates | Baymax platform crates, task system | External Godot task integration only | `platform-export/` |
| Language and scripting | GDScript, Godot C#, class docs | language, LSP, docs infrastructure | Add language support and docs indexing | `language-scripting/` |
| Game formats and assets | `.godot`, `.tscn`, `.tres`, `.import`, glTF | project/worktree/media | Add lightweight parser/indexer and generated asset registration | `game-formats-assets/` |
| Networking/collaboration | Multiplayer, ENet, WebRTC, debug protocols | collab, RPC, LSP, DAP | Do not port network runtime; optional debug metadata | `networking-collaboration/` |
| XR and spatial | OpenXR/WebXR/spatial metadata | media preview, docs | Docs/metadata boundaries only | `xr-spatial/` |
| Physics and navigation | physics servers, navigation mesh | docs/tasks | Metadata and external simulation fallback only | `physics-navigation/` |
| Build, test, docs | Godot build/docs/test corpus | existing build/docs/test infra | Docs ingestion, fixture attribution, dependency review | `build-test-docs/` |
| World-model runtime | `projects/world-model` Wan/LingBot harness | new `crates/world_model`, tasks, media | External worker/harness | `world-model-runtime/` |
| Diffusion graph editor | Node/flowchart pipelines | GPUI, task, media, agent | New typed graph editor | `diffusion-graph-editor/` |
| Mesh generation pipeline | textured 3D mesh generation | media/project preview | New mesh request/artifact/export pipeline | `mesh-generation-pipeline/` |
| Agentic game tools | graph edits, generation, asset tools | agent tool registry | Add game-specific tools | `agentic-game-tools/` |
| Model serving and packaging | Python env, weights, GPU, remote workers | task/process diagnostics | Add serving diagnostics and launcher traits | `model-serving-packaging/` |

## Migration Specs

| Spec | Scope | Current Artifacts | Primary Wave |
|---|---|---|---|
| `engine-core-runtime/` | Core metadata, resources, and project model | Requirements + Design + Tasks | W2 |
| `editor-experience/` | Godot-aware Baymax editor workflows | Requirements + Design + Tasks | W4/W6 |
| `rendering-media/` | Preview/media/shader/generated-media support without rendering-stack duplication | Requirements + Design + Tasks | W3/W5 |
| `platform-export/` | Godot project run/export task integration | Requirements + Design + Tasks | W6 |
| `language-scripting/` | GDScript and Godot C# language tooling | Requirements + Design + Tasks | W2 |
| `game-formats-assets/` | Godot files, scenes, resources, and generated assets | Requirements + Design + Tasks | W2/W5 |
| `networking-collaboration/` | Godot protocol awareness and debug integration boundaries | Requirements + Design + Tasks | W6 |
| `xr-spatial/` | XR/spatial docs and metadata boundaries | Requirements + Design + Tasks | W6 |
| `physics-navigation/` | Physics/navigation documentation and metadata boundaries | Requirements + Design + Tasks | W6 |
| `build-test-docs/` | Docs ingestion, fixture conversion, third-party policy | Requirements + Design + Tasks | W1/W2 |
| `unified-authoring-app/` | Cross-platform game authoring app and workspace | Requirements + Design + Tasks | W4 |
| `world-model-runtime/` | LingBot/Wan world-model harness and interactive runtime | Requirements + Design + Tasks | W3 |
| `diffusion-graph-editor/` | Graph/node diffusion pipeline authoring and execution | Requirements + Design + Tasks | W4 |
| `mesh-generation-pipeline/` | Textured 3D mesh generation and export | Requirements + Design + Tasks | W5 |
| `agentic-game-tools/` | Agent tools for game design, pipeline editing, and asset generation | Requirements + Design + Tasks | W4/W5 |
| `model-serving-packaging/` | Python worker, model downloads, GPU scheduling, and packaging | Requirements + Design + Tasks | W3/W6 |
