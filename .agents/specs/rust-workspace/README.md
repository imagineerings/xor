# Rust workspace specification catalog

## Purpose

This directory is the authoritative catalog for Zed's Rust development workspace. Each active subdirectory owns one implementation boundary and is self-contained. Canonical acceptance-criterion and task identities are qualified as `<pack>/<local-id>`; for example, `cargo-dashboard/1.1` and `cargo-dashboard/1.1` (task) are distinct from the same local number in another pack.

The root `requirements.md`, `design.md`, and `tasks.md` mega-specs were retired because their cross-cutting criteria and completion checklist duplicated the canonical packs. Git retains that history. `new-requirements/` and `tool-window/` remain unchanged historical source material, not active implementation plans.

Repository evidence was audited at `efadf85fb80c6ee1776ed01fc78466ce076221d7`. This checkout uses Zed terminology and `crates/zed`; there is no `crates/zed` directory. Historical Zed and RustRover observations are research only.

## Architectural invariants

- The user-facing panel is `Cargo`; `language_tools` owns the generic tree host, `cargo_workspace` the Cargo model/store, and `cargo_ui` the Cargo panel and presets.
- `rust-tools` is the optional distribution capability. It does not make grammars, rust-analyzer, existing Rust language initialization, or all Rust tasks optional.
- Cargo metadata, configuration probes, and Rust test discovery run only on the authoritative project host through existing project environments, trust, remote, and multiplayer boundaries.
- `CargoWorkspaceStore` owns metadata/configuration discovery, never arbitrary build, run, test, debug, coverage, or profile execution.
- Build, check, run, test, bench, debug, and future explicit tools compile into Tasks, terminals, and DAP. No panel parses terminal text or creates a second process runner.
- Structured execution is language-neutral. Rust test discovery is a separate provider and does not duplicate rust-analyzer semantic indexing.
- Protocols contain bounded, project-relative data and no environment values, raw terminal streams, or absolute host paths. Remote clients never fall back to local Cargo.
- No universal build-system model, public provider API, `metal_cargo`, broad `metal_*` naming, automatic dependency mutation, or automatic network/tool installation is in scope.

## Pack ownership and dependencies

| Pack | Canonical owner | Depends on | Roadmap |
| --- | --- | --- | --- |
| [cargo-dashboard](cargo-dashboard/requirements.md) | Cargo discovery, bounded configuration facts, Cargo tree/panel, navigation and refresh | `language_tools`; `rust-tools-platform` boundary | Now baseline; focused gaps |
| [cargo-execution](cargo-execution/requirements.md) | Cargo presets, contextual action availability, Tasks/DAP compilation and preset authoring UX | `cargo-dashboard`; existing Tasks/DAP | Now baseline; Next UX/config gaps |
| [structured-execution](structured-execution/requirements.md) | Generic result/event store, task lifecycle bridge and generic Tests panel | existing Tasks/terminal; `language_tools` | Now baseline; benchmark gap |
| [rust-test-explorer](rust-test-explorer/requirements.md) | Host-side Rust test protocol/discovery and Rust action adapter | `cargo-dashboard`; `cargo-execution`; `structured-execution` | Now baseline; physical compatibility gap |
| [rust-tools-platform](rust-tools-platform/requirements.md) | Compile-time boundary, host parity, privacy/trust matrix, release/CI and cross-capability certification | first four packs | Now baseline; hardening gaps |
| [call-hierarchy](call-hierarchy/requirements.md) | Generic LSP call hierarchy; Rust is a consumer | language/project/editor LSP infrastructure | Next; not started |
| [rust-coverage](rust-coverage/requirements.md) | Generic source-analysis coverage projection plus explicit Rust collector adapter | `cargo-execution`; `structured-execution` | Next; deferred |
| [cargo-dependency-insight](cargo-dependency-insight/requirements.md) | Bounded Cargo dependency/feature provenance insight | `cargo-dashboard` | Next; deferred |
| [rust-profiling](rust-profiling/requirements.md) | Explicit external profiler task/artifact workflow; later generic native view | `cargo-execution`; existing Tasks | External first; Later native UI |

## Feature-status matrix

| Capability | Classification | Evidence | Remaining canonical work |
| --- | --- | --- | --- |
| Generic language-tool tree host | Verified baseline | `crates/language_tools/src/language_tool_tree.rs`; 10,000-row and GPUI timer tests | Formal budgets are platform/dashboard gaps, not a new host |
| Cargo model, discovery and direct-only tree | Verified baseline | `crates/project/src/cargo_workspace.rs`, `cargo_workspace_store.rs`, deterministic fixtures; `crates/cargo_ui/src/cargo_panel.rs` | Comprehensive fixture and measured benchmark |
| Profiles/toolchain/compiler/active configuration | Complete | Cargo configuration fixtures and panel projection tests | Full layered Cargo-config evaluation remains explicitly unsupported |
| Cargo presets and six contextual actions | Partial | `cargo_preset.rs`, `cargo_actions.rs`; Build/Check/Run/Test/Bench/Debug tests | Toolchain/pre-launch fields, visual editor/preview, explicit save workflow, dedicated JSON compatibility tests; selected extra actions |
| Structured result store/task lifecycle/Tests UI | Complete at bounded 10,000-node scope | `structured_execution.rs`, `task.rs`, `tasks_ui/src/test_explorer.rs` | Formal time/memory budget; historical 100,000-node target superseded |
| Rust test protocol/provider/actions | Complete for stable built-in adapter | `rust_test_provider.rs`, fixtures, ignored protocol matrix, Tests panel actions | Physical SSH/dev-container/OS certification; no automatic nextest |
| Remote, multiplayer and feature mismatch | Verified behavioral baseline | typed protobufs, `HeadlessProject` registration tests, injected runners | Physical environment matrix remains partial |
| `rust-tools` disabled/enabled builds | Verified baseline | manifests, `script/check-rust-tools-feature-boundary`, CI and bundle dry runs | Cross-platform release certification remains ongoing |
| Accessibility semantics and keyboard behavior | Partial certification | ARIA labels/roles and GPUI keyboard tests in Cargo/Tests panels | Screen-reader/manual accessibility certification |
| Generic LSP call hierarchy | Not started / Next | no current implementation found | `call-hierarchy` pack |
| Rust coverage | Deferred / Next | no generic coverage model or Rust collector found | `rust-coverage` pack |
| Cargo dependency provenance | Deferred / Next | direct declared/resolved annotations only | `cargo-dependency-insight` pack |
| Profiling | External / Later | ordinary Tasks can launch user tools; no Rust-native profile UI | `rust-profiling` pack |

## Historical task reconciliation

| Historical item | Canonical owner | Status | Exact evidence and remaining gap |
| --- | --- | --- | --- |
| M0.1 Pin implementation baseline | `rust-tools-platform` | Superseded | Upstream SHA assumptions were replaced by the current Zed audit at `efadf85…`; repository paths and behavior are recorded here and in each design. |
| M0.2 Trace exact types/actions | `rust-tools-platform` | Complete | Current symbols are traced in `cargo_ui`, `project`, `tasks_ui`, `task`, `workspace`, `remote_server`, and `zed`; `crates/zed` is absent. |
| M0.3 Build evaluation fixture | `rust-tools-platform` | Partial | Deterministic Cargo and Rust-test fixtures exist under `crates/project/test_data/`; there is no single standalone comprehensive workspace fixture spanning every historical characteristic. |
| M0.4 Structured-test protocol spike | `rust-test-explorer` | Complete | Captured Cargo JSON/listing fixtures, bounded parsers, ignored stable-toolchain matrix, and partial unknown-record behavior in `rust_test_provider.rs`. |
| M1.1 Typed project summaries | `cargo-dashboard` | Complete | `cargo_workspace.rs` typed workspace/package/target/feature/direct-dependency/configuration model. |
| M1.2 Project-side metadata loader | `cargo-dashboard` | Complete | Injectable `CargoMetadataRunner` in `cargo_workspace_store.rs`; authoritative project environment and `kill_on_drop`. |
| M1.3 Trust gate | `rust-tools-platform` | Complete | `TrustedWorktrees` gates metadata/configuration/test runners and execution adapters. |
| M1.4 Cache/generation/invalidation | `cargo-dashboard` | Complete | fingerprinting, debounce, cancellation, stale retention and generation rejection in store/panel tests. |
| M1.5 Remote serialization/ownership | `rust-tools-platform` | Complete | `cargo.proto`, host store registration, visible `ProjectPath` filtering and peer-scoped cancellation. |
| M1.6 Benchmark large workspace | `cargo-dashboard` | Partial | 1,000-package and 10,000-row deterministic scale tests exist; no accepted wall-time/memory budget or repeatable benchmark harness. |
| M2.1 Read-only dashboard tree | `cargo-dashboard` | Complete | Dedicated dockable `CargoPanel` using `language_tool_tree`. |
| M2.2 Active configuration summary | `cargo-dashboard` | Complete | profiles, declared toolchain, host compiler/target, unresolved Cargo default, active preset summary. |
| M2.3 Contextual Cargo actions | `cargo-execution` | Partial | Build, Check, Run, Test, Bench, Debug are implemented through Tasks/DAP. Doc, clippy, fmt, clean and tree are deferred for separate applicability/safety design; `cargo update` is rejected because it mutates resolution and may use the network. |
| M2.4 Error/recovery UX | `cargo-dashboard` | Complete | loading/empty/partial/stale/restricted/missing/error/mismatch states and explicit refresh. |
| M3.1 Typed Cargo execution spec | `cargo-execution` | Partial | schema supports scope, target, profile, features, target triple, argv/env/cwd/presentation; toolchain override and pre-launch task are absent. |
| M3.2 Resolve to Tasks | `cargo-execution` | Complete | pure `compile_preset` emits structured `TaskTemplate`/context. |
| M3.3 Resolve to debugger | `cargo-execution` | Complete | `compile_debug_scenario` uses the Cargo locator and existing DAP path. |
| M3.4 Ephemeral visual editor | `cargo-execution` | Not started | only a non-persisted default preset exists; no form or command preview. |
| M3.5 User/project persistence | `cargo-execution` | Partial | versioned user/project settings precedence and safe workspace state exist; explicit Save for User/Project UI does not. |
| M3.6 Compatibility tests | `cargo-execution` | Partial | Tasks/DAP conversion preserves existing systems and docs state non-replacement; dedicated `tasks.json`/`debug.json` coexistence tests are missing. |
| M4.1 Generic result/event model | `structured-execution` | Complete, narrowed | bounded generic model/protocol/store is complete at 10,000 nodes; the historical unbudgeted 100,000-case target is superseded. |
| M4.2 Provider/execution bridge | `structured-execution` | Complete | structured task lifecycle handle preserves terminal/history and rejects stale events. |
| M4.3 Generic Tests UI | `structured-execution` | Complete | dockable generic Tests panel, filters, summaries, navigation, accessibility and action delegation. |
| M5.1 Rust discovery adapter | `rust-test-explorer` | Complete | authoritative, injectable, bounded Cargo/harness discovery with partial diagnostics. |
| M5.2 Run tests through Tasks | `rust-test-explorer` | Complete | exact node plans schedule ordinary Tasks and lifecycle updates. |
| M5.3 Debug selected test | `rust-test-explorer` | Complete | supported cases compile to existing Cargo DAP scenarios; doctests explain unsupported debug. |
| M5.4 Rerun/ignored/doctest/nextest | `rust-test-explorer` | Complete with narrowing | ignored/doctest/rerun behavior is covered; nextest is explicitly optional Later and never auto-installed. |
| M5.5 Remote/dev-container matrix | `rust-test-explorer` | Partial | headless/fake-host routing tests exist; `rust-test-explorer/2.1`–`2.2` own the missing physical SSH, WSL/dev-container and OS certification. |
| M6 Hardening and graduation | `rust-tools-platform` | Partial | feature boundary, CI, docs, privacy and scale tests landed; comprehensive fixture, formal budgets, physical matrix and screen-reader certification remain. |
| Generic LSP call hierarchy seed | `call-hierarchy` | Not started / Next | Generic LSP owner; no Rust call-graph engine. |
| Coverage seed | `rust-coverage` | Deferred / Next | Generic overlay first, explicit project-host collector later. |
| Cargo dependency insight seed | `cargo-dependency-insight` | Deferred / Next | Extends Cargo evidence without mutation/auditing. |
| Profiling seed | `rust-profiling` | External first / Later | Explicit Tasks and artifacts first; native profiler UI deferred. |

All 14 historical `tool-window/tasks.md` leaves are verified baseline and map to `cargo-dashboard` (model/store/tree/panel), `rust-tools-platform` (protocol/headless/settings/startup/CI), or their shared baseline. They are not recreated as active implementation tasks.

## Source-to-canonical proposal coverage

| Enhancement-summary proposal | Disposition | Canonical owner/rationale |
| --- | --- | --- |
| Do not rebuild Rust intelligence | Rejected alternative / invariant | All packs; rust-analyzer remains semantic authority. |
| Reuse Tasks, terminals, DAP, project/remote/trust | Implemented invariant | `cargo-execution`, `structured-execution`, `rust-test-explorer`, `rust-tools-platform`. |
| Cargo state as connective tissue | Implemented baseline, bounded extension | `cargo-dashboard`; dependency insight owns only the later provenance extension. |
| Unified Cargo execution configuration | Partial | `cargo-execution`. |
| Generic structured test results and Rust provider | Implemented baseline | `structured-execution` and `rust-test-explorer`. |
| Validate stable test protocol | Implemented with partial fallback | `rust-test-explorer`; no terminal scraping. |
| Generic overlays before Rust coverage | Deferred / Next | `rust-coverage`. |
| Generic LSP call hierarchy | Not started / Next | `call-hierarchy`; not gated by Rust tooling. |
| Remote/trust from first release | Implemented baseline; certification partial | `rust-tools-platform` and capability-local designs. |
| Avoid broad IntelliJ parity | Rejected/External | Database/HTTP/framework engines are out of scope; platform profilers and audit/deny/outdated tools remain explicit external Tasks. |
| Dashboard placement decision | Resolved | Dedicated dockable `Cargo` panel. |
| Preset schema/location | Resolved baseline; UI gap | Versioned settings plus safe workspace selection; explicit save workflow remains `cargo-execution`. |
| Test history duration | Resolved | Current and last complete run are session-bounded; only filters and opaque selection state persist. |
| Initial coverage surface | Open | `rust-coverage` recommends generic gutter annotations plus summary, not a dedicated panel for its first milestone. |

## Old-to-new acceptance-criterion migration

Old IDs are from the retired consolidated root spec. New IDs are globally identified by pack path.

| Old IDs | Canonical new IDs |
| --- | --- |
| 1.1–1.9 | `cargo-dashboard/1.1`–`cargo-dashboard/1.9` |
| 2.1–2.8 | `cargo-dashboard/2.1`–`cargo-dashboard/2.8` |
| 3.1–3.10 | `cargo-execution/1.1`–`cargo-execution/1.10` |
| 4.1–4.8 | `structured-execution/1.1`–`structured-execution/1.8` |
| 5.1–5.11 | `rust-test-explorer/1.1`–`rust-test-explorer/1.11` |
| 6.1–6.8 | `rust-tools-platform/1.1`–`rust-tools-platform/1.8` |
| 7.1–7.9 | `rust-tools-platform/2.1`–`rust-tools-platform/2.9` |
| 8.1–8.8 | `rust-tools-platform/3.1`–`rust-tools-platform/3.8` |
| 9.1–9.7 | Split into each owning pack's verification criteria and `rust-tools-platform/4.1`–`rust-tools-platform/4.5` |
| 9.8 | `cargo-dashboard/3.2`–`cargo-dashboard/3.3`, `structured-execution/2.1`–`structured-execution/2.3`, and `rust-tools-platform/4.2` (domain-specific split) |

Historical consolidated task IDs 1–2 map to completed `cargo-dashboard` baseline verification, 3–4 to completed `cargo-execution`, 5–7 to completed `structured-execution`, 8–10 to completed `rust-test-explorer`, and 11–12 to completed `rust-tools-platform`. Remaining unchecked leaves in the new packs are verified gaps only.

## Rejected and external boundaries

- Rejected: second Rust indexer, universal build/package model, public third-party provider API, terminal-output parsing, automatic nextest installation, automatic dependency changes or network commands, replacement of Tasks/terminals/config JSON/DAP, and making all existing Rust support conditional.
- External: `cargo audit`, `cargo deny`, `cargo outdated`, unused-dependency tools, and platform profilers are user-authored explicit Tasks until separately specified. Their output is not silently interpreted.
- Later: native profiling/flamegraph UI and any richer generic analysis surface beyond the approved coverage milestone.

## Open product decisions

Only pack-local material decisions remain: the Cargo preset editor interaction and save confirmation (`cargo-execution`), call-hierarchy initial direction/surface (`call-hierarchy`), the first coverage presentation and supported collector floor (`rust-coverage`), dependency provenance depth (`cargo-dependency-insight`), and external-tool/native-view thresholds (`rust-profiling`). Each pack records a recommended default and the tasks that depend on it.
