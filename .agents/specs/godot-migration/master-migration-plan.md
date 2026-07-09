# Sim Game Engine Migration Plan

## Purpose

Identify the specs, features, and functionality present in `projects/godot/`, `projects/world-model/`, and `projects/comfy/`, then plan their migration into Sim as native Sim features without duplicating Sim runtime infrastructure.

The target product is a cross-platform application for creating 2D and 3D games from a unified Sim interface. Godot contributes project structure, authoring concepts, scene/resource formats, scripting workflows, and export expectations. `projects/world-model` contributes the world foundation model harness used as the generative game engine: prompt and image conditioning, camera/action control, fast video inference, persistent serving, distributed GPU execution, diffusion graph pipelines, and generated 3D mesh workflows. `projects/comfy` contributes core world-model harness functionality: prompt and job APIs, WebSocket progress, typed node schemas, graph execution semantics, model folder/runtime policy, sampler and scheduler behavior, conditioning, latent/VAE execution, diffusion and world-model runner profiles, assets, blueprints, media-processing nodes, provider API nodes, custom-node extension loading, and packaging/test fixtures.

This is not a wholesale port of the classic Godot runtime. Sim should build a world-model engine harness around existing Sim UI, agent, task, media, and project infrastructure, while providing native Sim support for Godot-format projects and assets. Comfy is not a secondary compatibility target in that harness; implementation decisions for world-model graph orchestration, prompt/job lifecycle, model resolution, sampler/scheduler behavior, conditioning, diffusion/world-model execution, generated assets, media nodes, provider calls, and extensions must consult the Comfy-owned specs before introducing Sim-only behavior.

Every Godot-originated feature is modeled as a native Sim equivalent, but Godot compatibility is not the first implementation path. SimScript is the first-class executable Sim language, with natural language as the authoring interface. Legacy `.gd` files and Godot-format assets are migration/import sources, not the primary authoring model. Godot-origin task providers and scene preview routes become native Sim task providers and native Sim preview routes when they are product-enabling. There is no compatibility shim layer.

Every supported Comfy-derived feature is also recreated as native Sim functionality. Sim-owned records, services, workers, artifacts, states, queues, diagnostics, and registries use `Sim*` names such as `SimAssetSeedState`; `Comfy*` names are reserved for upstream source references, compatibility route shapes, and prose describing the source project.

## Value-First Sequencing

The highest-return path is to complete the Comfy and world-model harness before spending migration effort on Godot-format, Godot runtime, editor, export, XR, physics, networking, or other compatibility work. W2-W4 establish the native generation substrate: model serving diagnostics, Comfy model memory policy, prompt/job control, graph/node execution, sampler/scheduler/conditioning semantics, and generated artifact pipelines. W5 then exposes those capabilities through native Sim authoring, agent tools, natural-language workflows, and SimScript execution.

Godot-origin work is deferred into W7 unless it directly unlocks the target product. A deferred task can move earlier only with an explicit product-enabling reason, such as importing a required source project, surfacing a generated artifact in a Sim workspace, or supporting natural-language-to-SimScript authoring. Deferred does not mean abandoned: it means the feature becomes a native Sim equivalent when it contributes to the Comfy/world-model game-development product.

## Methodology

For each Godot, world-model, and Comfy feature area, assess:

- **Already exists in Sim**: no migration needed; document the correspondence.
- **Partially exists**: fill only the concrete gap.
- **New**: create a focused Sim feature/spec.
- **Do not migrate**: feature is a full game-engine runtime concern or duplicates Sim platform/rendering/runtime architecture.

Duplication rule: prefer existing Sim crates and extension points before adding new crates. Do not port Godot windowing, rendering, input, text shaping, physics, audio, networking, XR, or platform stacks directly when Sim already has its own equivalent. Where real-time world-model simulation replaces classic engine behavior, add explicit world-model harness components rather than copying Godot subsystems. Where world-model harness behavior overlaps Comfy prompt, graph, sampler, scheduler, conditioning, diffusion/world-model execution, model, asset, node, provider, or extension semantics, treat the Comfy spec as the functional starting point and document any safety-driven divergence.

## Execution Gates

| Gate | Name | Required Before | Exit Criteria |
|---|---|---|---|
| G0 | Spec consistency | Any code task | Every grouped spec has `requirements.md`, `design.md`, and `tasks.md`; task items include `_Requirements:` and `_writes:` |
| G1 | Boundary policy | Godot runtime-adjacent work | Excluded Godot runtime areas are encoded and tested |
| G2 | Shared Sim game metadata | Godot project, docs, asset, editor, and task integrations | Project descriptors, diagnostics, source references, fixture attribution, and parse status primitives exist |
| G3 | World-model foundations | World-model runtime, graph, mesh, agent, generated media, and authoring app work | Request, control, worker, graph, mesh, artifact, and provenance primitives exist |
| G4 | Worker safety | Python, GPU, remote, or persistent model execution | Serving diagnostics report setup problems without silent downloads |
| G5 | Graph safety | Diffusion graph execution | Graph validator rejects missing inputs, incompatible ports, and cycles |
| G6 | Provenance | Importing generated videos, meshes, textures, or exports | Artifact store preserves prompt, graph node, model settings, controls, source assets, and output paths |
| G7 | Dependency review | New vendored, native, heavy model, media, or mesh dependency | Review records license, maintenance, security, binary-size, and platform impact |
| G8 | Comfy harness alignment | World-model harness or Comfy-adjacent implementation decisions | Applicable Comfy spec is referenced, or safety/security/dependency/platform divergence is documented |
| G9 | Native Comfy recreation | Comfy-derived feature implementation | Supported Comfy-derived behavior is implemented with native Sim functionality, `Sim*` implementation names, and no ComfyUI pass-through |

## Dependency Waves

| Wave | Focus | Specs / Tasks | Depends On |
|---|---|---|---|
| W0 | Planning validation | Spec documents only; no code task starts until G0 passes | None |
| W1 | Shared foundations | Umbrella tasks 1 -> 14 are complete; `build-test-docs/` task 1 foundation helpers remain available if dependency-review or fixture helpers are needed | G0 |
| W2 | Value-first world-model serving substrate | `world-model-runtime/`, `model-serving-packaging/`, `comfy-model-memory-runtime/`, W2 portions of `comfy-packaging-quality/`, generated-media diagnostics/routing in `rendering-media/` | G3, G8, G9; G4 before real workers |
| W3 | Comfy execution core | `comfy-runtime-control-plane/`, `comfy-graph-node-runtime/`, `comfy-diffusion-world-model-runtime/`, W3 portions of `comfy-workflows-blueprints/`, `diffusion-graph-editor/` validation/execution planning | G3, G5, G8, G9; G4 before execution |
| W4 | Generation outputs and asset pipelines | `mesh-generation-pipeline/`, `comfy-asset-library/`, `comfy-media-node-pipelines/`, W4 portions of `comfy-workflows-blueprints/`, generated mesh/media previews, artifact import, generated asset provenance | G3, G5, G6, G8, G9 |
| W5 | Product authoring and agentic tools | `agentic-game-tools/`, `unified-authoring-app/`, native SimScript/natural-language authoring work, product editor affordances that consume W2-W4 capabilities | G2, G3, G5, G6, G8, G9 |
| W6 | Comfy provider, extension, and packaging hardening | `comfy-api-provider-nodes/`, `comfy-extension-ecosystem/`, W6 portions of `comfy-packaging-quality/`, persistent/remote worker hardening, provider policy gates | G4, G6, G7, G8, G9 |
| W7 | Deferred Godot-origin compatibility | `engine-core-runtime/`, legacy `.gd`/Godot C# parts of `language-scripting/`, legacy `game-formats-assets/`, `platform-export/`, `networking-collaboration/`, `xr-spatial/`, `physics-navigation/`, Godot run/debug/export/editor tasks | G1, G2, and explicit product-enabling dependency |

## Feature Inventory

| Feature Area | Source | Sim Equivalent | Decision | Location |
|---|---|---|---|---|
| Product goal and unified app | Godot editor + world model workflows | `crates/workspace`, `crates/project`, `crates/ui`, `crates/sim_apps` | Add unified game authoring app | `unified-authoring-app/` |
| Engine core and runtime metadata | `projects/godot/core`, scene/resource model | Rust std, `crates/project`, `crates/worktree`, `crates/fs` | Metadata/indexing only; no runtime port | `engine-core-runtime/` |
| Editor experience | Godot editor affordances | command palette, project panel, task/debugger UI | Add Sim editor affordances | `editor-experience/` |
| Rendering and media | Godot render/media/shader systems; generated video | GPUI/wgpu, media, image viewer | Preview and shader metadata only; generated media routing | `rendering-media/` |
| Platform and export | Godot platform/export templates | Sim platform crates, task system | External Godot task integration only | `platform-export/` |
| Language and scripting | SimScript, legacy `.gd`, natural-language authoring, Godot C#, class docs | language, LSP, docs infrastructure | Add native SimScript support, natural-language authoring, and docs indexing | `language-scripting/` |
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
| Comfy runtime control plane | `projects/comfy/server.py`, prompt queue, jobs, WebSocket events | Sim task, HTTP, WebSocket, media, and artifact systems | Add protocol adapter; do not port aiohttp app | `comfy-runtime-control-plane/` |
| Comfy graph/node runtime | `projects/comfy/nodes.py`, `comfy_execution/`, `comfy_extras/` | `crates/world_model`, diffusion graph primitives | Add Comfy schema, validation, execution, caching compatibility | `comfy-graph-node-runtime/` |
| Comfy model and memory runtime | `folder_paths.py`, `comfy/supported_models.py`, quantization/memory modules | model serving diagnostics, asset catalog | Add model folder catalog, model family detection, precision/device/memory policy | `comfy-model-memory-runtime/` |
| Comfy diffusion and world-model runtime | `comfy/samplers.py`, `comfy/sample.py`, `comfy/model_sampling.py`, `comfy/sd.py`, `comfy/ldm/**`, sampler/model nodes | worker diagnostics, graph runtime, model catalog | Add sampler, scheduler, conditioning, latent/VAE, model patch, and model-family execution semantics | `comfy-diffusion-world-model-runtime/` |
| Comfy asset library | `app/assets/`, user data, model browser | Sim artifact, media, storage, user data | Add asset API, tags, metadata, scans, and user-data compatibility | `comfy-asset-library/` |
| Comfy workflows and blueprints | `blueprints/`, workflow templates, subgraphs, node replacements | graph editor, authoring app, asset store | Add workflow/template/blueprint catalog and metadata import | `comfy-workflows-blueprints/` |
| Comfy media node pipelines | `comfy_extras/nodes_*`, media blueprints | Sim media, graph, mesh, artifact systems | Add media node capability groups and deterministic adapters | `comfy-media-node-pipelines/` |
| Comfy API provider nodes | `comfy_api_nodes/` providers | Sim secrets, policies, remote task/artifact systems | Add provider connector framework and policy gates | `comfy-api-provider-nodes/` |
| Comfy extension ecosystem | `custom_nodes/`, extension web dirs, manager hooks | Sim extension policy and diagnostics | Add controlled custom-node discovery/loading and extension assets | `comfy-extension-ecosystem/` |
| Comfy packaging and quality | CLI args, OpenAPI, examples, CI, frontend packages | Sim config, diagnostics, tests, dependency review | Add launch profile mapping, fixtures, schema catalog, dependency gates | `comfy-packaging-quality/` |

## Migration Specs

| Spec | Scope | Current Artifacts | Primary Wave |
|---|---|---|---|
| `engine-core-runtime/` | Core metadata, resources, and project model | Requirements + Design + Tasks | W7 |
| `editor-experience/` | Sim editor workflows for game development | Requirements + Design + Tasks | W5/W7 |
| `rendering-media/` | Preview/media/shader/generated-media support without rendering-stack duplication | Requirements + Design + Tasks | W2/W4/W7 |
| `platform-export/` | Godot project run/export task integration | Requirements + Design + Tasks | W7 |
| `language-scripting/` | SimScript, legacy `.gd`, natural-language authoring, and Godot C# language tooling | Requirements + Design + Tasks | W5/W7 |
| `game-formats-assets/` | Godot files, scenes, resources, and generated assets | Requirements + Design + Tasks | W4/W7 |
| `networking-collaboration/` | Godot protocol awareness and debug integration boundaries | Requirements + Design + Tasks | W7 |
| `xr-spatial/` | XR/spatial docs and metadata boundaries | Requirements + Design + Tasks | W7 |
| `physics-navigation/` | Physics/navigation documentation and metadata boundaries | Requirements + Design + Tasks | W7 |
| `build-test-docs/` | Docs ingestion, fixture conversion, third-party policy | Requirements + Design + Tasks | W1/W7 |
| `unified-authoring-app/` | Cross-platform game authoring app and workspace | Requirements + Design + Tasks | W5 |
| `world-model-runtime/` | LingBot/Wan world-model harness and interactive runtime | Requirements + Design + Tasks | W2 |
| `diffusion-graph-editor/` | Graph/node diffusion pipeline authoring and execution | Requirements + Design + Tasks | W3 |
| `mesh-generation-pipeline/` | Textured 3D mesh generation and export | Requirements + Design + Tasks | W4 |
| `agentic-game-tools/` | Agent tools for game design, pipeline editing, and asset generation | Requirements + Design + Tasks | W5 |
| `model-serving-packaging/` | Python worker, model downloads, GPU scheduling, and packaging | Requirements + Design + Tasks | W2/W6 |
| `comfy-runtime-control-plane/` | Comfy-compatible HTTP/WebSocket prompt, job, queue, progress, preview, and safety APIs | Requirements + Design + Tasks | W3 |
| `comfy-graph-node-runtime/` | Comfy node schema, graph validation, replacement, execution planning, caching, async/list execution | Requirements + Design + Tasks | W3 |
| `comfy-model-memory-runtime/` | Comfy model folders, model metadata, family detection, precision, device, and memory policy | Requirements + Design + Tasks | W2 |
| `comfy-diffusion-world-model-runtime/` | Comfy sampler, scheduler, conditioning, latent/VAE, model patch, diffusion runner, and world-model runner semantics | Requirements + Design + Tasks | W3 |
| `comfy-asset-library/` | Asset CRUD, upload/download, tags, metadata filters, seed scans, user data, settings, and output enrichment | Requirements + Design + Tasks | W4 |
| `comfy-workflows-blueprints/` | Blueprint catalog, workflow save/load/export, subgraphs, node replacements, embedded workflow metadata, app-mode metadata | Requirements + Design + Tasks | W3/W4 |
| `comfy-media-node-pipelines/` | Image, mask, video, audio, 3D, geometry, analysis, control, utility, and dataset node capability migration | Requirements + Design + Tasks | W4 |
| `comfy-api-provider-nodes/` | External provider node catalog, secrets, remote task lifecycle, media output import, policy and cost controls | Requirements + Design + Tasks | W6 |
| `comfy-extension-ecosystem/` | Custom node discovery/loading policy, extension assets, translations, templates, subgraphs, and manager boundary | Requirements + Design + Tasks | W6 |
| `comfy-packaging-quality/` | Launch profiles, feature flags, frontend package diagnostics, OpenAPI/schema fixtures, tests, dependency review, diagnostics | Requirements + Design + Tasks | W2/W6 |
