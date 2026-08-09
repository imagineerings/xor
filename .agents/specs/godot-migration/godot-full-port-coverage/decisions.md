# Decisions requiring review

The native Sim ownership constraint is settled: Godot delegation, runtime linkage, wrappers, hidden instances, and unreviewed source copying are not available alternatives. No direction among the remaining native product/architecture alternatives below was selected by this audit.

| ID | Decision | Material alternatives | Capability impact |
| --- | --- | --- | --- |
| DEC-GODOT-001 | Native runtime product scope and owner composition | Full Sim-native game runtime at existing owners; phased native runtime profiles; import/edit/preview only with executable areas explicitly excluded | Scene lifecycle, rendering, UI runtime, physics, navigation, animation, audio, particles, input, multiplayer, XR, headless, export |
| DEC-GODOT-002 | Compatibility floor and ceiling | Import only; lossless edit/round-trip; runtime compatibility; export compatibility; Godot 4.7 only; bounded 4.x; legacy 3.x conversion | Project, scene/resource formats, scripting, plugins, imports, exports, migrations, tests |
| DEC-GODOT-003 | Script product contract | SimScript replaces executable GDScript and imported scripts are migration-only; native GDScript implementation at existing language/runtime owners; reviewed native C#/Mono tier; explicit script-family exclusions | `GODOT-SCRIPT-*`, project lifecycle, inspector, debugger, export |
| DEC-GODOT-004 | Rendering and simulation scope | Keep explicit exclusions; build selected native equivalents at existing owners; scope to asset preview and authoring metadata | All classification-6 rendering/physics/navigation/particle/network/XR rows and animation/audio gaps |
| DEC-GODOT-005 | Platform tiers | Authoring-only platforms; native exported-runtime platforms; explicitly unsupported targets | `GODOT-PLAT-*`, input/display, import/export, permissions, CI |
| DEC-GODOT-006 | Native extension and plugin trust | Refuse; translate a safe subset into Sim extension models; provide a separately reviewed Sim-owned compatibility host without Godot libraries | `GODOT-EXT-*`, @tool/post-import scripts, security, editor recovery |
| DEC-GODOT-007 | Native import/export ownership | Reproduce a selected importer/exporter subset through existing Sim owners; full native pipeline; explicit unsupported formats/targets | `GODOT-IMPORT-*`, `GODOT-EXPORT-*`, caches, dependencies, formats, platform toolchains |
| DEC-GODOT-008 | Source, fixture, documentation, and asset reuse | Clean-room behavioral implementation; selective MIT source reuse; generated API/docs ingestion; fixture-only reuse | Licensing, attribution, dependency review, maintenance, verification scope |
| DEC-GODOT-009 | Project trust and permission policy | Trust per workspace; signed source; sandboxed tool scripts/plugins; deny by default with explicit grants | Scripts, extensions, post-import hooks, mobile/web permissions, filesystem/network boundaries |
| DEC-GODOT-010 | Resource limits and performance targets | Match selected Godot limits natively; define Sim-specific bounded profiles; explicitly exclude workloads outside approved bounds | Parsing, imports, rendering, networking, worker isolation, mobile/web/headless behavior |

Until these decisions are approved, the catalog retains baseline exclusions and gaps but does not describe them as implemented parity. None may be resolved by launching, linking, wrapping, embedding, or delegating to Godot.
