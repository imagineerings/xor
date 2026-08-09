#!/usr/bin/env python3

import csv
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).parent
CATALOG = ROOT / "catalogs" / "master-coverage.csv"

CLASSIFICATIONS = {
    1: "Already implemented in Sim and reusable without changes",
    2: "Partially implemented in Sim and should be extended",
    3: "Fully covered by an existing Godot migration spec",
    4: "Partially covered by an existing migration spec",
    5: "Missing from the migration specs",
    6: "Intentionally excluded, with a documented rationale",
    7: "Internal/upstream infrastructure that does not require a direct port",
}


@dataclass(frozen=True)
class Domain:
    code: str
    name: str
    subdomain: str
    modes: str
    lifecycle: str
    godot_evidence: str
    sim_evidence: str
    baseline_spec: str
    owner: str
    writes: str
    validation: str
    question: str
    capabilities: tuple[tuple[str, int], ...]


DOMAINS = (
    Domain(
        "PROJ",
        "Project manager and lifecycle",
        "discovery, creation, import, launch, recovery",
        "Editor project manager and CLI project discovery on desktop; Android editor has a distinct project manager; exported runtimes consume one project root.",
        "Successful operations persist project metadata and recent/favorite state; malformed manifests, missing paths, unsupported features, and upgrade failures remain recoverable and diagnostic.",
        "projects/godot/editor/project_manager/project_manager.cpp::ProjectManager; projects/godot/editor/project_manager/project_dialog.cpp::ProjectDialog; projects/godot/core/config/project_settings.cpp::ProjectSettings::setup",
        "crates/project/src/project.rs::Project; crates/workspace/src/workspace.rs::Workspace; crates/recent_projects/src/recent_projects.rs::RecentProjects; no project.godot integration was found",
        "engine-core-runtime R2.1-R2.2 / Properties 2-3 / Task 1; editor-experience R1.1-R1.2 / Property 1 / Task 1",
        "project + workspace + recent_projects",
        "crates/project/src/project.rs, crates/workspace/src/workspace.rs, crates/recent_projects/src/recent_projects.rs",
        "cargo test -p project -p workspace -p recent_projects godot",
        "Which create, upgrade, import, and launch behaviors are mandatory native Sim capabilities, and which are intentionally excluded?",
        (
            ("create a project with name, path, renderer, version-control metadata, and default files", 5),
            ("import an existing project.godot and reject invalid or duplicate roots without losing user data", 4),
            ("scan, sort, filter, favorite, rename, remove, and reopen projects in the project manager", 2),
            ("persist recent projects, favorites, tags, sort mode, and missing-project state", 2),
            ("parse project features, application metadata, main scene, autoloads, input map, and rendering settings", 4),
            ("start the editor, project manager, or game based on project discovery and command-line mode", 5),
            ("detect incompatible engine versions and offer project conversion or manager-assisted upgrade", 5),
            ("open in safe mode after editor/plugin failure and recover unsaved scene state", 5),
            ("install and instantiate project templates while surfacing download and extraction failures", 5),
            ("use per-project .godot data and cache roots without treating generated metadata as source", 5),
            ("apply project settings overrides and feature-tag-specific overrides with deterministic precedence", 5),
        ),
    ),
    Domain(
        "SCENE",
        "Scene, node, resource, and serialization",
        "runtime object graph and file contracts",
        "Editor and exported runtime; text and binary resource formats; tool/editor and runtime lifecycle differ; compatibility loaders are version-sensitive.",
        "Scene ownership, groups, signals, resources, UIDs, dependencies, caches, and lifecycle notifications survive load/save/instantiate; cycles, missing dependencies, corrupt data, and incompatible versions produce errors or placeholders rather than silent loss.",
        "projects/godot/scene/main/node.cpp::Node; projects/godot/scene/main/scene_tree.cpp::SceneTree; projects/godot/scene/resources/packed_scene.cpp::PackedScene; projects/godot/core/io/resource_loader.cpp::ResourceLoader; projects/godot/scene/resources/resource_format_text.cpp::ResourceFormatLoaderText",
        "crates/project/src/project.rs::Project and crates/worktree/src/worktree.rs::Worktree provide file/project ownership only; Cargo.toml has no Godot scene runtime owner",
        "engine-core-runtime R1.1-R1.3,R3.1-R3.2 / Properties 1-3 / Task 1; game-formats-assets R1.1 / Property 1 / Task 1",
        "project + worktree + language",
        "crates/project/src/project.rs, crates/worktree/src/worktree.rs, crates/language/src/language_registry.rs",
        "cargo test -p project -p worktree -p language godot_scene",
        "Does the product require Sim-owned executable scene trees, or only lossless native import/editing with runtime execution explicitly unresolved or excluded?",
        (
            ("create, parent, reorder, name, own, group, and free nodes while preserving scene-tree invariants", 6),
            ("deliver enter-tree, ready, process, physics-process, pause, exit-tree, and deletion lifecycle notifications", 6),
            ("connect, persist, emit, disconnect, and inspect typed and deferred signals", 6),
            ("pack, instantiate, inherit, edit, save, reload, and revert scenes with editable children and ownership", 4),
            ("load, preload, cache, duplicate, localize, reference-count, and release resources", 4),
            ("round-trip .tscn and .tres values, subresources, ext_resources, scripts, and connection records", 4),
            ("round-trip binary .scn and .res resources with version and endianness compatibility", 5),
            ("assign stable resource UIDs and repair moved dependency paths without corrupting references", 5),
            ("enumerate dependencies and surface missing, cyclic, corrupt, or type-mismatched resources", 4),
            ("serialize Variant values, exported properties, dictionaries, arrays, typed containers, and object references", 5),
            ("apply autoload singletons and change, reload, and quit the active scene predictably", 6),
            ("preserve unknown or newer-format data sufficiently for non-destructive migration", 5),
        ),
    ),
    Domain(
        "EDITOR",
        "Editor workspace and authoring surfaces",
        "workspaces, docks, inspector, commands, settings",
        "Desktop editor primarily; embedded/mobile editor omits or adapts surfaces; single-window and multi-window modes; project-specific and global settings.",
        "Selection, edit history, dock/layout state, shortcuts, open scenes, and unsaved state persist where promised; unavailable tools, invalid selections, and failed saves remain visible and recoverable.",
        "projects/godot/editor/editor_node.cpp::EditorNode; projects/godot/editor/docks/scene_tree_dock.cpp::SceneTreeDock; projects/godot/editor/docks/filesystem_dock.cpp::FileSystemDock; projects/godot/editor/inspector/editor_inspector.cpp::EditorInspector; projects/godot/editor/settings/editor_settings.cpp::EditorSettings",
        "crates/workspace/src/workspace.rs::Workspace; crates/project_panel/src/project_panel.rs::ProjectPanel; crates/inspector_ui/src/inspector_ui.rs::Inspector; crates/command_palette/src/command_palette.rs::CommandPalette; crates/settings_ui/src/settings_ui.rs; no Godot scene editor connection was found",
        "editor-experience R1.1-R3.2 / Properties 1-3 / Task 1; unified-authoring-app R1.1-R2.2 / Properties 1-2 / Task 1",
        "workspace + project_panel + inspector_ui + editor + command_palette",
        "crates/workspace/src/workspace.rs, crates/project_panel/src/project_panel.rs, crates/inspector_ui/src/inspector_ui.rs",
        "cargo test -p workspace -p project_panel -p inspector_ui godot",
        "Which Godot editor surfaces are mandatory native Sim authoring experiences, and which are intentionally excluded rather than delegated?",
        (
            ("restore open scenes, selected objects, bottom panels, docks, and workspace layout per project", 2),
            ("browse and manipulate the scene tree with create, rename, reparent, group, visibility, and ownership operations", 4),
            ("browse project files with type filters, favorites, move/rename dependency repair, and reimport state", 4),
            ("inspect and edit grouped, typed, ranged, resource, node-path, and script-exposed properties", 4),
            ("edit scenes through dedicated 2D, 3D, script, asset-library, and game workspaces", 5),
            ("provide searchable menus and command palette actions with context-sensitive enablement", 1),
            ("configure and resolve user, project, feature-tag, and platform-specific editor settings", 2),
            ("edit shortcuts, chords, physical keys, and platform variants with conflict diagnostics", 1),
            ("perform undo, redo, history navigation, inspector pinning, and multi-object edits without reentrant updates", 2),
            ("save, save-as, save-all, autosave, recover, and warn before closing unsaved resources", 2),
            ("run and stop the main scene, current scene, selected scene, and custom runnable with embedded game controls", 4),
            ("expose output, debugger, profiler, audio, animation, shader, navigation, and import bottom panels", 5),
            ("support distraction-free, multi-window, presentation, and embedded-play layout modes", 5),
            ("search help and class reference by class, method, property, signal, constant, and theme item", 2),
        ),
    ),
    Domain(
        "R2D",
        "2D rendering",
        "canvas scene and renderer",
        "Editor viewport and exported runtime; Vulkan/Metal/D3D12 rendering-device paths and GLES3 compatibility path; low-end and mobile constraints vary.",
        "Canvas state is frame-driven and viewport-scoped; resource loss and unsupported shaders have explicit fallbacks or errors; visual output, culling, batching, and ordering remain deterministic for the same inputs.",
        "projects/godot/scene/main/canvas_item.cpp::CanvasItem; projects/godot/servers/rendering/rendering_server_default.cpp::RenderingServerDefault; projects/godot/servers/rendering/renderer_canvas_cull.cpp::RendererCanvasCull; projects/godot/scene/2d/tile_map_layer.cpp::TileMapLayer",
        "crates/gpui/src/element.rs::Element and crates/gpui_wgpu/src/wgpu_renderer.rs::WgpuRenderer render Sim UI, not Godot CanvasItem scenes",
        "rendering-media R1.1-R2.2 / Property 1 / Task 1 documents a blanket render-backend exclusion but no behavioral replacement",
        "gpui + gpui_wgpu + image_viewer",
        "crates/gpui/src/element.rs, crates/gpui_wgpu/src/wgpu_renderer.rs, crates/image_viewer/src/image_viewer.rs",
        "cargo test -p gpui -p gpui_wgpu -p image_viewer godot_canvas",
        "Must Sim own exported-runtime CanvasItem behavior, or is the capability intentionally excluded or narrowed to native asset preview?",
        (
            ("compose CanvasItem and Node2D transforms, visibility, modulation, clipping, z-order, y-sort, and draw commands", 6),
            ("render sprites, regions, nine-patches, polygons, lines, text, and texture rectangles with filtering and repeat modes", 6),
            ("author and render tile sets and tile-map layers with terrains, alternatives, patterns, quadrants, and navigation/physics metadata", 6),
            ("render 2D lights, normal/specular maps, occluders, shadow atlases, masks, and blend modes", 6),
            ("execute canvas shaders and materials with uniforms, screen textures, time, and instance parameters", 6),
            ("deform 2D skeletons, bones, polygons, and particles and preview the result in the editor", 6),
            ("cull and batch canvas items while preserving draw order and viewport isolation", 6),
            ("render SubViewport output to textures and embed or capture the result", 6),
            ("preview common Godot image and texture assets without executing the Godot renderer", 3),
        ),
    ),
    Domain(
        "R3D",
        "3D rendering",
        "scene renderer, materials, lighting, post-processing",
        "Forward+, Mobile, and Compatibility renderers; Vulkan/Metal/D3D12/GLES3 drivers vary by platform; editor previews and exported runtimes share resources but not tooling.",
        "Frame and resource lifecycles cover device creation/loss, shader compile failure, streaming, visibility, and viewport resize; unsupported driver/features fail visibly or choose a documented fallback.",
        "projects/godot/scene/3d/node_3d.cpp::Node3D; projects/godot/servers/rendering/renderer_rd/renderer_scene_render_rd.cpp::RendererSceneRenderRD; projects/godot/servers/rendering/renderer_scene_cull.cpp::RendererSceneCull; projects/godot/scene/resources/environment.cpp::Environment",
        "crates/gpui_wgpu/src/wgpu_renderer.rs::WgpuRenderer and crates/component_preview/src/component_preview.rs preview Sim UI; no Godot 3D scene renderer or camera pipeline was found",
        "rendering-media R1.1 / Property 1 / Task 1 excludes renderer migration; mesh-generation-pipeline covers generated mesh artifact metadata only",
        "gpui_wgpu + component_preview + image_viewer",
        "crates/gpui_wgpu/src/wgpu_renderer.rs, crates/component_preview/src/component_preview.rs, crates/image_viewer/src/image_viewer.rs",
        "cargo test -p gpui_wgpu -p component_preview godot_3d",
        "Which visual parity tier will a Sim-native renderer own, and which unsupported behaviors are intentionally excluded or limited to asset preview?",
        (
            ("compose Node3D transforms, visibility, layers, top-level state, and camera projections", 6),
            ("render meshes, surfaces, blend shapes, skeleton skinning, MultiMesh instances, and material overrides", 6),
            ("render standard, ORM, shader, particle, fog, sky, and post-process materials with platform fallbacks", 6),
            ("render directional, omni, and spot lights with shadows, cookies, distance fade, and culling masks", 6),
            ("apply environments, sky, fog, exposure, tone mapping, glow, SSAO, SSIL, SSR, DOF, and color adjustment", 6),
            ("select Forward+, Mobile, or Compatibility rendering and report unavailable driver/feature combinations", 6),
            ("compile and execute spatial, sky, fog, and compute shaders with include and uniform dependency tracking", 6),
            ("perform visibility range, frustum, occlusion, LOD, portal-like room, and instance culling", 6),
            ("bake and consume lightmaps, probes, voxel GI, SDFGI, reflection probes, and environment captures", 6),
            ("render nested viewports, camera feeds, render targets, scaling, MSAA, TAA, FSR, and screen capture", 6),
            ("preview imported meshes and materials with orbit, lighting, animation, and failure diagnostics", 4),
        ),
    ),
    Domain(
        "UI",
        "UI/control framework and themes",
        "runtime Control tree and editor UI reuse",
        "Editor and exported runtime; desktop, mobile, web, RTL, accessibility, pointer, touch, keyboard, and controller navigation variants.",
        "Layout and theme changes invalidate predictably; focus/event ownership follows tree lifecycle; invalid theme resources or inaccessible actions degrade visibly without trapping input.",
        "projects/godot/scene/gui/control.cpp::Control; projects/godot/scene/gui/container.cpp::Container; projects/godot/scene/resources/theme.cpp::Theme; projects/godot/scene/gui/text_edit.cpp::TextEdit; projects/godot/scene/gui/rich_text_label.cpp::RichTextLabel",
        "crates/gpui/src/elements/div.rs::Div; crates/ui/src/ui.rs::init; crates/theme/src/theme.rs::Theme; crates/ui_input/src/ui_input.rs; Sim editor UI is not Godot exported-runtime Control semantics",
        "editor-experience and unified-authoring-app partially cover authoring UI; rendering-media excludes Godot UI/text runtime stacks",
        "gpui + ui + theme + ui_input",
        "crates/ui/src/ui.rs, crates/theme/src/theme.rs, crates/ui_input/src/ui_input.rs",
        "cargo test -p ui -p theme -p ui_input godot_control",
        "May Godot Control scenes be translated to GPUI for authoring only, or must exported runtime behavior and theme compatibility also be preserved?",
        (
            ("lay out Controls using anchors, offsets, grow directions, minimum sizes, containers, aspect ratios, and RTL mirroring", 2),
            ("route mouse, touch, keyboard, controller, shortcut, focus, tooltip, and drag/drop events through Control hierarchy", 2),
            ("provide buttons, ranges, lists, trees, tabs, menus, dialogs, color/file pickers, splitters, and scroll containers", 2),
            ("edit plain and rich text with selection, undo, syntax, bidi, shaping, images, tables, meta links, and IME", 2),
            ("resolve theme inheritance, type variations, icons, fonts, sizes, colors, style boxes, and live overrides", 2),
            ("manage popups, modal dialogs, embedded windows, exclusive state, and safe cancellation", 2),
            ("expose accessible roles, names, values, actions, focus, and tree updates to platform assistive technology", 2),
            ("preview and migrate Godot UI scenes without claiming GPUI is runtime-compatible by default", 5),
        ),
    ),
    Domain(
        "INPUT",
        "Input, windowing, display, accessibility, and internationalization",
        "devices, actions, windows, locale, assistive technology",
        "Desktop, mobile, web, XR, headless; X11/Wayland and native Windows/macOS differ; locale, IME, RTL, DPI, and assistive technology are platform-sensitive.",
        "Hotplug, focus, resize, suspend/resume, locale change, screen-reader activation, and permission transitions have explicit lifecycle behavior; unavailable devices and denied permissions remain diagnostic.",
        "projects/godot/core/input/input.cpp::Input; projects/godot/core/input/input_map.cpp::InputMap; projects/godot/servers/display/display_server.cpp::DisplayServer; projects/godot/core/string/translation_server.cpp::TranslationServer; projects/godot/drivers/accesskit/accessibility_server_accesskit.cpp::AccessibilityServerAccessKit",
        "crates/gpui/src/platform.rs::PlatformWindow,PlatformInput,A11yCallbacks; crates/gpui/src/keymap.rs::Keymap; crates/gpui_web/src/events.rs; no Godot InputMap or exported-runtime device layer was found",
        "No dedicated migration spec; editor-experience only covers shortcuts and platform-export only states a broad platform boundary",
        "gpui + gpui_platform + keymap_editor + settings",
        "crates/gpui/src/platform.rs, crates/gpui_platform/src/gpui_platform.rs, crates/settings/src/settings.rs",
        "cargo test -p gpui -p gpui_platform -p keymap_editor godot_input",
        "Which runtime input/display contracts must Sim reproduce beyond its editor-window platform layer?",
        (
            ("define InputMap actions, deadzones, physical/logical keys, device filters, and multiple event bindings", 5),
            ("report pressed, just-pressed, just-released, strength, vector, mouse velocity, and accumulated input deterministically", 5),
            ("handle keyboard, mouse, pen, touch, gestures, gamepads, hotplug, mappings, vibration, sensors, and emulation", 5),
            ("create and manage multiple windows, screens, modes, flags, focus, DPI, scale, vsync, orientation, and safe areas", 2),
            ("support clipboard, cursor, mouse modes, virtual keyboard, IME composition, and text input", 2),
            ("expose accessibility activation, semantic trees, actions, bounds, focus, announcements, and deactivation", 2),
            ("load translations, select locale and fallbacks, pluralize, remap resources, shape bidi text, and mirror layout", 5),
            ("handle suspend, resume, low-memory, quit, focus, file-drop, and platform notification events", 5),
            ("provide dummy/headless display, audio, input, and text drivers with explicit unsupported behavior", 5),
        ),
    ),
    Domain(
        "SIM",
        "Physics, navigation, animation, audio, and particles",
        "simulation services",
        "Editor tools and exported runtime; 2D and 3D servers can be disabled independently; Godot Physics and Jolt are selectable; audio/device backends and GPU features vary.",
        "Fixed-step simulation, pause, activation, sleep, resource creation/free, baking, playback, seek, bus routing, and device changes are lifecycle-bound; invalid data and unavailable backends remain diagnostic.",
        "projects/godot/servers/physics_2d/physics_server_2d.cpp::PhysicsServer2D; projects/godot/servers/physics_3d/physics_server_3d.cpp::PhysicsServer3D; projects/godot/servers/navigation_3d/navigation_server_3d.cpp::NavigationServer3D; projects/godot/scene/animation/animation_player.cpp::AnimationPlayer; projects/godot/servers/audio/audio_server.cpp::AudioServer; projects/godot/scene/3d/gpu_particles_3d.cpp::GPUParticles3D",
        "Cargo.toml::workspace.members has no physics/navigation/game runtime; crates/audio/src/audio.rs::init; crates/media/src/media.rs::init; crates/task/src/task.rs::Task",
        "physics-navigation R1.1-R2.1 / Properties 1-2 / Task 1 excludes runtime execution; rendering-media R1.1 excludes audio server and particles implicitly",
        "task + project metadata; architecture decision required for runtime owner",
        "crates/project/src/project.rs, crates/task/src/task.rs, crates/audio/src/audio.rs, crates/media/src/media.rs",
        "cargo test -p project -p task -p audio godot_simulation",
        "Which physics, navigation, animation, audio, and particle behaviors are native Sim runtime requirements, and which are intentionally excluded?",
        (
            ("simulate 2D rigid, static, character, animatable, and soft bodies with areas, shapes, joints, layers, masks, sleeping, and callbacks", 6),
            ("simulate 3D rigid, static, character, animatable, and soft bodies with areas, shapes, joints, layers, masks, sleeping, and callbacks", 6),
            ("select Godot Physics or Jolt 3D and report backend-specific settings and unsupported behavior", 6),
            ("perform direct-space point, ray, shape, motion, contact, and rest-info queries with exclusions and limits", 6),
            ("build navigation maps from regions, meshes, links, obstacles, costs, layers, and avoidance agents in 2D and 3D", 6),
            ("bake, parse, cache, update, and debug navigation meshes and source geometry asynchronously with cancellation", 6),
            ("author and play Animation, AnimationPlayer, AnimationTree, Tween, tracks, blends, state machines, method/audio tracks, and root motion", 5),
            ("route sample playback through buses, sends, effects, capture, device switching, spatial emitters, polyphony, and interactive music", 5),
            ("simulate CPU and GPU particles, trails, collisions, attractors, process materials, subemitters, fixed FPS, and restart state", 6),
        ),
    ),
    Domain(
        "SCRIPT",
        "Scripting languages and script lifecycle",
        "GDScript, C#, expression, editor tooling",
        "Editor and exported runtime; debug/release and tool scripts differ; Mono is optional and platform-limited; language server and debug adapter are external protocol modes.",
        "Scripts load, compile, instantiate, reload, serialize, call, yield, signal, debug, and unload with owner/object lifetime; parse/build/runtime errors and unsafe editor execution are visible and bounded.",
        "projects/godot/core/object/script_language.cpp::ScriptLanguage; projects/godot/modules/gdscript/gdscript.cpp::GDScriptLanguage; projects/godot/modules/gdscript/gdscript_compiler.cpp::GDScriptCompiler; projects/godot/modules/mono/csharp_script.cpp::CSharpLanguage; projects/godot/modules/gdscript/language_server/gdscript_language_server.cpp::GDScriptLanguageServer",
        "crates/language/src/language_registry.rs::LanguageRegistry; crates/lsp/src/lsp.rs; crates/dap/src/dap.rs; crates/extension_host/src/extension_host.rs; no SimScript or .gd registration was found",
        "language-scripting R1.1-R3.3 / Properties 1-4 / Task 1; existing spec substitutes SimScript but lacks GDScript/C#/GDExtension lifecycle parity",
        "language + lsp + dap + extension_host + task",
        "crates/languages/src/lib.rs, crates/language/src/language_registry.rs, crates/lsp/src/lsp.rs, crates/dap/src/dap.rs",
        "cargo test -p language -p languages -p lsp -p dap godot",
        "Is SimScript a replacement product direction or must imported GDScript and C# projects retain executable behavior?",
        (
            ("register script languages and create, load, reload, instance, attach, detach, and free scripts with object lifetime", 5),
            ("parse and compile GDScript including typed syntax, annotations, lambdas, pattern matching, classes, inheritance, and warnings", 4),
            ("execute GDScript bytecode, calls, properties, signals, coroutines, awaits, errors, stack traces, and deterministic tests", 5),
            ("run @tool scripts in the editor with explicit trust, reload, inspector, undo, and failure isolation", 5),
            ("build, load, run, debug, hot-reload, export, and diagnose C# projects and assemblies when Mono is enabled", 5),
            ("serve GDScript completion, hover, symbols, rename, references, formatting, diagnostics, semantic tokens, and DAP debugging", 4),
            ("evaluate Expression resources with input names, base instances, parse errors, and execute failures", 5),
            ("preserve exported script properties and placeholder instances when a script is missing or invalid", 5),
            ("recognize SimScript and generate inspectable diffs from natural-language authoring intent", 3),
        ),
    ),
    Domain(
        "EXT",
        "Native extensions and editor plugins",
        "GDExtension ABI and plugin lifecycle",
        "Editor and exported runtime; platform-specific shared libraries and feature tags; initialization levels core, servers, scene, editor; tools and release builds differ.",
        "Libraries resolve, initialize, register, reload where supported, and terminate in order; ABI/version/library failures are isolated and diagnosed; plugins persist enabled state and unregister cleanly.",
        "projects/godot/core/extension/gdextension.cpp::GDExtension; projects/godot/core/extension/gdextension_manager.cpp::GDExtensionManager; projects/godot/core/extension/gdextension_interface.cpp::gdextension_setup_interface; projects/godot/editor/plugins/editor_plugin.cpp::EditorPlugin; projects/godot/editor/editor_data.cpp::EditorData::add_editor_plugin",
        "crates/extension/src/extension.rs::ExtensionStore; crates/extension_host/src/extension_host.rs::ExtensionHost; crates/extension_api/src/extension_api.rs; extensions/ supports Sim WASM extensions, not GDExtension ABI or Godot EditorPlugin contracts",
        "No dedicated Godot extension/plugin spec; language-scripting mentions legacy .gd and C# only; build-test-docs has dependency review",
        "extension + extension_host + extension_api + extensions_ui",
        "crates/extension_host/src/extension_host.rs, crates/extension_api/src/extension_api.rs, crates/extensions_ui/src/extensions_ui.rs",
        "cargo test -p extension -p extension_host -p extensions_ui godot",
        "Should Sim refuse GDExtension binaries, translate a supported subset, or provide a separately reviewed Sim-owned compatibility host without Godot libraries?",
        (
            ("parse .gdextension manifests and select libraries by OS, architecture, build, and feature tags", 5),
            ("validate GDExtension minimum version, entry symbol, ABI, interface functions, and initialization levels", 5),
            ("load and unload extension libraries while registering classes, methods, properties, signals, constants, virtuals, and singletons", 5),
            ("marshal Variants, native structures, pointers, call errors, object bindings, memory, strings, arrays, and dictionaries across the ABI", 5),
            ("generate and preserve extension_api.json and gdextension_interface.h compatibility contracts", 5),
            ("discover plugin.cfg addons and enable, disable, persist, reload, and diagnose EditorPlugin instances", 5),
            ("allow editor plugins to add docks, inspectors, importers, exporters, gizmos, debuggers, settings, shortcuts, and autoloads with cleanup", 5),
            ("reuse Sim extension trust, capability, installation, and UI boundaries instead of creating a second plugin manager", 2),
        ),
    ),
    Domain(
        "IMPORT",
        "Asset importing, caching, and dependencies",
        "editor filesystem and import pipeline",
        "Editor-only pipeline producing runtime resources; source, imported cache, remap, UID, and generated-file modes; importer and platform feature variants.",
        "Scans and imports are incremental, cancelable, dependency-aware, and persistent; moved sources, changed import settings, unavailable importers, corrupt cache, and failed subprocesses trigger visible recovery/reimport.",
        "projects/godot/editor/file_system/editor_file_system.cpp::EditorFileSystem; projects/godot/core/io/resource_importer.cpp::ResourceFormatImporter; projects/godot/editor/import/editor_import_plugin.cpp::EditorImportPlugin; projects/godot/editor/import/3d/resource_importer_scene.cpp::ResourceImporterScene",
        "crates/worktree/src/worktree.rs::Worktree observes files; crates/fs/src/fs.rs::Fs abstracts IO; image_viewer/svg_preview handle selected outputs; no .godot/imported or .import pipeline was found",
        "game-formats-assets R1.1-R3.1 / Properties 1-2 / Task 1; editor-experience R2.1-R2.2 / Task 1 covers only type/link metadata",
        "worktree + fs + project + image_viewer + svg_preview",
        "crates/worktree/src/worktree.rs, crates/project/src/project.rs, crates/image_viewer/src/image_viewer.rs",
        "cargo test -p worktree -p project -p image_viewer godot_import",
        "Which Godot import artifacts must Sim reproduce as Sim-native resources, and which importers are intentionally excluded?",
        (
            ("scan the project filesystem incrementally with ignore rules, UIDs, type detection, moves, removals, and watcher reconciliation", 4),
            ("select importers by extension and priority and persist importer, options, source, destination, remap, generator, and validity metadata", 4),
            ("queue threaded imports and reimports with progress, cancellation, restart, dependency ordering, and failure isolation", 5),
            ("invalidate imported caches from source hashes, importer versions, settings, dependencies, feature tags, and generated files", 5),
            ("import images and SVGs into textures with compression, mipmaps, color-space, normal-map, atlas, and platform variants", 5),
            ("import audio into streams/samples with compression, looping, normalization, trimming, and channel modes", 5),
            ("import 3D scenes and animations with node/path filters, materials, meshes, skins, LOD, lightmaps, physics, and post-import scripts", 5),
            ("import glTF, FBX, OBJ, Blender, DAE, and other enabled formats with dependency and unsupported-feature diagnostics", 5),
            ("import fonts, translations, CSV, bitmaps, textures, shaders, and custom plugin formats", 5),
            ("link source assets, imported outputs, generated files, resource UIDs, dependencies, owners, and reimport actions in the project panel", 4),
        ),
    ),
    Domain(
        "EXPORT",
        "Export, packaging, templates, and deployment",
        "presets, PCK, templates, platform exporters",
        "Editor UI and headless CLI; debug/release/dedicated-server templates; Android, iOS, macOS, visionOS, Linux, Windows, and Web exporters have distinct signing/toolchain/options.",
        "Presets persist filters, features, credentials references, templates, patches, encryption, and platform settings; validation blocks missing templates/toolchains/signing; cancellation and partial packages are cleaned up.",
        "projects/godot/editor/export/editor_export.cpp::EditorExport; projects/godot/editor/export/editor_export_platform.cpp::EditorExportPlatform; projects/godot/editor/export/editor_export_preset.cpp::EditorExportPreset; projects/godot/editor/export/editor_export_plugin.cpp::EditorExportPlugin; projects/godot/main/main.cpp::Main::start",
        "crates/task/src/task_template.rs::TaskTemplate and crates/terminal/src/terminal.rs can run external commands; script/bundle-* packages Sim itself; no export_presets.cfg parser was found",
        "platform-export R1.1-R2.2 / Property 1 / Task 1 covers parsing presets into external tasks but omits packaging semantics and per-platform outcomes",
        "task + terminal + project + settings",
        "crates/task/src/task.rs, crates/project/src/project.rs, crates/settings/src/settings.rs",
        "cargo test -p task -p project -p settings godot_export",
        "Which platform packager and exporter behaviors must Sim reproduce natively, and which targets are intentionally excluded?",
        (
            ("parse, edit, duplicate, reorder, persist, and validate export presets, filters, features, patches, and custom options", 4),
            ("discover, install, uninstall, mirror, and validate matching debug/release export templates without silent downloads", 5),
            ("export project data as PCK/ZIP or embedded pack with include/exclude filters, remaps, conversion, and deterministic manifests", 5),
            ("export debug, release, and dedicated-server builds from editor or CLI and propagate progress, cancellation, warnings, and errors", 4),
            ("export and deploy Android APK/AAB/Gradle builds with SDK/JDK/keystore/permissions/architectures and remote run", 5),
            ("export iOS, macOS, and visionOS bundles/projects with entitlements, privacy manifests, provisioning, codesign, notarization, and architectures", 5),
            ("export Linux/BSD and Windows executables with architectures, icons, metadata, signing, console mode, and embedded data", 5),
            ("export Web builds with WASM, threads, service worker/PWA, extensions, HTML shell, compression, and browser feature validation", 5),
            ("encrypt packs or scripts and protect credentials/signing material without persisting secrets in project files", 5),
            ("launch, stop, remote-deploy, and collect logs from an exported or editor-run project through existing Sim tasks", 4),
        ),
    ),
    Domain(
        "NET",
        "Filesystem, networking, HTTP, multiplayer, and web",
        "runtime IO and communication",
        "Editor and exported runtime; desktop/mobile/web/headless; TLS, browser sandbox, IPv4/IPv6, platform permissions, dedicated server, and peer topology variants.",
        "Connections, requests, peers, channels, replication, RPCs, downloads, and files have explicit open/close/cancel/error/timeout lifecycles; limits and path/network failures are observable.",
        "projects/godot/core/io/file_access.cpp::FileAccess; projects/godot/core/io/dir_access.cpp::DirAccess; projects/godot/core/io/http_client.cpp::HTTPClient; projects/godot/scene/main/http_request.cpp::HTTPRequest; projects/godot/modules/multiplayer/scene_multiplayer.cpp::SceneMultiplayer; projects/godot/modules/websocket/websocket_peer.cpp::WebSocketPeer",
        "crates/fs/src/fs.rs::Fs, crates/net/src/net.rs, crates/http_client/src/http_client.rs::HttpClient, crates/rpc/src/rpc.rs, and crates/collab/src/collab.rs exist for Sim application services, not Godot runtime API compatibility",
        "networking-collaboration R1.1-R2.1 / Properties 1-2 / Task 1 excludes multiplayer runtime broadly and preserves only debug metadata",
        "fs + net + http_client + rpc + collab + task",
        "crates/fs/src/fs.rs, crates/net/src/net.rs, crates/http_client/src/http_client.rs, crates/rpc/src/rpc.rs, crates/collab/src/lib.rs",
        "cargo test -p fs -p net -p http_client -p rpc godot",
        "Which Godot runtime networking APIs are native product requirements versus intentional exclusions, and may Sim collaboration ever back gameplay networking?",
        (
            ("read, write, seek, resize, flush, compress, encrypt, hash, map, and atomically replace files through res:// and user://", 5),
            ("list, create, rename, copy, remove, and watch directories while confining paths and preserving platform semantics", 5),
            ("resolve DNS and use TCP, UDP, Unix sockets, PacketPeer, StreamPeer, multicast, broadcast, IPv4, and IPv6 with nonblocking errors", 5),
            ("perform HTTP requests, redirects, proxies, cookies/headers, body streaming, downloads, timeouts, cancellation, TLS, and size limits", 2),
            ("connect WebSocket peers and multiplayer peers with protocols, channels, packet modes, close codes, heartbeats, and browser constraints", 6),
            ("connect WebRTC peers and data channels with SDP, ICE, polling, ordered/reliable modes, and platform plugin availability", 6),
            ("connect ENet peers with server/client/mesh topology, compression, bandwidth, channels, disconnects, and statistics", 6),
            ("perform high-level multiplayer RPC authority, transfer modes, object configuration, peer authentication, and refusal", 6),
            ("replicate and spawn scene state with MultiplayerSynchronizer/Spawner, visibility filters, authority changes, and late joins", 6),
            ("discover and manage UPnP mappings with timeout, gateway, conflict, and cleanup behavior", 6),
            ("bridge browser JavaScript, downloads, clipboard, virtual keyboard, service workers, and cross-origin restrictions in Web exports", 6),
        ),
    ),
    Domain(
        "DEBUG",
        "Debugger, profiler, logging, diagnostics, and crashes",
        "editor/runtime observability",
        "Editor and remote exported runtime; local/remote, debug/release, script/native, rendering/physics/network profilers, headless logs, and platform crash handlers.",
        "Sessions connect/disconnect/reconnect, pause/resume/step, collect bounded samples, flush logs, and terminate cleanly; protocol mismatch, transport loss, script/native crashes, and corrupted messages remain diagnostic.",
        "projects/godot/core/debugger/engine_debugger.cpp::EngineDebugger; projects/godot/scene/debugger/scene_debugger.cpp::SceneDebugger; projects/godot/editor/debugger/editor_debugger_node.cpp::EditorDebuggerNode; projects/godot/editor/debugger/script_editor_debugger.cpp::ScriptEditorDebugger; projects/godot/core/error/error_macros.cpp; projects/godot/platform/linuxbsd/crash_handler_linuxbsd.cpp::CrashHandler",
        "crates/dap/src/dap.rs::Client; crates/debugger_ui/src/debugger_ui.rs; crates/diagnostics/src/diagnostics.rs; crates/zlog/src/zlog.rs; crates/crashes/src/crashes.rs; crates/miniprofiler_ui/src/miniprofiler_ui.rs; reusable owners are not wired to Godot protocols",
        "editor-experience R3.1-R3.2 covers external debug launch only; networking-collaboration R2.1 mentions debug metadata; no profiler/crash protocol criteria",
        "dap + debugger_ui + diagnostics + zlog + crashes + miniprofiler_ui",
        "crates/dap/src/dap.rs, crates/debugger_ui/src/debugger_ui.rs, crates/diagnostics/src/diagnostics.rs, crates/crashes/src/crashes.rs",
        "cargo test -p dap -p debugger_ui -p diagnostics -p crashes godot",
        "Which debugger protocol semantics must Sim own directly, and which unsupported operations should remain explicit exclusions?",
        (
            ("format, route, filter, timestamp, persist, and flush stdout/stderr, print, warning, error, and structured engine log messages", 2),
            ("connect and authenticate editor/runtime debugger sessions with protocol negotiation, timeouts, reconnect, and multiple instances", 4),
            ("set breakpoints and exception breaks and inspect stacks, locals, members, globals, expressions, errors, and live script reload", 4),
            ("inspect and edit the remote scene tree, nodes, resources, properties, camera overrides, selection, and live edits safely", 5),
            ("profile script/native time, calls, frame stages, GPU, servers, memory, resources, and custom monitors with bounded sampling", 5),
            ("profile multiplayer RPC/bandwidth and visualize collisions, paths, navigation, canvas redraw, and rendering diagnostics", 5),
            ("capture errors and crashes with backtraces, symbols, platform handlers, suppression rules, and safe shutdown/reporting", 2),
            ("recover editor state after a crashed game/editor/plugin and preserve actionable logs without claiming success", 5),
        ),
    ),
    Domain(
        "CLI",
        "CLI, headless, automation, and developer workflows",
        "main process modes and tooling",
        "Editor, project manager, game, headless, dedicated server, import, export, script, benchmark, doctool, and platform-specific command options.",
        "Argument parsing selects one mode, validates conflicts and prerequisites, propagates exit codes, signals cancellation, and shuts down initialized services in reverse order; automation remains deterministic and noninteractive.",
        "projects/godot/main/main.cpp::Main::setup,Main::start,Main::cleanup and command-line option table; projects/godot/main/main_timer_sync.cpp::MainTimerSync; projects/godot/platform/linuxbsd/godot_linuxbsd.cpp::main",
        "crates/cli/src/cli.rs::CliRequest; crates/task/src/task.rs::Task; crates/terminal/src/terminal.rs; crates/remote_server/src/remote_server.rs; Sim workflows have no Godot option compatibility",
        "platform-export partially covers CLI export; editor-experience covers run/debug; build-test-docs excludes duplicate build infrastructure; no complete CLI/headless criteria",
        "cli + task + terminal + remote_server",
        "crates/cli/src/cli.rs, crates/task/src/task.rs, crates/remote_server/src/main.rs",
        "cargo test -p cli -p task -p remote_server godot",
        "Which Godot-compatible CLI flags must map to Sim-owned execution, and which are explicitly unsupported?",
        (
            ("resolve project path, main pack, scene, editor, project-manager, and runtime mode with conflict diagnostics", 4),
            ("run headless or with dummy display/audio/text/input drivers and report unsupported visual operations", 5),
            ("scan/import resources and quit after import or after a requested frame/time boundary with useful exit status", 5),
            ("export or pack named presets from CLI and propagate template, toolchain, signing, progress, cancellation, and failure status", 4),
            ("run a script or main loop, pass user arguments, select language, evaluate doctool/test modes, and exit deterministically", 5),
            ("enable remote debug, editor PID, breakpoints, profiler, GPU validation, crash handler, logging, and protocol ports", 4),
            ("select rendering/audio/display drivers, GPU, screen, window mode, resolution, locale, time scale, and frame pacing", 5),
            ("print stable help, version, path, verbose, benchmark, and build-feature diagnostics without starting a project", 2),
            ("run dedicated-server exports and automation without editor-only services or interactive prompts", 5),
        ),
    ),
    Domain(
        "SEC",
        "Authentication, permissions, sandboxing, and security",
        "trust and resource boundaries",
        "Editor and exported runtime; desktop, mobile permission models, web sandbox/CORS, XR permissions, external plugins/scripts, TLS trust, and export signing/encryption.",
        "Trust and permission decisions are explicit, least-privilege, persisted only where appropriate, revocable, and redacted; denials, invalid certificates, unsafe paths, oversized inputs, and untrusted code fail closed.",
        "projects/godot/core/io/file_access.cpp::FileAccess; projects/godot/core/io/http_client.cpp::HTTPClient; projects/godot/core/crypto/crypto.cpp::Crypto; projects/godot/core/extension/gdextension_manager.cpp::GDExtensionManager; projects/godot/platform/android/export/export_plugin.cpp::permission export options; projects/godot/platform/web/os_web.cpp::OS_Web",
        "crates/sandbox/src/sandbox.rs::Sandbox; crates/credentials_provider/src/credentials_provider.rs; crates/http_client_tls/src/http_client_tls.rs; crates/settings/src/settings.rs; crates/extension_host/src/extension_host.rs; no Godot project trust model was found",
        "root migration R11.1-R11.3 covers dependency review; build-test-docs R3.1-R3.2 covers dependencies; extension/runtime permissions and exported-project permissions are otherwise missing",
        "sandbox + credentials_provider + http_client_tls + extension_host + settings",
        "crates/sandbox/src/sandbox.rs, crates/credentials_provider/src/credentials_provider.rs, crates/extension_host/src/extension_host.rs",
        "cargo test -p sandbox -p credentials_provider -p extension_host godot_security",
        "What is the trust policy for imported @tool scripts, native extensions, editor plugins, and post-import scripts executed or translated by Sim?",
        (
            ("confine res://, user://, temp, pack, import, extension, and export paths against traversal, symlink, and archive attacks", 5),
            ("establish TLS trust from system/bundled/custom certificates and expose hostname, chain, expiry, and protocol failures", 2),
            ("request, explain, persist, revoke, and diagnose mobile camera, microphone, storage, network, notification, and XR permissions", 5),
            ("store export signing keys, passwords, tokens, and remote credentials through Sim secret facilities with redaction", 2),
            ("gate @tool scripts, post-import scripts, GDExtension libraries, and EditorPlugins by explicit project trust and isolation policy", 5),
            ("enforce browser sandbox, secure-context, cross-origin, CSP-like embedding, storage, clipboard, fullscreen, and thread prerequisites", 5),
            ("bound resource parsing, decompression, image dimensions, archive entries, recursion, network bodies, queues, and worker memory/time", 5),
            ("encrypt project data/scripts where configured and document integrity, key-management, and threat-model limitations", 5),
        ),
    ),
    Domain(
        "PERSIST",
        "Persistence, compatibility, migrations, and formats",
        "durable project/editor/runtime state",
        "Project source versus generated .godot cache versus global editor state versus user:// runtime data; versioned engine and platform variants; text/binary and debug/release.",
        "Writes are atomic or recoverable, versions are validated, migrations preserve backups/unknown data, concurrent changes are detected, and corruption/permission/disk-full errors never silently report success.",
        "projects/godot/core/io/config_file.cpp::ConfigFile; projects/godot/core/config/project_settings.cpp::ProjectSettings; projects/godot/editor/settings/editor_settings.cpp::EditorSettings; projects/godot/editor/editor_data.cpp::EditorData; projects/godot/core/io/resource_saver.cpp::ResourceSaver; projects/godot/editor/project_upgrade/project_converter_3_to_4.cpp::ProjectConverter3To4",
        "crates/settings/src/settings.rs::SettingsStore; crates/session/src/session.rs::Session; crates/workspace/src/persistence/model.rs::SessionWorkspace; crates/db/src/db.rs; crates/migrator/src/migrator.rs; reusable owners do not parse Godot formats",
        "engine-core-runtime and game-formats-assets cover partial parsing; no atomic save, editor state, user://, version migration, or compatibility contract",
        "settings + session + workspace persistence + db + migrator + fs",
        "crates/settings/src/settings.rs, crates/session/src/session.rs, crates/workspace/src/persistence/model.rs, crates/migrator/src/migrator.rs",
        "cargo test -p settings -p session -p workspace -p migrator godot_persistence",
        "Which Godot file versions must round-trip losslessly, and is 3.x-to-4.x conversion in scope?",
        (
            ("round-trip project.godot and override.cfg sections, values, feature overrides, ordering/comments policy, and unknown settings", 4),
            ("round-trip text and binary scene/resource formats with version, UID, dependency, unknown-field, and compatibility guarantees", 4),
            ("persist import metadata, file cache, UID cache, editor filesystem state, and generated artifacts without treating them as source", 4),
            ("persist global editor settings, shortcuts, favorites, templates, asset-library state, and per-version migrations", 5),
            ("persist per-project editor metadata, layouts, open scenes, folding, script breakpoints, run instances, and debugger state", 5),
            ("provide user:// ConfigFile, FileAccess, resource save, and save-game behavior across desktop/mobile/web storage", 5),
            ("perform atomic saves, backups, conflict detection, autosave, crash recovery, permission handling, and disk-full reporting", 5),
            ("convert supported legacy projects/resources/settings with dry-run diagnostics, backups, idempotence, and explicit unsupported cases", 5),
            ("publish and test a stable compatibility matrix for imported, edited, externally-run, and exported Godot versions", 5),
        ),
    ),
    Domain(
        "PLAT",
        "Platform-specific behavior",
        "desktop, mobile, web, XR",
        "Windows, macOS, Linux/BSD X11 and Wayland, Android, iOS, visionOS, Web, headless, and XR each have distinct runtime, editor, input, audio, display, filesystem, packaging, and permission roots.",
        "Platform initialization, suspend/resume, surface/device loss, permissions, app lifecycle, native integration, export, and shutdown are explicit; unavailable APIs are feature-detected and diagnostic.",
        "projects/godot/platform/windows/os_windows.cpp::OS_Windows; projects/godot/platform/macos/os_macos.mm::OS_MacOS; projects/godot/platform/linuxbsd/os_linuxbsd.cpp::OS_LinuxBSD; projects/godot/platform/android/os_android.cpp::OS_Android; projects/godot/platform/ios/os_ios.mm::OS_IOS; projects/godot/platform/visionos/os_visionos.mm::OS_VisionOS; projects/godot/platform/web/os_web.cpp::OS_Web",
        "crates/gpui_windows/src/gpui_windows.rs; crates/gpui_macos/src/gpui_macos.rs; crates/gpui_linux/src/gpui_linux.rs; crates/gpui_web/src/gpui_web.rs; script/bundle-mac; script/bundle-linux; script/bundle-windows.ps1; Android/iOS/visionOS Godot runtime equivalents are absent",
        "platform-export R1.1-R2.2 broadly delegates; xr-spatial R1.1-R2.1 excludes XR runtime; no per-platform acceptance criteria",
        "gpui_windows + gpui_macos + gpui_linux + gpui_web + task",
        "crates/gpui_platform/src/gpui_platform.rs, crates/task/src/task.rs",
        "cargo test -p gpui_platform -p task godot_platform",
        "Which target platforms must Sim support for authoring and native exported runtime, and which are intentionally excluded?",
        (
            ("run and export on Windows with native windows, input, IME, accessibility, gamepads, audio/MIDI, filesystem, registry, crash handling, signing, and D3D12/Vulkan/GLES", 4),
            ("run and export on macOS with Cocoa windows, input/IME, accessibility, Metal/Vulkan/GLES, audio/MIDI, filesystem, menus, bundles, sandbox, signing, and notarization", 4),
            ("run and export on Linux/BSD with X11 and Wayland variants, portals/DBus, input, accessibility/TTS, audio/MIDI, Vulkan/GLES, headless, packaging, and dynamic libraries", 4),
            ("run and export on Android with editor/runtime variants, lifecycle, permissions, input/sensors, accessibility, audio, Vulkan/GLES, plugins, Gradle, APK/AAB, and remote deploy", 5),
            ("run and export on iOS with lifecycle, permissions, touch/sensors, accessibility, audio, Metal, plugins, Xcode project, simulator/device, signing, and privacy manifests", 5),
            ("run and export on visionOS with spatial lifecycle, simulator/device, permissions, Metal, Xcode, signing, and OpenXR/spatial integration", 5),
            ("run and export on Web with WASM, single-thread/pthread variants, browser input/display/audio, storage, networking, JavaScript, WebXR, PWA, and secure-context limits", 5),
            ("run headless and dedicated-server builds without window/audio dependencies and with deterministic exit, signals, and resource limits", 5),
            ("run OpenXR, WebXR, and mobile VR interfaces with sessions, action maps, tracking, composition layers, spatial entities, permissions, and teardown", 6),
            ("report unsupported consoles and out-of-tree platform ports as non-baseline rather than implying coverage", 7),
        ),
    ),
    Domain(
        "QA",
        "Tests, examples, docs, localization, build tooling, and CI",
        "quality and developer infrastructure",
        "Unit/integration/compatibility/editor/platform tests; local and CI; documentation/class reference generation; translations; debug/dev/release configurations.",
        "Tests are reproducible with seeds and exit status; generated docs/build artifacts are checked; platform matrices and optional modules are explicit; flaky, skipped, unsupported, or dependency-blocked results are not reported as passes.",
        "projects/godot/tests/test_main.cpp::test_main; projects/godot/tests/compatibility_test/src/compat_checker.c::compatibility ABI checks; projects/godot/tests/compatibility_test/run_compatibility_test.py::compatibility runner; projects/godot/doc/tools/make_rst.py; projects/godot/misc/scripts; projects/godot/.github/workflows/runner.yml; projects/godot/editor/translations",
        "crates/project/tests/integration/project_tests.rs::init_test; crates/gpui/src/test.rs::TestAppContext; script/clippy; script/check-licenses; .github/workflows/run_tests.yml; no Godot fixtures or compatibility matrix was found",
        "build-test-docs R1.1-R3.2 / Task 1 covers reuse, attribution, and dependency review but not executable parity validation",
        "existing crate tests + script tooling + docs preprocessing",
        "crates/project/tests/integration/project_tests.rs, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv, .github/workflows/run_tests.yml",
        "cargo test -p project godot_compat && ./script/clippy",
        "Which upstream test fixtures and docs may be copied under MIT/third-party licensing, and which platform test matrix is required before claims of parity?",
        (
            ("run core, scene, server, module, editor, and platform unit tests with filters, tags, repeats, seeds, timing, and machine-readable exit status", 7),
            ("run resource/API compatibility tests against declared previous versions and detect removed/changed classes, methods, properties, signals, enums, and hashes", 5),
            ("exercise editor workflows, import/export fixtures, headless modes, crashes, recovery, and platform-specific behavior in integration tests", 5),
            ("generate and validate class-reference documentation from bound APIs, examples, links, inheritance, and translations", 7),
            ("provide source-backed user/developer docs for supported, divergent, decision-blocked, and excluded migration behavior", 4),
            ("preserve fixture, icon, font, sample, test-data, and converted-output attribution and license metadata", 3),
            ("build Android, iOS, Linux, macOS, Web, and Windows matrices plus static checks with explicit options and optional modules", 7),
            ("run formatting, header, documentation, API, shader, generated-file, sanitizers, warnings, licenses, and dependency checks", 7),
            ("distinguish external demo projects and tutorials from engine source capabilities and port only examples needed to verify supported behavior", 7),
        ),
    ),
    Domain(
        "MOD",
        "Optional modules and build features",
        "SCons feature composition",
        "Editor/template targets; platform/architecture-conditioned modules; modules enabled by default versus opt-in; feature build profiles and subsystem disables; builtin versus system dependencies.",
        "Configuration resolution is deterministic and reports unavailable dependencies/platforms; generated module registration and docs match enabled code; disabled modules do not leave connected UI/API placeholders.",
        "projects/godot/SConstruct::module detection,module_*_enabled options; projects/godot/modules/gdscript/config.py::can_build,is_enabled,get_opts; projects/godot/modules/modules_builders.py; projects/godot/modules/register_module_types.h",
        "Cargo.toml::workspace.members; crates/system_specs/src/system_specs.rs; crates/task/src/task.rs::Task; no Godot module-profile mapper exists",
        "build-test-docs covers dependency review only; rendering-media/physics-navigation/xr-spatial exclude families broadly; no complete module or feature-profile ledger",
        "Cargo feature owners + task diagnostics + system_specs",
        "crates/system_specs/src/system_specs.rs, crates/task/src/task.rs, Cargo.toml",
        "cargo test -p system_specs -p task godot_features",
        "Which Godot modules are mandatory native capabilities, optional native capabilities, or intentionally excluded for each Sim product profile?",
        (
            ("resolve all 55 built-in modules and custom modules by default, explicit module flags, dependencies, can_build, platform, architecture, and build profile", 5),
            ("enable GDScript and common codec/text/network/import modules by default only when their dependencies and product profile permit", 5),
            ("keep Mono/C# and fallback text server opt-in and expose build/runtime/tooling prerequisites", 5),
            ("select Godot Physics 2D/3D, Jolt, navigation, OpenXR, WebXR, mobile VR, raycast, and lightmapper modules by subsystem/platform flags", 5),
            ("select image/audio/video/texture/mesh/import codecs and builtin-versus-system third-party implementations with license and feature effects", 5),
            ("select Vulkan, GLES3, D3D12, Metal, ANGLE, AccessKit, SDL, audio, MIDI, display, and profiler drivers by build and platform", 5),
            ("apply disable_3d, advanced GUI, physics, navigation, XR, overrides, path overrides, threads, precision, deprecated, and production options consistently", 5),
            ("generate module registration, enabled defines, extension API, docs, tests, and build outputs from the same resolved feature set", 7),
        ),
    ),
    Domain(
        "UPSTREAM",
        "Third-party and upstream infrastructure",
        "vendor, generators, release engineering",
        "Build-time, test-time, editor-only, exported runtime, optional module, platform-conditioned, and vendored/system-library modes.",
        "Infrastructure affects product behavior only through the capability it supports; version/license/security/build failures remain traceable; generated outputs are not counted as implementations without connected runtime/editor consumers.",
        "projects/godot/thirdparty/README.md; projects/godot/COPYRIGHT.txt; projects/godot/SConstruct; projects/godot/methods.py; projects/godot/gles3_builders.py; projects/godot/glsl_builders.py; projects/godot/.github/workflows; projects/godot/misc/scripts",
        "script/check-licenses; script/generate-licenses; Cargo.lock; deny.toml; tooling/compliance/src/lib.rs; .github/workflows/run_tests.yml",
        "build-test-docs R1.1-R3.2 / Task 1; root R11.1-R11.3; broad dependency/reuse policy only",
        "existing dependency, license, build, CI, and docs tooling",
        "script/check-licenses, script/generate-licenses, tooling/compliance/src/lib.rs",
        "./script/check-licenses && cargo test -p compliance godot",
        "Which Godot-derived code, data, docs, fixtures, fonts, icons, or vendor patches will be copied versus behaviorally reimplemented?",
        (
            ("track vendored libraries, versions, patches, licenses, notices, security updates, and builtin/system selection that affect shipped behavior", 7),
            ("generate bindings, extension APIs, docs, shaders, fonts, icons, translations, platform templates, and registration sources reproducibly", 7),
            ("maintain SCons helpers, compiler/linker probes, caches, SCU/Ninja/compile-db support, and platform toolchain integration", 7),
            ("maintain upstream CI, packaging, signing, release, update-check, and artifact-publishing workflows separately from product parity", 7),
            ("treat imported documentation, examples, test fixtures, and generated files as evidence until a connected Sim behavior consumes them", 7),
            ("reuse Sim license, dependency, compliance, CI, documentation, and release infrastructure instead of porting Godot's equivalents", 1),
        ),
    ),
)


GODOT_COMPATIBLE_BOUNDARIES = {
    "PROJ": "Godot project.godot, override.cfg, feature tags, project identifiers, and project-directory conventions at import/export boundaries only.",
    "SCENE": "Godot .tscn, .tres, .scn, .res, UID, Variant, node/resource type, path, and signal representations at serialization boundaries only.",
    "EDITOR": "Godot-compatible command names, shortcuts, workspace concepts, inspector metadata, and project files at the Sim UI boundary only.",
    "R2D": "Godot CanvasItem, Node2D, texture, shader, material, and scene data accepted as input or emitted for interoperability; rendering executes in Sim.",
    "R3D": "Godot Node3D, mesh, material, camera, lighting, environment, shader, and scene data accepted as input or emitted for interoperability; rendering executes in Sim.",
    "UI": "Godot Control, theme, focus, layout, accessibility, and input data accepted as a compatibility model; UI layout and events execute in GPUI/Sim.",
    "INPUT": "Godot InputMap names, event encodings, display settings, locale, and platform metadata at project/file/API boundaries only.",
    "SIM": "Godot physics, navigation, animation, audio, and particle resource/property encodings at import/export boundaries only; supported simulation executes in Sim.",
    "SCRIPT": "Godot .gd/.cs sources, annotations, project metadata, diagnostics, language/debug protocol shapes, and serialized script properties at migration/tooling boundaries only.",
    "EXT": "Godot .gdextension, extension_api.json, gdextension_interface.h, plugin.cfg, and declared ABI metadata may be parsed; no Godot library or process owns execution.",
    "IMPORT": "Godot source assets, .import metadata, UID/remap data, and importer option names may be read; imported outputs are Sim-native resources and caches.",
    "EXPORT": "Godot export_presets.cfg, compatible package/resource formats, option names, and platform metadata may be read or emitted; packaging and execution are Sim-owned.",
    "NET": "Godot-compatible res:// and user:// paths, HTTP/network API shapes, RPC metadata, and serialized protocol values at explicit compatibility boundaries only.",
    "DEBUG": "Godot-compatible debug messages, source locations, breakpoints, profiler samples, and protocol payloads may be translated into Sim diagnostics/DAP models.",
    "CLI": "Approved Godot-compatible flags, exit meanings, project paths, and automation inputs may map to Sim CLI operations; commands execute inside Sim.",
    "SEC": "Godot project permission, signing, encryption, certificate, plugin, and script metadata may be imported; trust enforcement and secrets remain Sim-owned.",
    "PERSIST": "Godot project/resource/editor/import/save formats may be read or written for interoperability; authoritative state and migrations are Sim-owned.",
    "PLAT": "Godot-compatible platform option names, manifests, entitlements, permissions, package metadata, and resource formats at native Sim packaging boundaries only.",
    "QA": "Godot behavior fixtures, schemas, documentation, examples, and expected outputs are test evidence only unless licensing review approves copied material.",
    "MOD": "Godot module names, feature flags, build profiles, and availability metadata may be represented as compatibility inputs to Sim-owned capability resolution.",
    "UPSTREAM": "Godot source, generated files, third-party manifests, docs, examples, fixtures, and CI are evidence/provenance only and are not shipped or executed by default.",
}


def no_godot_validation(domain: Domain, behavior: str) -> str:
    return (
        f"{domain.validation}; run the {domain.code} scenario for {behavior} in a hermetic environment "
        "with Godot absent from PATH, loader paths, installed applications, package contents, and process allowlists; "
        "inspect the process tree and linked/runtime dependency manifests and assert that no Godot executable, "
        "shared library, server, command-line tool, or hidden instance is discovered, loaded, or invoked."
    )


def native_storage_path(domain: Domain, classification: int) -> str:
    if classification == 7:
        return f"No product storage is introduced; provenance remains in Sim audit/compliance owners. Relevant existing or proposed owner paths: {domain.writes}."
    if classification == 6:
        return f"No runtime state until a native architecture is approved; imported metadata, exclusions, and diagnostics remain Sim-owned at {domain.writes}."
    return f"Sim-owned records, resources, caches, settings, or artifacts persist through the existing owner at {domain.writes}; Godot files are boundary inputs or outputs, never authoritative runtime state."


def native_execution_path(domain: Domain, classification: int) -> str:
    if classification == 7:
        return f"No direct product execution path; existing Sim build, compliance, test, and release owners process evidence without executing Godot. Owner: {domain.owner}."
    if classification == 6:
        return f"No execution path is claimed. The capability stays excluded or decision-blocked until `{domain.owner}` can own native execution without Godot."
    return f"`{domain.owner}` owns execution through existing Sim services and registries; no Godot process, API wrapper, engine server, or shared library may execute the behavior."


def native_ui_path(domain: Domain, classification: int) -> str:
    if classification == 7:
        return "No direct user-facing UI; supported downstream behavior and provenance use existing Sim diagnostics, documentation, settings, and compliance surfaces."
    if classification == 6:
        return f"Existing Sim diagnostics owned by `{domain.owner}` expose the explicit exclusion or unresolved decision; no launch-Godot affordance counts as support."
    return f"Existing Sim UI, command, task, preview, settings, or diagnostic surfaces owned by `{domain.owner}` present the behavior and failures; no hidden Godot UI is used."


def native_lifecycle_path(domain: Domain, classification: int) -> str:
    if classification == 7:
        return "Sim build/test/compliance jobs own evidence ingestion, validation, cancellation, failure reporting, retention, and cleanup."
    if classification == 6:
        return "The Sim specification and diagnostic lifecycle owns unresolved/excluded state; no Godot instance is started and no external lifecycle is delegated."
    return f"`{domain.owner}` owns create/open/start/update/cancel/recover/persist/close behavior and cleanup using Sim entity, task, session, resource, or platform lifecycles."


def strategy(classification: int, owner: str) -> str:
    if classification == 1:
        return f"Reuse {owner} unchanged; add only evidence and regression coverage."
    if classification == 2:
        return f"Extend {owner} at its existing integration points; do not fork parallel project, UI, network, security, or persistence services."
    if classification == 3:
        return f"Execute the existing owner spec through {owner}; verify implementation before changing the design."
    if classification == 4:
        return f"Extend the named owner spec and {owner} with capability-specific success, failure, persistence, lifecycle, and platform criteria."
    if classification == 5:
        return f"Add capability-specific requirements and implement through {owner}; no new crate unless an architecture decision proves these owners insufficient."
    if classification == 6:
        return "Preserve the documented exclusion as unresolved or intentionally excluded; external Godot execution is prohibited and a native owner requires product/architecture review."
    return "Do not port this infrastructure directly; reuse Sim tooling and trace only the externally observable behavior it enables."


def gap(classification: int, behavior: str) -> str:
    if classification == 1:
        return f"Godot-specific verification that existing Sim behavior satisfies: {behavior}."
    if classification == 2:
        return f"Godot semantics, format mapping, and platform/error coverage for: {behavior}."
    if classification == 3:
        return f"Implementation evidence and validation for the fully specified behavior: {behavior}."
    if classification == 4:
        return f"Leaf acceptance criteria and validation do not yet cover all observable outcomes for: {behavior}."
    if classification == 5:
        return f"No baseline requirement/design/leaf task completely owns: {behavior}."
    if classification == 6:
        return f"No Sim-native behavior replaces the excluded capability: {behavior}; native scope or intentional exclusion remains an unresolved product/architecture decision."
    return f"Only provenance and supported-behavior linkage are needed for: {behavior}."


def rows():
    task_id = 2
    for requirement_id, domain in enumerate(DOMAINS, start=2):
        for number, (behavior, classification) in enumerate(domain.capabilities, start=1):
            capability_id = f"GODOT-{domain.code}-{number:03d}"
            yield {
                "capability_id": capability_id,
                "domain": domain.name,
                "subdomain": domain.subdomain,
                "observable_behavior": behavior,
                "supported_modes_and_platform_differences": domain.modes,
                "success_failure_persistence_lifecycle": domain.lifecycle,
                "godot_evidence": domain.godot_evidence,
                "existing_sim_evidence": domain.sim_evidence,
                "spec_coverage": f"Baseline: {domain.baseline_spec}. Audit closure: R{requirement_id}.1-R{requirement_id}.4; D-{domain.code}; T{task_id}. Native gate: R23.1-R23.10; D-NATIVE; T200.",
                "classification": CLASSIFICATIONS[classification],
                "proposed_owner_in_sim": domain.owner,
                "existing_or_proposed_native_sim_owner": domain.owner,
                "build_time_dependency_on_godot": "No. Godot source, generators, libraries, executables, servers, and command-line tools are prohibited build dependencies unless separately approved compatibility tooling is isolated from shipped artifacts.",
                "runtime_dependency_on_godot": "No. The shipped Sim editor and exported runtime must not embed, bundle, invoke, link, wrap, or communicate with any Godot runtime component.",
                "sim_native_storage_path": native_storage_path(domain, classification),
                "sim_native_execution_path": native_execution_path(domain, classification),
                "sim_native_ui_path": native_ui_path(domain, classification),
                "sim_native_lifecycle_path": native_lifecycle_path(domain, classification),
                "godot_compatible_file_or_api_boundary": GODOT_COMPATIBLE_BOUNDARIES[domain.code],
                "existing_sim_reuse_evidence": domain.sim_evidence,
                "reuse_or_extension_strategy": strategy(classification, domain.owner),
                "remaining_gap": gap(classification, behavior),
                "verification_needed": f"{domain.validation}; scenario must prove {behavior}, including the domain failure/lifecycle contract and native Sim ownership.",
                "no_godot_installation_validation": no_godot_validation(domain, behavior),
                "confidence": "High" if classification in {1, 6, 7} else "Medium",
                "open_questions": domain.question,
                "_classification_number": classification,
                "_requirement_id": requirement_id,
                "_task_id": task_id,
                "_domain_code": domain.code,
            }
            task_id += 1


def write_catalog(all_rows):
    columns = [key for key in all_rows[0] if not key.startswith("_")]
    with CATALOG.open("w", encoding="utf-8", newline="") as file:
        writer = csv.DictWriter(file, fieldnames=columns)
        writer.writeheader()
        writer.writerows({key: row[key] for key in columns} for row in all_rows)


def write_requirements():
    lines = [
        "# Requirements: Godot Full Port Coverage",
        "",
        "## Problem",
        "",
        "The existing Godot migration pack groups broad areas but does not prove source-complete capability coverage, connected native Sim implementation, or leaf-level traceability. This audit freezes the source baseline and makes every independently observable capability reviewable without treating checked tasks, placeholders, external Godot delegation, dependencies, or blanket exclusions as parity.",
        "",
        "## Scope",
        "",
        "### In scope",
        "",
        "- Godot 4.7-stable editor, runtime, format, build, platform, optional-module, test, documentation, and infrastructure behavior present in the frozen source snapshot.",
        "- Existing Sim implementation and every specification under `.agents/specs/godot-migration/`.",
        "- Source-backed classification, native Sim ownership, anti-duplication, no-Godot dependency validation, platform/failure/lifecycle coverage, and implementation planning.",
        "",
        "### Out of scope",
        "",
        "- Product implementation, dependency installation, external mutation, commits, pushes, and pull requests.",
        "- Choosing unresolved native product scope, compatibility floors, source-copy licensing policy, or materially different native architecture directions.",
        "",
        "## Requirements",
        "",
        "### Requirement 1: Reproducible audit baseline and catalog",
        "",
        "**User story:** As a reviewer, I want the audit bound to reproducible source revisions and exhaustive catalog fields so that coverage claims can be independently checked.",
        "",
        "#### Acceptance criteria",
        "",
        "1. **1.1** THE audit SHALL record the Sim commit, Godot commit, content-manifest fingerprint, source-file count, source version, working-tree state, submodule state, build targets, feature options, modules, platform roots, and CI roots.",
        "2. **1.2** THE catalog SHALL assign every independently observable capability one stable `GODOT-<DOMAIN>-<NUMBER>` ID and exactly one of the seven requested classifications.",
        "3. **1.3** THE catalog SHALL record every field requested by the audit, including exact source, Sim, requirement, design, task, validation, confidence, decision, native owner, Godot build/runtime dependency, Sim storage/execution/UI/lifecycle path, Godot-compatible boundary, reuse evidence, and no-Godot-installation evidence.",
        "4. **1.4** THE summary SHALL reconcile every catalog row by domain and classification and state the coverage denominator and formula.",
        "5. **1.5** IF evidence is absent or a product, compatibility, licensing, or architecture choice is unresolved, THEN THE audit SHALL record uncertainty and SHALL NOT assume parity or choose a direction.",
        "",
    ]
    for requirement_id, domain in enumerate(DOMAINS, start=2):
        lines.extend(
            [
                f"### Requirement {requirement_id}: {domain.name}",
                "",
                f"**User story:** As a Godot project owner, I want migration coverage for {domain.subdomain} so that observable behavior is implemented natively in Sim, intentionally excluded, identified as upstream-only, or blocked on an explicit decision without duplicate Sim infrastructure.",
                "",
                "#### Acceptance criteria",
                "",
                f"1. **{requirement_id}.1** WHEN a cataloged {domain.name.lower()} capability succeeds, THEN THE selected Sim owner SHALL provide the cataloged observable result in every declared supported mode.",
                f"2. **{requirement_id}.2** IF an input, dependency, permission, resource, platform, configuration, cancellation, persistence, or lifecycle transition fails, THEN THE selected Sim owner SHALL provide the cataloged failure, recovery, and state-preservation behavior.",
                f"3. **{requirement_id}.3** IF existing Sim functionality owns any part of the capability, THEN THE migration SHALL extend or reuse `{domain.owner}` and SHALL NOT add a parallel implementation.",
                f"4. **{requirement_id}.4** WHEN parity is claimed, THEN verification SHALL exercise every `{domain.code}` catalog row against the frozen Godot evidence and connected Sim behavior; documentation, stubs, types, disabled code, and unchecked tasks alone SHALL NOT count.",
                "",
            ]
        )
    lines.extend(
        [
            "### Requirement 23: Native Sim implementation gate",
            "",
            "**User story:** As a product and distribution owner, I want every supported Godot-origin capability to be owned and executed by Sim so that shipped products remain independent of a Godot installation or runtime dependency.",
            "",
            "#### Acceptance criteria",
            "",
            "1. **23.1** WHEN a Godot-origin capability is implemented or classified as fully specified, THEN its storage, execution, UI, persistence, cancellation, recovery, and lifecycle paths SHALL be owned by named existing or proposed Sim components.",
            "2. **23.2** THE migration SHALL NOT embed, bundle, invoke, launch, link against, wrap, proxy, communicate with, or depend at build time or runtime on the Godot editor, engine, executable, shared library, server, command-line tool, or hidden Godot instance.",
            "3. **23.3** WHEN Godot-compatible projects, scenes, resources, scripts, APIs, imports, or export settings cross a compatibility boundary, THEN Sim SHALL parse or emit the boundary representation while storing and executing supported behavior as Sim-native data, resources, scenes, artifacts, and runtime state.",
            "4. **23.4** WHEN an existing Sim component owns an adjacent responsibility, THEN the migration SHALL extend that owner and SHALL NOT create a parallel Godot-specific crate, registry, manager, runtime, task source, UI subsystem, persistence layer, network stack, plugin host, renderer, or platform service.",
            "5. **23.5** IF source code, generated code, vendor patches, libraries, bindings, fixtures, assets, or documentation would be copied from Godot, THEN work SHALL remain blocked until a separate licensing and architecture review approves the exact material, license obligations, provenance, maintenance, linkage, and distribution effects.",
            "6. **23.6** WHEN an importer or migration tool reads Godot formats, THEN every successful output SHALL be a Sim-native record, resource, scene, artifact, cache entry, or runtime state, and failure/cancellation SHALL not require Godot to recover.",
            "7. **23.7** WHEN an exported project is validated, THEN it SHALL execute on a machine without Godot installed and its package, process tree, loader resolution, network connections, and runtime dependency manifest SHALL contain no Godot editor, engine, executable, shared library, server, or command-line tool.",
            "8. **23.8** IF a capability cannot currently be implemented through a native Sim owner, THEN it SHALL be classified as unresolved, intentionally excluded with rationale, or requiring a material product/architecture decision and SHALL NOT be treated as covered by a wrapper, interface, placeholder, format declaration, task template, or external delegation.",
            "9. **23.9** WHEN a capability is classified as already implemented or fully specified, THEN its acceptance criteria, leaf task validation, and implementation evidence SHALL prove Sim-owned execution with Godot absent; otherwise the classification SHALL be partial, missing, excluded, upstream-only, or decision-blocked as applicable.",
            "10. **23.10** WHEN this audit is validated, THEN it SHALL report every plan that embeds, wraps, invokes, vendors, links, or delegates to Godot; every Godot-specific abstraction duplicating a Sim owner; every placeholder-only support claim; every license/linkage dependency; and every material native architecture decision without silently selecting a direction.",
            "",
            "## Constraints",
            "",
            "- Classification records the pre-audit baseline. Newly added closure traceability does not rewrite historical classification.",
            "- Editor behavior and exported-runtime behavior remain distinct.",
            "- Mandatory behavior, optional modules, examples, tests, vendor code, and release infrastructure remain distinct.",
            "- Every implementation task remains unchecked until separately implemented and verified.",
            "",
            "## Open questions",
            "",
            "See `decisions.md`; those decisions intentionally block native runtime scope, compatibility floors, platform tiers, plugin trust, and source reuse/licensing choices. External Godot execution is not an available architecture direction.",
        ]
    )
    (ROOT / "requirements.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_design():
    lines = [
        "# Design: Godot Full Port Coverage",
        "",
        "## Overview",
        "",
        "The audit uses a frozen source manifest, one master capability catalog, and domain owners that point into existing Sim crates. The catalog is the granularity authority; requirements define cross-cutting success/failure/reuse/evidence and native-ownership contracts, design elements bind each domain to its smallest existing owners, and one leaf task per capability prevents broad labels, wrappers, or Godot delegation from hiding materially different behavior.",
        "",
        "## Existing context",
        "",
        "The inspected Sim revision has mature project, worktree, editor, GPUI, language/LSP/DAP, task, filesystem, HTTP/RPC, collaboration, diagnostics, settings, persistence, sandbox, extension, media, and platform crates. It has no `sim_game` or `world_model` workspace members and no connected `project.godot`, `.tscn`, `.tres`, GDScript, GDExtension, Godot import, or Godot export implementation. Existing checked migration tasks are therefore planning history, not implementation evidence.",
        "",
        "## Design decisions",
        "",
        "### D-BASELINE: Frozen evidence and historical classification",
        "",
        "- Responsibility: Reproduce the exact Godot and Sim sources, preserve baseline classification, and generate reconciled counts.",
        "- Integration: `baseline.md`, `catalogs/master-coverage.csv`, `coverage-summary.md`, and `verify_snapshot.py`.",
        "- Rationale: A source snapshot without nested Git metadata must be verified against the official tag rather than assigned an inferred SHA.",
        "",
        "### D-NATIVE: Native Sim ownership and no-Godot dependency gate",
        "",
        "- Responsibility: Require every supported Godot-origin capability to name the existing or proposed Sim owner, Sim-native storage/execution/UI/lifecycle path, compatibility boundary, reuse evidence, build/runtime dependency status, and a validation that runs with Godot absent.",
        "- Integration: Existing Sim owners named by each domain, `catalogs/master-coverage.csv`, `findings.md`, `decisions.md`, `validate_audit.py`, owner-spec acceptance criteria, and leaf-task validation metadata.",
        "- Runtime boundary: Godot is a behavioral and format reference only. Sim must not embed, bundle, invoke, launch, link, wrap, proxy, or communicate with Godot. Imported formats terminate at Sim-native records/resources; exports package Sim-owned execution.",
        "- Source boundary: Godot source, generated code, vendor patches, fixtures, assets, and docs remain evidence unless exact copying is separately approved after licensing and architecture review.",
        "- Classification gate: Classifications 1 and 3 require acceptance criteria and connected evidence proving Sim-owned execution in a hermetic no-Godot environment. A wrapper, task template, file declaration, type, interface, stub, placeholder, disabled path, or external delegation cannot satisfy the gate.",
        "- Decision handling: Capabilities without a viable native owner remain unresolved, intentionally excluded, upstream-only, or decision-blocked; the audit never selects between materially different native product/architecture directions.",
        "",
    ]
    for domain in DOMAINS:
        lines.extend(
            [
                f"### D-{domain.code}: {domain.name} ownership",
                "",
                f"- Responsibility: Own the `{domain.code}` catalog rows for {domain.subdomain}, including success, failure, persistence, lifecycle, mode, and platform outcomes.",
                f"- Integration: Reuse or extend `{domain.owner}`. Proposed focused writes are `{domain.writes}`.",
                f"- Rationale: {strategy(2, domain.owner)}",
                "",
            ]
        )
    lines.extend(
        [
            "## Requirements traceability",
            "",
            "| Requirement | Design element | Verification |",
            "| --- | --- | --- |",
            "| 1.1 | D-BASELINE | Recompute commit, manifest, build/module/platform inventory |",
            "| 1.2 | D-BASELINE | Validate IDs and seven-value classification enum |",
            "| 1.3 | D-BASELINE | Validate required nonempty catalog columns and exact trace IDs |",
            "| 1.4 | D-BASELINE | Reconcile generated summary counts to CSV rows |",
            "| 1.5 | D-BASELINE | Review uncertainty and decision registers for silent choices |",
        ]
    )
    for requirement_id, domain in enumerate(DOMAINS, start=2):
        for acceptance in range(1, 5):
            verification = domain.validation if acceptance == 4 else f"Catalog scenarios for every {domain.code} row"
            lines.append(f"| {requirement_id}.{acceptance} | D-{domain.code} | {verification} |")
    for acceptance in range(1, 11):
        lines.append(f"| 23.{acceptance} | D-NATIVE | Catalog native-owner/dependency/path/boundary fields, violation report, and hermetic no-Godot validation |")
    lines.extend(
        [
            "",
            "## Error handling and recovery",
            "",
            "A missing path, symbol, criterion, design element, task, classification, native owner, Godot dependency declaration, Sim-native path, compatibility boundary, reuse evidence, no-Godot validation, confidence, or question is a catalog error. A source or architecture uncertainty is not converted into an implementation claim: it remains an open decision with dependent capability tasks blocked by review. Any proposed Godot process, library, server, command, wrapper, proxy, hidden instance, or shipped vendor/linkage dependency is a blocking native-ownership violation.",
            "",
            "## Testing strategy",
            "",
            "- Recompute the local Godot Git blob manifest and compare it with the official tag tree.",
            "- Validate CSV schema, IDs, enum values, nonempty fields, unique capabilities, counts, and requirement/design/task references.",
            "- Validate that every catalog row declares no Godot build/runtime dependency and records Sim-native storage, execution, UI, lifecycle, compatibility boundary, reuse evidence, and a hermetic no-Godot scenario.",
            "- Scan all migration specs for plans that invoke, embed, wrap, link, vendor, or delegate execution to Godot and for Godot-specific abstractions that duplicate existing Sim owners.",
            "- Run the feature-spec validator for this pack and every modified migration pack.",
            "- During implementation, execute each catalog row's focused command plus scenario tests for success, failure, persistence, lifecycle, permissions, limits, cancellation, relevant platforms, package contents, linked dependencies, process trees, and operation with Godot absent.",
        ]
    )
    (ROOT / "design.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def write_tasks(all_rows):
    lines = [
        "# Implementation Plan: Godot Full Port Coverage",
        "",
        "## Approach",
        "",
        "Keep the audit baseline reproducible, enforce native Sim ownership, resolve blocking product decisions, then execute one independently reviewable capability task at the existing Sim owner. Task order is catalog order only; implementation may be regrouped into dependency waves after the decisions in `decisions.md` are approved and write conflicts are reviewed. Every task is intentionally unchecked.",
        "",
        "## Tasks",
        "",
        "- [ ] 1. Maintain the frozen audit baseline and master reconciliation",
        "  - Recompute source revision/content evidence, catalog schema, classifications, native ownership/dependency paths, traceability, counts, overclaims, duplicates, contradictions, and decision registers without promoting plans, wrappers, delegation, or documentation to implementation evidence.",
        "  - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_",
        "  - _Depends on: none_",
        "  - _Reads: projects/godot/version.py, projects/godot/SConstruct, projects/godot/modules/*/config.py, projects/godot/platform/*/detect.py, Cargo.toml, .agents/specs/godot-migration/**_",
        "  - _Writes: .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv, .agents/specs/godot-migration/godot-full-port-coverage/coverage-summary.md, .agents/specs/godot-migration/godot-full-port-coverage/baseline.md_",
        "  - _Validation: python3 .agents/specs/godot-migration/godot-full-port-coverage/validate_audit.py_",
        "",
    ]
    domain_by_code = {domain.code: domain for domain in DOMAINS}
    previous_task_by_domain = {}
    for row in all_rows:
        domain = domain_by_code[row["_domain_code"]]
        reads = f"{domain.godot_evidence.split(';')[0].split('::')[0]}, {domain.sim_evidence.split(';')[0].split('::')[0]}"
        previous_dependency = previous_task_by_domain.get(row["_domain_code"])
        dependency = f"{previous_dependency}, 200" if previous_dependency is not None else "1, 200"
        writes = ", ".join(
            f"{path.strip()}#{row['capability_id']}"
            for path in domain.writes.split(",")
        )
        lines.extend(
            [
                f"- [ ] {row['_task_id']}. Close or verify {row['capability_id']}: {row['observable_behavior']}",
                f"  - Apply the cataloged native Sim owner, reuse strategy, modes, failure/recovery, storage, execution, UI, persistence, lifecycle, security, limit, compatibility boundary, and platform contract. Classification remains historical; implementation may claim completion only with connected Sim behavior and hermetic no-Godot evidence.",
                f"  - _Requirements: {row['_requirement_id']}.1, {row['_requirement_id']}.2, {row['_requirement_id']}.3, {row['_requirement_id']}.4, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_",
                f"  - _Depends on: {dependency}_",
                f"  - _Reads: {reads}, .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv_",
                f"  - _Writes: {writes}_",
                f"  - _Validation: {row['no_godot_installation_validation']}_",
                "",
            ]
        )
        previous_task_by_domain[row["_domain_code"]] = row["_task_id"]
    lines.extend(
        [
            "- [ ] 200. Enforce the native Sim implementation gate across the Godot migration",
            "  - Audit every migration requirement, design, task, dependency proposal, and catalog row for embedding, bundling, invocation, linkage, wrappers, hidden instances, external delegation, source copying, duplicate Godot-specific owners, placeholder-only support, and missing no-Godot validation. Keep material product, compatibility, licensing, and architecture choices in `decisions.md`.",
            "  - _Requirements: 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 23.7, 23.8, 23.9, 23.10_",
            "  - _Depends on: 1_",
            "  - _Reads: .agents/specs/godot-migration/**, Cargo.toml, Cargo.lock, deny.toml, projects/godot/COPYRIGHT.txt, projects/godot/thirdparty/README.md_",
            "  - _Writes: .agents/specs/godot-migration/godot-full-port-coverage/catalogs/master-coverage.csv, .agents/specs/godot-migration/godot-full-port-coverage/findings.md, .agents/specs/godot-migration/godot-full-port-coverage/decisions.md, .agents/specs/godot-migration/godot-full-port-coverage/validation-results.md, .agents/specs/godot-migration/**/requirements.md, .agents/specs/godot-migration/**/design.md, .agents/specs/godot-migration/**/tasks.md_",
            "  - _Validation: python3 .agents/specs/godot-migration/godot-full-port-coverage/validate_audit.py; validate all feature-spec packs; assert zero checked tasks; assert supported/fully specified rows include hermetic execution and package/link/process inspection with Godot absent_",
            "",
        ]
    )
    (ROOT / "tasks.md").write_text("\n".join(lines), encoding="utf-8")


def write_summary(all_rows):
    counts = defaultdict(Counter)
    overall = Counter()
    missing_partial = []
    for row in all_rows:
        number = row["_classification_number"]
        counts[row["domain"]][number] += 1
        overall[number] += 1
        if number in {2, 4, 5}:
            missing_partial.append(row)
    denominator = len(all_rows)
    covered = overall[1] + overall[3] + overall[6] + overall[7]
    estimated = covered / denominator * 100
    lines = [
        "# Godot migration coverage summary",
        "",
        "## Denominator and estimate",
        "",
        f"The denominator is **{denominator} independently observable capability families** in `catalogs/master-coverage.csv`. Baseline estimated disposition coverage is **{covered}/{denominator} ({estimated:.1f}%)**, counting classifications 1, 3, 6, and 7 as fully dispositioned only because the updated acceptance criteria make native Sim ownership and hermetic no-Godot validation mandatory gates. Classifications 2, 4, and 5 remain partial or missing. This is specification/disposition coverage, not implemented runtime parity; any class 1 or 3 row must be downgraded if its connected implementation or leaf validation cannot pass without Godot installed.",
        "",
        "## Counts by domain and classification",
        "",
        "| Domain | 1 | 2 | 3 | 4 | 5 | 6 | 7 | Total |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for domain in DOMAINS:
        values = counts[domain.name]
        total = sum(values.values())
        lines.append(f"| {domain.name} | " + " | ".join(str(values[index]) for index in range(1, 8)) + f" | {total} |")
    lines.append("| **Overall** | " + " | ".join(str(overall[index]) for index in range(1, 8)) + f" | **{denominator}** |")
    lines.extend(
        [
            "",
            "Classification legend: 1 implemented/reusable; 2 partial Sim implementation; 3 fully covered existing spec; 4 partially covered existing spec; 5 missing baseline spec; 6 intentionally excluded; 7 upstream-only infrastructure.",
            "",
            "## Native Sim implementation gate",
            "",
            "Every row records a native Sim owner, zero permitted Godot build/runtime dependency, Sim-native storage/execution/UI/lifecycle path, Godot-compatible boundary, existing Sim reuse evidence, and a hermetic no-Godot-installation validation. External Godot execution, API wrapping, hidden instances, runtime linkage, and unreviewed source copying are not coverage strategies. Capabilities without an approved native owner remain partial, missing, intentionally excluded, upstream-only, or decision-blocked.",
            "",
            "## Missing and partially covered capabilities",
            "",
        ]
    )
    for row in missing_partial:
        lines.append(f"- `{row['capability_id']}` — {row['classification']}: {row['observable_behavior']}")
    (ROOT / "coverage-summary.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def main():
    all_rows = list(rows())
    CATALOG.parent.mkdir(parents=True, exist_ok=True)
    write_catalog(all_rows)
    write_requirements()
    write_design()
    write_tasks(all_rows)
    write_summary(all_rows)
    print(f"Generated {len(all_rows)} Godot capability rows")


if __name__ == "__main__":
    main()
