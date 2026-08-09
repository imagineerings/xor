# Godot migration audit baseline

## Revisions and source integrity

| Source | Inspected revision | Verification |
| --- | --- | --- |
| Sim | `95c903d0d2feba228d73b813216c2ff2cc585119` (`Add feature specification workflows (#598)`, 2026-08-08T21:31:51+01:00) | `git rev-parse HEAD`; tracked worktree had no modifications. The pre-audit `.agents/specs/godot-migration/` tree was untracked user material and was preserved. |
| Godot | `5b4e0cb0fd279832bbdd69fed5354d4e5ad26f88`, official `4.7-stable` | `projects/godot` has no nested `.git`; `version.py` declares 4.7.0 stable. The official tag was resolved with `git ls-remote`, and all 13,979 local paths and normalized Git blob hashes matched the tag tree. |

The sorted content manifest SHA-256 is `3f52220d352a6156c26f75006476201b548b41b418903832f8d318eb9aca34e2`. `verify_snapshot.py --manifest --index-content projects/godot` reproduces its rows. Twenty-five Windows/Visual Studio files are stored with LF in Git and checked out with CRLF according to `projects/godot/.gitattributes`; the snapshot also does not preserve upstream executable bits. Those expected working-tree transformations explain why the raw local Git tree hash `b410c076a7f1b178f651005d90ad6dc573f7a4a8` differs from the tag tree `a74f51b5f510fdacf72be7d8f8d598e7b7c192cd` without indicating content drift.

Sim and Godot have no configured Git submodules in the inspected trees. Godot's `thirdparty/` contents are vendored source, not submodules.

## Godot build surface

`SConstruct` requires SCons 4.4 and Python 3.9. It discovers `platform/*/detect.py`, detects built-in and optional custom modules, resolves platform flags before module options, and generates module/platform registration sources. Primary targets are `editor`, `template_debug`, and `template_release`, with `arch=auto`, configurable optimization, debug symbols, LTO, production, threads, deprecated compatibility, and single/double precision.

Relevant component defaults and gates:

- Enabled by default: threading, deprecated compatibility, minizip, Brotli, Vulkan, GLES3, volk, AccessKit, ANGLE, SDL, modules, builtin third-party libraries, project-manager update checks, project setting overrides, 3D, advanced GUI, 2D/3D physics, 2D/3D navigation, and XR when the selected platform/module can build them.
- Disabled by default or opt-in: XAudio2, D3D12, Metal, external profiler integrations, tests, developer mode, unsafe incremental options, Ninja/compile database generation, Visual Studio project generation, Steam API usage tracking, Mono/C#, and fallback text server.
- Independently disableable: 3D, advanced GUI, 2D physics, 3D physics, 2D navigation, 3D navigation, XR, setting overrides, export-template path overrides, exception handling, threads, deprecated compatibility, and individual modules.
- Build composition: `build_profile`, `custom_modules`, recursive custom-module discovery, `modules_enabled_by_default`, `module_<name>_enabled`, builtin-versus-system dependency flags, warning/Werror/strict/dev modes, sanitizers and coverage on supported platforms, compiler/linker/launcher overrides, caching, SCU, Ninja, and compilation database modes.
- Rendering/input/accessibility drivers: Vulkan, GLES3, D3D12, Metal, ANGLE, volk, SDL, AccessKit, platform display servers, platform audio/MIDI drivers, and selectable profiling hooks.
- Generated outputs include enabled-module headers, module and platform API registration, test registration, shaders, icons, extension APIs, documentation data, and platform templates. They count only as evidence for the connected capability they support.

## Modules

The snapshot contains 55 built-in module roots with `config.py`:

`astcenc`, `basis_universal`, `bcdec`, `betsy`, `bmp`, `camera`, `csg`, `cvtt`, `dds`, `enet`, `etcpak`, `fbx`, `freetype`, `gdscript`, `glslang`, `gltf`, `godot_physics_2d`, `godot_physics_3d`, `gridmap`, `hdr`, `interactive_music`, `jolt_physics`, `jpg`, `jsonrpc`, `ktx`, `lightmapper_rd`, `mbedtls`, `meshoptimizer`, `mobile_vr`, `mono`, `mp3`, `msdfgen`, `multiplayer`, `navigation_2d`, `navigation_3d`, `noise`, `objectdb_profiler`, `ogg`, `openxr`, `raycast`, `regex`, `svg`, `text_server_adv`, `text_server_fb`, `tga`, `theora`, `tinyexr`, `upnp`, `vhacd`, `visual_shader`, `vorbis`, `webp`, `webrtc`, `websocket`, `webxr`, `xatlas_unwrap`, and `zip`.

`mono` and `text_server_fb` explicitly return disabled from `is_enabled()`. Other detected modules default enabled when `modules_enabled_by_default=yes`, subject to `can_build`, dependencies, architecture, platform, and subsystem flags. Important conditions include OpenXR on Linux/BSD, Windows, Android, and macOS when XR is enabled; WebXR requiring GLES3 and, on Web, no `proxy_to_pthread`; camera excluding BSD variants; raycast architecture constraints; Theora excluding RISC-V; navigation and physics subsystem disables; and Mono requiring a platform that advertises support.

## Platform source roots and variants

| Platform root | Relevant variants/options |
| --- | --- |
| `platform/android` | Runtime, Android editor, API/plugin/variant/export roots; store release, Swappy frame pacing, Gradle/APK/AAB, permissions, remote deploy. |
| `platform/ios` | Device/simulator, app bundle generation, native API/export, Xcode/signing/privacy metadata. |
| `platform/linuxbsd` | X11 and Wayland, libdecor, DBus/portals, ALSA/PulseAudio, Speech Dispatcher, fontconfig, udev, touch, static C++ and dynamic library wrapping, sanitizers/coverage. |
| `platform/macos` | Native editor/export, app bundle, sanitizers/coverage, Metal/Vulkan/GLES and Apple integrations. |
| `platform/visionos` | Device/simulator, app bundle, native API/export, spatial lifecycle and Apple toolchain. |
| `platform/web` | Web API/editor/export/JavaScript roots, extension policy, JavaScript eval, threads/proxy modes, sanitizers/safe heap, closure, SIMD, PWA/browser constraints. |
| `platform/windows` | MSVC/MinGW/LLVM, GUI/console subsystem, static runtime, sanitizers, PIX, WinRT, native export and signing. |

Platform-specific behavior also exists under `drivers/` for AccessKit, SDL, Vulkan, GLES3, D3D12, Metal, audio, MIDI, Unix, Windows, Apple, EGL, backtrace, and image support. `platform/*/export` and `platform/*/api` are discovered separately by the build.

## Tests, documentation, build, and CI roots

- Tests: `tests/core`, `tests/scene`, `tests/servers`, `tests/compatibility_test`, `tests/python_build`, module-local `tests`, editor test hooks, and platform fixtures.
- Documentation: `doc/classes`, module/platform `doc_classes`, `doc/tools`, embedded editor help, translations, and API/hash generators.
- CI: `.github/workflows/{android,ios,linux,macos,web,windows}_builds.yml`, `runner.yml`, and `static_checks.yml`.
- Build/release tooling: `SConstruct`, 209 `SCsub` files, `methods.py`, builder scripts, `misc/scripts`, `misc/dist`, platform templates, and third-party manifests.

## Sim evidence boundary

The Sim workspace has existing project/worktree/workspace/editor/GPUI/language/LSP/DAP/task/filesystem/network/settings/persistence/sandbox/extension/media/platform/test infrastructure. It has no `crates/sim_game` or `crates/world_model` workspace member and repository-wide source searches found no connected `project.godot`, `.tscn`, `.tres`, GDScript, GDExtension, Godot importer, or Godot exporter implementation. Accordingly, checked tasks and proposed interfaces in the pre-audit specifications were not accepted as implementation evidence.
