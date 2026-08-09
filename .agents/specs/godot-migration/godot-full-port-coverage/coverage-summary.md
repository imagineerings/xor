# Godot migration coverage summary

## Denominator and estimate

The denominator is **198 independently observable capability families** in `catalogs/master-coverage.csv`. Baseline estimated disposition coverage is **55/198 (27.8%)**, counting classifications 1, 3, 6, and 7 as fully dispositioned only because the updated acceptance criteria make native Sim ownership and hermetic no-Godot validation mandatory gates. Classifications 2, 4, and 5 remain partial or missing. This is specification/disposition coverage, not implemented runtime parity; any class 1 or 3 row must be downgraded if its connected implementation or leaf validation cannot pass without Godot installed.

## Counts by domain and classification

| Domain | 1 | 2 | 3 | 4 | 5 | 6 | 7 | Total |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Project manager and lifecycle | 0 | 2 | 0 | 2 | 7 | 0 | 0 | 11 |
| Scene, node, resource, and serialization | 0 | 0 | 0 | 4 | 4 | 4 | 0 | 12 |
| Editor workspace and authoring surfaces | 2 | 5 | 0 | 4 | 3 | 0 | 0 | 14 |
| 2D rendering | 0 | 0 | 1 | 0 | 0 | 8 | 0 | 9 |
| 3D rendering | 0 | 0 | 0 | 1 | 0 | 10 | 0 | 11 |
| UI/control framework and themes | 0 | 7 | 0 | 0 | 1 | 0 | 0 | 8 |
| Input, windowing, display, accessibility, and internationalization | 0 | 3 | 0 | 0 | 6 | 0 | 0 | 9 |
| Physics, navigation, animation, audio, and particles | 0 | 0 | 0 | 0 | 2 | 7 | 0 | 9 |
| Scripting languages and script lifecycle | 0 | 0 | 1 | 2 | 6 | 0 | 0 | 9 |
| Native extensions and editor plugins | 0 | 1 | 0 | 0 | 7 | 0 | 0 | 8 |
| Asset importing, caching, and dependencies | 0 | 0 | 0 | 3 | 7 | 0 | 0 | 10 |
| Export, packaging, templates, and deployment | 0 | 0 | 0 | 3 | 7 | 0 | 0 | 10 |
| Filesystem, networking, HTTP, multiplayer, and web | 0 | 1 | 0 | 0 | 3 | 7 | 0 | 11 |
| Debugger, profiler, logging, diagnostics, and crashes | 0 | 2 | 0 | 2 | 4 | 0 | 0 | 8 |
| CLI, headless, automation, and developer workflows | 0 | 1 | 0 | 3 | 5 | 0 | 0 | 9 |
| Authentication, permissions, sandboxing, and security | 0 | 2 | 0 | 0 | 6 | 0 | 0 | 8 |
| Persistence, compatibility, migrations, and formats | 0 | 0 | 0 | 3 | 6 | 0 | 0 | 9 |
| Platform-specific behavior | 0 | 0 | 0 | 3 | 5 | 1 | 1 | 10 |
| Tests, examples, docs, localization, build tooling, and CI | 0 | 0 | 1 | 1 | 2 | 0 | 5 | 9 |
| Optional modules and build features | 0 | 0 | 0 | 0 | 7 | 0 | 1 | 8 |
| Third-party and upstream infrastructure | 1 | 0 | 0 | 0 | 0 | 0 | 5 | 6 |
| **Overall** | 3 | 24 | 3 | 31 | 88 | 37 | 12 | **198** |

Classification legend: 1 implemented/reusable; 2 partial Sim implementation; 3 fully covered existing spec; 4 partially covered existing spec; 5 missing baseline spec; 6 intentionally excluded; 7 upstream-only infrastructure.

## Native Sim implementation gate

Every row records a native Sim owner, zero permitted Godot build/runtime dependency, Sim-native storage/execution/UI/lifecycle path, Godot-compatible boundary, existing Sim reuse evidence, and a hermetic no-Godot-installation validation. External Godot execution, API wrapping, hidden instances, runtime linkage, and unreviewed source copying are not coverage strategies. Capabilities without an approved native owner remain partial, missing, intentionally excluded, upstream-only, or decision-blocked.

## Missing and partially covered capabilities

- `GODOT-PROJ-001` — Missing from the migration specs: create a project with name, path, renderer, version-control metadata, and default files
- `GODOT-PROJ-002` — Partially covered by an existing migration spec: import an existing project.godot and reject invalid or duplicate roots without losing user data
- `GODOT-PROJ-003` — Partially implemented in Sim and should be extended: scan, sort, filter, favorite, rename, remove, and reopen projects in the project manager
- `GODOT-PROJ-004` — Partially implemented in Sim and should be extended: persist recent projects, favorites, tags, sort mode, and missing-project state
- `GODOT-PROJ-005` — Partially covered by an existing migration spec: parse project features, application metadata, main scene, autoloads, input map, and rendering settings
- `GODOT-PROJ-006` — Missing from the migration specs: start the editor, project manager, or game based on project discovery and command-line mode
- `GODOT-PROJ-007` — Missing from the migration specs: detect incompatible engine versions and offer project conversion or manager-assisted upgrade
- `GODOT-PROJ-008` — Missing from the migration specs: open in safe mode after editor/plugin failure and recover unsaved scene state
- `GODOT-PROJ-009` — Missing from the migration specs: install and instantiate project templates while surfacing download and extraction failures
- `GODOT-PROJ-010` — Missing from the migration specs: use per-project .godot data and cache roots without treating generated metadata as source
- `GODOT-PROJ-011` — Missing from the migration specs: apply project settings overrides and feature-tag-specific overrides with deterministic precedence
- `GODOT-SCENE-004` — Partially covered by an existing migration spec: pack, instantiate, inherit, edit, save, reload, and revert scenes with editable children and ownership
- `GODOT-SCENE-005` — Partially covered by an existing migration spec: load, preload, cache, duplicate, localize, reference-count, and release resources
- `GODOT-SCENE-006` — Partially covered by an existing migration spec: round-trip .tscn and .tres values, subresources, ext_resources, scripts, and connection records
- `GODOT-SCENE-007` — Missing from the migration specs: round-trip binary .scn and .res resources with version and endianness compatibility
- `GODOT-SCENE-008` — Missing from the migration specs: assign stable resource UIDs and repair moved dependency paths without corrupting references
- `GODOT-SCENE-009` — Partially covered by an existing migration spec: enumerate dependencies and surface missing, cyclic, corrupt, or type-mismatched resources
- `GODOT-SCENE-010` — Missing from the migration specs: serialize Variant values, exported properties, dictionaries, arrays, typed containers, and object references
- `GODOT-SCENE-012` — Missing from the migration specs: preserve unknown or newer-format data sufficiently for non-destructive migration
- `GODOT-EDITOR-001` — Partially implemented in Sim and should be extended: restore open scenes, selected objects, bottom panels, docks, and workspace layout per project
- `GODOT-EDITOR-002` — Partially covered by an existing migration spec: browse and manipulate the scene tree with create, rename, reparent, group, visibility, and ownership operations
- `GODOT-EDITOR-003` — Partially covered by an existing migration spec: browse project files with type filters, favorites, move/rename dependency repair, and reimport state
- `GODOT-EDITOR-004` — Partially covered by an existing migration spec: inspect and edit grouped, typed, ranged, resource, node-path, and script-exposed properties
- `GODOT-EDITOR-005` — Missing from the migration specs: edit scenes through dedicated 2D, 3D, script, asset-library, and game workspaces
- `GODOT-EDITOR-007` — Partially implemented in Sim and should be extended: configure and resolve user, project, feature-tag, and platform-specific editor settings
- `GODOT-EDITOR-009` — Partially implemented in Sim and should be extended: perform undo, redo, history navigation, inspector pinning, and multi-object edits without reentrant updates
- `GODOT-EDITOR-010` — Partially implemented in Sim and should be extended: save, save-as, save-all, autosave, recover, and warn before closing unsaved resources
- `GODOT-EDITOR-011` — Partially covered by an existing migration spec: run and stop the main scene, current scene, selected scene, and custom runnable with embedded game controls
- `GODOT-EDITOR-012` — Missing from the migration specs: expose output, debugger, profiler, audio, animation, shader, navigation, and import bottom panels
- `GODOT-EDITOR-013` — Missing from the migration specs: support distraction-free, multi-window, presentation, and embedded-play layout modes
- `GODOT-EDITOR-014` — Partially implemented in Sim and should be extended: search help and class reference by class, method, property, signal, constant, and theme item
- `GODOT-R3D-011` — Partially covered by an existing migration spec: preview imported meshes and materials with orbit, lighting, animation, and failure diagnostics
- `GODOT-UI-001` — Partially implemented in Sim and should be extended: lay out Controls using anchors, offsets, grow directions, minimum sizes, containers, aspect ratios, and RTL mirroring
- `GODOT-UI-002` — Partially implemented in Sim and should be extended: route mouse, touch, keyboard, controller, shortcut, focus, tooltip, and drag/drop events through Control hierarchy
- `GODOT-UI-003` — Partially implemented in Sim and should be extended: provide buttons, ranges, lists, trees, tabs, menus, dialogs, color/file pickers, splitters, and scroll containers
- `GODOT-UI-004` — Partially implemented in Sim and should be extended: edit plain and rich text with selection, undo, syntax, bidi, shaping, images, tables, meta links, and IME
- `GODOT-UI-005` — Partially implemented in Sim and should be extended: resolve theme inheritance, type variations, icons, fonts, sizes, colors, style boxes, and live overrides
- `GODOT-UI-006` — Partially implemented in Sim and should be extended: manage popups, modal dialogs, embedded windows, exclusive state, and safe cancellation
- `GODOT-UI-007` — Partially implemented in Sim and should be extended: expose accessible roles, names, values, actions, focus, and tree updates to platform assistive technology
- `GODOT-UI-008` — Missing from the migration specs: preview and migrate Godot UI scenes without claiming GPUI is runtime-compatible by default
- `GODOT-INPUT-001` — Missing from the migration specs: define InputMap actions, deadzones, physical/logical keys, device filters, and multiple event bindings
- `GODOT-INPUT-002` — Missing from the migration specs: report pressed, just-pressed, just-released, strength, vector, mouse velocity, and accumulated input deterministically
- `GODOT-INPUT-003` — Missing from the migration specs: handle keyboard, mouse, pen, touch, gestures, gamepads, hotplug, mappings, vibration, sensors, and emulation
- `GODOT-INPUT-004` — Partially implemented in Sim and should be extended: create and manage multiple windows, screens, modes, flags, focus, DPI, scale, vsync, orientation, and safe areas
- `GODOT-INPUT-005` — Partially implemented in Sim and should be extended: support clipboard, cursor, mouse modes, virtual keyboard, IME composition, and text input
- `GODOT-INPUT-006` — Partially implemented in Sim and should be extended: expose accessibility activation, semantic trees, actions, bounds, focus, announcements, and deactivation
- `GODOT-INPUT-007` — Missing from the migration specs: load translations, select locale and fallbacks, pluralize, remap resources, shape bidi text, and mirror layout
- `GODOT-INPUT-008` — Missing from the migration specs: handle suspend, resume, low-memory, quit, focus, file-drop, and platform notification events
- `GODOT-INPUT-009` — Missing from the migration specs: provide dummy/headless display, audio, input, and text drivers with explicit unsupported behavior
- `GODOT-SIM-007` — Missing from the migration specs: author and play Animation, AnimationPlayer, AnimationTree, Tween, tracks, blends, state machines, method/audio tracks, and root motion
- `GODOT-SIM-008` — Missing from the migration specs: route sample playback through buses, sends, effects, capture, device switching, spatial emitters, polyphony, and interactive music
- `GODOT-SCRIPT-001` — Missing from the migration specs: register script languages and create, load, reload, instance, attach, detach, and free scripts with object lifetime
- `GODOT-SCRIPT-002` — Partially covered by an existing migration spec: parse and compile GDScript including typed syntax, annotations, lambdas, pattern matching, classes, inheritance, and warnings
- `GODOT-SCRIPT-003` — Missing from the migration specs: execute GDScript bytecode, calls, properties, signals, coroutines, awaits, errors, stack traces, and deterministic tests
- `GODOT-SCRIPT-004` — Missing from the migration specs: run @tool scripts in the editor with explicit trust, reload, inspector, undo, and failure isolation
- `GODOT-SCRIPT-005` — Missing from the migration specs: build, load, run, debug, hot-reload, export, and diagnose C# projects and assemblies when Mono is enabled
- `GODOT-SCRIPT-006` — Partially covered by an existing migration spec: serve GDScript completion, hover, symbols, rename, references, formatting, diagnostics, semantic tokens, and DAP debugging
- `GODOT-SCRIPT-007` — Missing from the migration specs: evaluate Expression resources with input names, base instances, parse errors, and execute failures
- `GODOT-SCRIPT-008` — Missing from the migration specs: preserve exported script properties and placeholder instances when a script is missing or invalid
- `GODOT-EXT-001` — Missing from the migration specs: parse .gdextension manifests and select libraries by OS, architecture, build, and feature tags
- `GODOT-EXT-002` — Missing from the migration specs: validate GDExtension minimum version, entry symbol, ABI, interface functions, and initialization levels
- `GODOT-EXT-003` — Missing from the migration specs: load and unload extension libraries while registering classes, methods, properties, signals, constants, virtuals, and singletons
- `GODOT-EXT-004` — Missing from the migration specs: marshal Variants, native structures, pointers, call errors, object bindings, memory, strings, arrays, and dictionaries across the ABI
- `GODOT-EXT-005` — Missing from the migration specs: generate and preserve extension_api.json and gdextension_interface.h compatibility contracts
- `GODOT-EXT-006` — Missing from the migration specs: discover plugin.cfg addons and enable, disable, persist, reload, and diagnose EditorPlugin instances
- `GODOT-EXT-007` — Missing from the migration specs: allow editor plugins to add docks, inspectors, importers, exporters, gizmos, debuggers, settings, shortcuts, and autoloads with cleanup
- `GODOT-EXT-008` — Partially implemented in Sim and should be extended: reuse Sim extension trust, capability, installation, and UI boundaries instead of creating a second plugin manager
- `GODOT-IMPORT-001` — Partially covered by an existing migration spec: scan the project filesystem incrementally with ignore rules, UIDs, type detection, moves, removals, and watcher reconciliation
- `GODOT-IMPORT-002` — Partially covered by an existing migration spec: select importers by extension and priority and persist importer, options, source, destination, remap, generator, and validity metadata
- `GODOT-IMPORT-003` — Missing from the migration specs: queue threaded imports and reimports with progress, cancellation, restart, dependency ordering, and failure isolation
- `GODOT-IMPORT-004` — Missing from the migration specs: invalidate imported caches from source hashes, importer versions, settings, dependencies, feature tags, and generated files
- `GODOT-IMPORT-005` — Missing from the migration specs: import images and SVGs into textures with compression, mipmaps, color-space, normal-map, atlas, and platform variants
- `GODOT-IMPORT-006` — Missing from the migration specs: import audio into streams/samples with compression, looping, normalization, trimming, and channel modes
- `GODOT-IMPORT-007` — Missing from the migration specs: import 3D scenes and animations with node/path filters, materials, meshes, skins, LOD, lightmaps, physics, and post-import scripts
- `GODOT-IMPORT-008` — Missing from the migration specs: import glTF, FBX, OBJ, Blender, DAE, and other enabled formats with dependency and unsupported-feature diagnostics
- `GODOT-IMPORT-009` — Missing from the migration specs: import fonts, translations, CSV, bitmaps, textures, shaders, and custom plugin formats
- `GODOT-IMPORT-010` — Partially covered by an existing migration spec: link source assets, imported outputs, generated files, resource UIDs, dependencies, owners, and reimport actions in the project panel
- `GODOT-EXPORT-001` — Partially covered by an existing migration spec: parse, edit, duplicate, reorder, persist, and validate export presets, filters, features, patches, and custom options
- `GODOT-EXPORT-002` — Missing from the migration specs: discover, install, uninstall, mirror, and validate matching debug/release export templates without silent downloads
- `GODOT-EXPORT-003` — Missing from the migration specs: export project data as PCK/ZIP or embedded pack with include/exclude filters, remaps, conversion, and deterministic manifests
- `GODOT-EXPORT-004` — Partially covered by an existing migration spec: export debug, release, and dedicated-server builds from editor or CLI and propagate progress, cancellation, warnings, and errors
- `GODOT-EXPORT-005` — Missing from the migration specs: export and deploy Android APK/AAB/Gradle builds with SDK/JDK/keystore/permissions/architectures and remote run
- `GODOT-EXPORT-006` — Missing from the migration specs: export iOS, macOS, and visionOS bundles/projects with entitlements, privacy manifests, provisioning, codesign, notarization, and architectures
- `GODOT-EXPORT-007` — Missing from the migration specs: export Linux/BSD and Windows executables with architectures, icons, metadata, signing, console mode, and embedded data
- `GODOT-EXPORT-008` — Missing from the migration specs: export Web builds with WASM, threads, service worker/PWA, extensions, HTML shell, compression, and browser feature validation
- `GODOT-EXPORT-009` — Missing from the migration specs: encrypt packs or scripts and protect credentials/signing material without persisting secrets in project files
- `GODOT-EXPORT-010` — Partially covered by an existing migration spec: launch, stop, remote-deploy, and collect logs from an exported or editor-run project through existing Sim tasks
- `GODOT-NET-001` — Missing from the migration specs: read, write, seek, resize, flush, compress, encrypt, hash, map, and atomically replace files through res:// and user://
- `GODOT-NET-002` — Missing from the migration specs: list, create, rename, copy, remove, and watch directories while confining paths and preserving platform semantics
- `GODOT-NET-003` — Missing from the migration specs: resolve DNS and use TCP, UDP, Unix sockets, PacketPeer, StreamPeer, multicast, broadcast, IPv4, and IPv6 with nonblocking errors
- `GODOT-NET-004` — Partially implemented in Sim and should be extended: perform HTTP requests, redirects, proxies, cookies/headers, body streaming, downloads, timeouts, cancellation, TLS, and size limits
- `GODOT-DEBUG-001` — Partially implemented in Sim and should be extended: format, route, filter, timestamp, persist, and flush stdout/stderr, print, warning, error, and structured engine log messages
- `GODOT-DEBUG-002` — Partially covered by an existing migration spec: connect and authenticate editor/runtime debugger sessions with protocol negotiation, timeouts, reconnect, and multiple instances
- `GODOT-DEBUG-003` — Partially covered by an existing migration spec: set breakpoints and exception breaks and inspect stacks, locals, members, globals, expressions, errors, and live script reload
- `GODOT-DEBUG-004` — Missing from the migration specs: inspect and edit the remote scene tree, nodes, resources, properties, camera overrides, selection, and live edits safely
- `GODOT-DEBUG-005` — Missing from the migration specs: profile script/native time, calls, frame stages, GPU, servers, memory, resources, and custom monitors with bounded sampling
- `GODOT-DEBUG-006` — Missing from the migration specs: profile multiplayer RPC/bandwidth and visualize collisions, paths, navigation, canvas redraw, and rendering diagnostics
- `GODOT-DEBUG-007` — Partially implemented in Sim and should be extended: capture errors and crashes with backtraces, symbols, platform handlers, suppression rules, and safe shutdown/reporting
- `GODOT-DEBUG-008` — Missing from the migration specs: recover editor state after a crashed game/editor/plugin and preserve actionable logs without claiming success
- `GODOT-CLI-001` — Partially covered by an existing migration spec: resolve project path, main pack, scene, editor, project-manager, and runtime mode with conflict diagnostics
- `GODOT-CLI-002` — Missing from the migration specs: run headless or with dummy display/audio/text/input drivers and report unsupported visual operations
- `GODOT-CLI-003` — Missing from the migration specs: scan/import resources and quit after import or after a requested frame/time boundary with useful exit status
- `GODOT-CLI-004` — Partially covered by an existing migration spec: export or pack named presets from CLI and propagate template, toolchain, signing, progress, cancellation, and failure status
- `GODOT-CLI-005` — Missing from the migration specs: run a script or main loop, pass user arguments, select language, evaluate doctool/test modes, and exit deterministically
- `GODOT-CLI-006` — Partially covered by an existing migration spec: enable remote debug, editor PID, breakpoints, profiler, GPU validation, crash handler, logging, and protocol ports
- `GODOT-CLI-007` — Missing from the migration specs: select rendering/audio/display drivers, GPU, screen, window mode, resolution, locale, time scale, and frame pacing
- `GODOT-CLI-008` — Partially implemented in Sim and should be extended: print stable help, version, path, verbose, benchmark, and build-feature diagnostics without starting a project
- `GODOT-CLI-009` — Missing from the migration specs: run dedicated-server exports and automation without editor-only services or interactive prompts
- `GODOT-SEC-001` — Missing from the migration specs: confine res://, user://, temp, pack, import, extension, and export paths against traversal, symlink, and archive attacks
- `GODOT-SEC-002` — Partially implemented in Sim and should be extended: establish TLS trust from system/bundled/custom certificates and expose hostname, chain, expiry, and protocol failures
- `GODOT-SEC-003` — Missing from the migration specs: request, explain, persist, revoke, and diagnose mobile camera, microphone, storage, network, notification, and XR permissions
- `GODOT-SEC-004` — Partially implemented in Sim and should be extended: store export signing keys, passwords, tokens, and remote credentials through Sim secret facilities with redaction
- `GODOT-SEC-005` — Missing from the migration specs: gate @tool scripts, post-import scripts, GDExtension libraries, and EditorPlugins by explicit project trust and isolation policy
- `GODOT-SEC-006` — Missing from the migration specs: enforce browser sandbox, secure-context, cross-origin, CSP-like embedding, storage, clipboard, fullscreen, and thread prerequisites
- `GODOT-SEC-007` — Missing from the migration specs: bound resource parsing, decompression, image dimensions, archive entries, recursion, network bodies, queues, and worker memory/time
- `GODOT-SEC-008` — Missing from the migration specs: encrypt project data/scripts where configured and document integrity, key-management, and threat-model limitations
- `GODOT-PERSIST-001` — Partially covered by an existing migration spec: round-trip project.godot and override.cfg sections, values, feature overrides, ordering/comments policy, and unknown settings
- `GODOT-PERSIST-002` — Partially covered by an existing migration spec: round-trip text and binary scene/resource formats with version, UID, dependency, unknown-field, and compatibility guarantees
- `GODOT-PERSIST-003` — Partially covered by an existing migration spec: persist import metadata, file cache, UID cache, editor filesystem state, and generated artifacts without treating them as source
- `GODOT-PERSIST-004` — Missing from the migration specs: persist global editor settings, shortcuts, favorites, templates, asset-library state, and per-version migrations
- `GODOT-PERSIST-005` — Missing from the migration specs: persist per-project editor metadata, layouts, open scenes, folding, script breakpoints, run instances, and debugger state
- `GODOT-PERSIST-006` — Missing from the migration specs: provide user:// ConfigFile, FileAccess, resource save, and save-game behavior across desktop/mobile/web storage
- `GODOT-PERSIST-007` — Missing from the migration specs: perform atomic saves, backups, conflict detection, autosave, crash recovery, permission handling, and disk-full reporting
- `GODOT-PERSIST-008` — Missing from the migration specs: convert supported legacy projects/resources/settings with dry-run diagnostics, backups, idempotence, and explicit unsupported cases
- `GODOT-PERSIST-009` — Missing from the migration specs: publish and test a stable compatibility matrix for imported, edited, externally-run, and exported Godot versions
- `GODOT-PLAT-001` — Partially covered by an existing migration spec: run and export on Windows with native windows, input, IME, accessibility, gamepads, audio/MIDI, filesystem, registry, crash handling, signing, and D3D12/Vulkan/GLES
- `GODOT-PLAT-002` — Partially covered by an existing migration spec: run and export on macOS with Cocoa windows, input/IME, accessibility, Metal/Vulkan/GLES, audio/MIDI, filesystem, menus, bundles, sandbox, signing, and notarization
- `GODOT-PLAT-003` — Partially covered by an existing migration spec: run and export on Linux/BSD with X11 and Wayland variants, portals/DBus, input, accessibility/TTS, audio/MIDI, Vulkan/GLES, headless, packaging, and dynamic libraries
- `GODOT-PLAT-004` — Missing from the migration specs: run and export on Android with editor/runtime variants, lifecycle, permissions, input/sensors, accessibility, audio, Vulkan/GLES, plugins, Gradle, APK/AAB, and remote deploy
- `GODOT-PLAT-005` — Missing from the migration specs: run and export on iOS with lifecycle, permissions, touch/sensors, accessibility, audio, Metal, plugins, Xcode project, simulator/device, signing, and privacy manifests
- `GODOT-PLAT-006` — Missing from the migration specs: run and export on visionOS with spatial lifecycle, simulator/device, permissions, Metal, Xcode, signing, and OpenXR/spatial integration
- `GODOT-PLAT-007` — Missing from the migration specs: run and export on Web with WASM, single-thread/pthread variants, browser input/display/audio, storage, networking, JavaScript, WebXR, PWA, and secure-context limits
- `GODOT-PLAT-008` — Missing from the migration specs: run headless and dedicated-server builds without window/audio dependencies and with deterministic exit, signals, and resource limits
- `GODOT-QA-002` — Missing from the migration specs: run resource/API compatibility tests against declared previous versions and detect removed/changed classes, methods, properties, signals, enums, and hashes
- `GODOT-QA-003` — Missing from the migration specs: exercise editor workflows, import/export fixtures, headless modes, crashes, recovery, and platform-specific behavior in integration tests
- `GODOT-QA-005` — Partially covered by an existing migration spec: provide source-backed user/developer docs for supported, divergent, decision-blocked, and excluded migration behavior
- `GODOT-MOD-001` — Missing from the migration specs: resolve all 55 built-in modules and custom modules by default, explicit module flags, dependencies, can_build, platform, architecture, and build profile
- `GODOT-MOD-002` — Missing from the migration specs: enable GDScript and common codec/text/network/import modules by default only when their dependencies and product profile permit
- `GODOT-MOD-003` — Missing from the migration specs: keep Mono/C# and fallback text server opt-in and expose build/runtime/tooling prerequisites
- `GODOT-MOD-004` — Missing from the migration specs: select Godot Physics 2D/3D, Jolt, navigation, OpenXR, WebXR, mobile VR, raycast, and lightmapper modules by subsystem/platform flags
- `GODOT-MOD-005` — Missing from the migration specs: select image/audio/video/texture/mesh/import codecs and builtin-versus-system third-party implementations with license and feature effects
- `GODOT-MOD-006` — Missing from the migration specs: select Vulkan, GLES3, D3D12, Metal, ANGLE, AccessKit, SDL, audio, MIDI, display, and profiler drivers by build and platform
- `GODOT-MOD-007` — Missing from the migration specs: apply disable_3d, advanced GUI, physics, navigation, XR, overrides, path overrides, threads, precision, deprecated, and production options consistently
