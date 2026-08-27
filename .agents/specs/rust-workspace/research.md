# Zed Rust Development Environment — Research

**Evaluation date:** 2026-08-13  
**Scope:** RustRover 2026.2 as behavioural reference; Zed v1.15.0 stable plus Zed `main`; rust-analyzer current public release evidence.  
**Research gate:** no Zed product implementation was performed.

## 1. Tested/reference versions and environment

| Component | Version / revision | Evidence / note |
|---|---|---|
| RustRover | 2026.2, build 262.8665.323 | JetBrains 2026.2 Knowledge Base and 2026.2 Help |
| Zed stable | v1.15.0, published 2026-08-12 | GitHub latest release |
| Zed `main` | `0307288d903afd5673c361c548bd448fc8a684df` at 2026-08-13 19:14:49Z | GitHub commits API |
| rust-analyzer | latest indexed stable release observed: 2026-07-27 / v0.3.2989 / commit `12c3381` | rust-analyzer GitHub releases; the index did not expose a newer dated stable release at research time |
| Research runtime | Linux x86_64, kernel 6.18.35 | execution environment for this research |
| Rust toolchain in research runtime | unavailable | `rustc`, `cargo`, and `rust-analyzer` are not installed in this sandbox |
| Git | 2.47.3 | research runtime |

### Important limitation

This environment has no GUI and no installed RustRover, Zed desktop application, Rust compiler, Cargo, or rust-analyzer binary. Therefore this report distinguishes:

- **Verified from product documentation/source** — current public behaviour is documented or visible in current source.
- **Reproducible observation** — only where the research runtime could execute the relevant command.
- **Unknown / hands-on validation required** — interaction details that should not be inferred from documentation alone.

No claim is marked **Missing** solely because it was not found quickly.

## 2. Methodology

The comparison uses the same developer jobs for both products: understand a workspace; select packages/targets/features/toolchains; build/check/run/test/bench/doc/lint; save launch/debug configurations; inspect structured tests; coverage; profiling; debugging; navigation/refactoring; dependency insight; project creation; remote/container work; and Cargo/import failure diagnosis.

Evidence hierarchy:

1. Current official product documentation and release notes.
2. Current Zed public source at the recorded `main` SHA.
3. Current rust-analyzer public release/manual evidence.
4. Public Zed issues/PRs when an architectural/product gap is under active discussion.
5. Hands-on observation where possible.

JetBrains implementation details are intentionally not inferred. RustRover is treated only as a behavioural reference.

## 3. Evaluation fixture specification

A disposable fixture should be created during implementation validation, containing:

- workspace members `app`, `core`, `macros`, and `ffi`;
- lib + bin targets, example, integration tests, unit tests, doctest, benchmark;
- proc-macro crate and `build.rs`;
- mutually interacting features (`tls-native`, `tls-rustls`, `experimental`);
- target-specific dependency and `cfg` blocks;
- dev/release/custom Cargo profiles;
- Tokio async binary;
- a Criterion benchmark;
- a small unsafe/FFI module;
- one intentionally failing test and one compilation diagnostic;
- enough symbols and packages to exercise refresh/navigation.

The fixture was **specified but not executed** because Cargo is unavailable in the research runtime. Before implementation begins, validate the top three features both against this fixture and a large real workspace such as Zed itself.

## 4. Current Zed baseline

### Language intelligence

Zed's Rust language support is built around rust-analyzer rather than a parallel Rust semantic engine. Current Zed documentation describes Rust language support and settings around rust-analyzer. Existing Zed primitives cover completion, diagnostics, code actions/quick fixes, inlay hints, semantic highlighting/tokens, go-to definition, rename, references, macro expansion/runnables where exposed by rust-analyzer, and language-server task integration.

**Assessment:** mostly **Complete** or **Partial**, depending on the specific rust-analyzer method. The correct roadmap is to expose missing generic LSP affordances before recreating Rust analysis.

### Cargo/runnables/tasks

Zed's task system supports global, worktree, one-shot, and language-extension tasks. It exposes worktree/editor variables and a Rust-specific `ZED_CUSTOM_RUST_PACKAGE`, and supports runnable tag binding. rust-analyzer runnables can therefore feed existing task execution rather than requiring another process runner.

**Assessment:** **Partial**. Execution is capable; configuration, target discovery, persistence, and structured outcomes remain fragmented or text/JSON oriented.

### Debugging

The current Rust documentation states that Zed supports Rust binaries and tests out of the box with CodeLLDB and GDB and can infer output binaries from Cargo build/test commands. Generic debugger documentation supports launch and attach, breakpoints, variables, stacks/threads and DAP-based behaviour, with debug configurations in `.zed/debug.json` and user-global debug configuration.

**Assessment:** **Partial to Complete** for generic DAP fundamentals; **Partial** for Rust-specific discoverability and higher-level Cargo-target configuration. Any proposed panic breakpoint, pretty-printer, memory/disassembly, data-breakpoint or async-task feature must be gated by adapter capability discovery rather than assumed to be a Zed defect.

### Remote, containers and trust

Zed's remote architecture runs source, language servers, tasks and terminals on the remote server while keeping the UI local. Dev Containers similarly run tasks, terminals and language servers in the container. Worktree Trust starts projects restricted and prevents project-local settings plus language/MCP server execution until trust is granted; trust is per host for local/SSH/WSL.

**Assessment:** **Complete as a platform foundation**. New Cargo/test/coverage/profile commands must use the same execution side as the project and require trust before project-controlled execution.

### Generic panels

Project, Outline, Diagnostics, Terminal, Debugger and Git surfaces already exist. The roadmap should extend these patterns or introduce narrowly scoped generic result views, not add a parallel Rust windowing system.

## 5. RustRover workflow observations

The following are behavioural observations supported by current JetBrains documentation, not implementation claims.

### Workspace/project creation and Cargo model

RustRover 2026.2 can create a Cargo project from the new-project flow, detect a Rust toolchain, and presents Cargo-oriented project/target actions. Its Cargo tool window provides a discoverable place to run Cargo commands/targets.

**Outcome to borrow:** a developer can see and act on Cargo structure without remembering command syntax.  
**Do not copy:** JetBrains window layout, names, icons, wording or visual hierarchy.

### Saved Cargo run/debug configurations

RustRover Cargo configurations support a named configuration, Cargo command/options and program arguments, and can be stored as a project file for sharing. Current documentation also supports custom launch behaviour and before-launch work.

**Outcome to borrow:** one reusable configuration feeds repeatable run/debug workflows and can be project-shared when safe.

### Structured test execution

RustRover creates a default workspace test configuration, supports gutter execution for tests/doctests, and shows results in a structured Run tool window. 2026.1 added native cargo-nextest integration.

**Outcome to borrow:** tests are entities with status, hierarchy, duration/output and rerun/debug operations, rather than only terminal commands.

### Coverage

RustRover supports Run with Coverage, a Coverage tool window, multiple coverage suites and merged display. The editor can present coverage data without forcing the user to manually inspect raw reports.

**Outcome to borrow:** optional analysis results are attached to source and can be revisited/merged.

### Debugger depth

RustRover documents breakpoints, variables/watch, frames/threads, stepping, memory and disassembly. 2025.2 added remote/on-chip debugger improvements. These are largely platform/debug-adapter capabilities and should not automatically become Rust-specific Zed work.

### Navigation and macro workflows

RustRover 2026.1 added call hierarchy and easier macro-expansion access; 2026.2 added an interactive declarative macro tester and framework-aware axum/reqwest navigation.

**Outcome to borrow selectively:** expose standards-based/rust-analyzer capabilities generically where possible. Framework-specific semantic modelling is not a first-roadmap priority for Zed.

### Benchmarks

RustRover 2026.2 added Criterion benchmark run configurations and gutter launch.

**Outcome to borrow:** benchmarks should be first-class Cargo targets/configurations, but their result visualization can initially remain external/terminal based.

## 6. Complete feature comparison matrix

Legend: **C** Complete, **P** Partial, **M** Missing after verification, **D** Different by design, **U** Unknown / hands-on evidence required.

| Capability | Developer job | RustRover behaviour | Zed behaviour | Gap | Frequency | Value | Rust specificity | Zed fit | Reuse path | Surface | Cross-platform / remote / security | Perf & maintenance | Recommendation | Confidence |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Rust semantic editing | edit safely | first-class Rust analysis | rust-analyzer-backed LSP | C/P | Very high | High | Rust | High | LSP + rust-analyzer | core/LSP | server runs with project; trust applies | avoid duplicate indexing | Reject duplicate | High |
| Cargo workspace model | understand members/targets | Cargo-oriented project model/tooling | no verified unified structured Cargo view | P | Very high | Very high | Rust | High | project/worktree + `cargo_metadata` | core | execute metadata where project runs; trusted only if project config execution involved | cache/invalidate; no per-keystroke Cargo | **Now** | High |
| Cargo dashboard/actions | discover targets/actions | Cargo tool window actions | tasks/runnables/terminal are distributed | P | High | High | Rust | High with progressive disclosure | project model + tasks | core Rust UI | same host/container; trust for execution | lightweight view only | **Now** (with project model) | High |
| Cargo feature selection | switch cfg/features | structured configuration | rust-analyzer/Cargo settings can be configured manually | P | High | High | Rust | High | Rust project model + settings | core | worktree scoped; remote model owns resolved state | debounce RA reload | **Now** (config model) | High |
| Saved Cargo run preset | repeat run with args/env/profile | named Cargo config, shareable | tasks/debug JSON can persist but are separate/manual | P | High | High | Rust | High | tasks + debugger | core + config schema | secrets user-local; project presets reviewable | no new runner | **Now** | High |
| Saved Cargo debug preset | repeat debugging | Cargo run/debug configuration | `.zed/debug.json`, Cargo build inference | P | High | High | Rust | High | DAP + tasks + config model | core | capability gated; trust required | no adapter duplication | **Now** | High |
| Test discovery/explorer | see tests before/after runs | structured test runner | runnable gutter/task execution; no verified generic structured explorer | M/P | High | Very high | systems/general | High | tasks + generic result model | core generic + Rust provider | discovery remote; do not compile continuously | protocol/parser stability is key risk | **Now** | Med-High |
| Rerun failed/filter/status/duration | iterate tests | structured runner | terminal/task-oriented | M/P | High | High | generic | High | generic result model | core | remote execution | bounded output retention | **Now** | Med-High |
| cargo-nextest integration | large test suites | native integration documented | external task possible; no verified structured integration | P | Med-High | High | Rust | High | test provider + external tool | external integration/core adapter | explicit install; remote binary | version/protocol maintenance | Next | High |
| Doctest entity/results | validate docs | detected and shown with tests | runnable via rust-analyzer/Cargo; structured results not verified | P | Medium | Medium | Rust | High | test provider | core provider | remote | discovery correctness | Next via test explorer | Med |
| Coverage collection | identify untested code | integrated coverage suites | no verified first-class Rust coverage UI | M | Medium | High | systems/general | High if optional | generic overlays + `cargo llvm-cov` | core generic + external collector | tool runs project-side; explicit setup; trust | artifact parsing, stale data | Next | High |
| Coverage gutter/summary | navigate uncovered lines | integrated source presentation | no verified generic coverage overlay | M | Medium | High | generic | High | editor markers + generic analysis overlay | core generic | local UI from remote result | large report memory | Next | Med-High |
| Profiling launch | find hot code | integrated/platform profiler workflows | external tasks/terminal available | P | Medium | Medium-High | systems | Medium | tasks + artifact opener | External/Next | platform-specific; remote complex | profiler upkeep/licensing | External first | High |
| Native flamegraph/call tree | inspect profile | richer IDE workflows depending platform | no verified native view | M | Low-Med | Medium | generic | Medium | future generic profiler view | Later | substantial platform variance | high complexity | Later | Med |
| Attach process picker | debug running process | debugger attach workflows | generic attach documented; picker quality needs hands-on validation | P/U | Medium | Medium | generic | High | debugger UI/DAP | core generic | OS/remote process enumeration | adapter-dependent | Next if gap confirmed | Medium |
| Panic breakpoint preset | stop on Rust panic | debugger can configure breakpoint behaviour | adapter-specific support not fully verified | U/P | Medium | Medium | Rust | High if thin preset | DAP capability + CodeLLDB/GDB | Rust preset | adapter/platform differences | low if capability backed | Next/External | Medium |
| Memory/register/disassembly | low-level debug | documented memory/disassembly | DAP/debugger support varies; current exact Rust parity not fully verified | P/U | Low-Med | High for systems work | generic | Medium | DAP/debugger_ui | generic core | adapter/platform capability | expensive UI if absent | Later | Medium |
| Call hierarchy | understand callers/callees | 2026.1 feature | public Zed issue #14203 requests LSP call hierarchy; source search found no `CallHierarchy` symbol at recorded revision | M | Medium | High | generic LSP | Very high | LSP + generic hierarchy view | core generic | language server side remote | modest | Next | High |
| Find usages grouping | understand impact | structured usages | references available; traditional grouped references UI historically requested (#5117) | P | High | High | generic | High | editor/project search | core generic | remote LSP | UI/data grouping only | Next | Med-High |
| Rename preview/conflicts | refactor safely | IDE refactoring UI | rename via LSP; exact preview/conflict UX needs validation | P/U | Medium | High | generic | High | LSP workspace edits | core generic | server-side compute | no duplicate semantics | Next if verified gap | Medium |
| Advanced move/extract/inline/change signature | refactor | broader IDE refactorings | rust-analyzer actions vary; Zed should expose what server supplies | P | Medium | Medium-High | Rust/generic | High selectively | code actions/custom LSP | core generic first | remote LSP | RA capability changes | Later/Next | Medium |
| Macro expansion | inspect generated code | expansion + 2026.2 macro tester | rust-analyzer macro expansion capability available; UX depth differs | P/D | Medium | Medium | Rust | High for simple viewer | rust-analyzer command | Rust integration | remote LSP | low | Next only for history/UX | High |
| Dependency versions/tree | understand dependency graph | Cargo/manifests integrated assistance | extensions/tasks can assist; no verified unified dependency view | P | Medium | Medium-High | Rust | High if low-noise | Cargo model + manifest editor | extension/core hybrid | network only explicit; project-side Cargo | registry freshness | Next | Med |
| Audit/deny/outdated/unused | dependency hygiene | plugins/external tooling possible | tasks/external tools | D/P | Medium | Medium | Rust | High as external integration | tasks | External | explicit install/network/trust | tool churn | External | High |
| Project/crate creation | create Rust codebase | New Project flow | rust-analyzer itself added Create Rust project command in 2026; Zed exposure needs verification | P/U | Low-Med | Medium | Rust | High command-first | LSP command or `cargo new` | extension/core command | trust/path validation | low | Next | Medium |
| Scratch/REPL | experiment | IDE scratch workflows | terminal/external tools possible | D/P | Low | Low-Med | Rust | Medium | terminal + evcxr external | External | explicit install | tool maintenance | External | High |
| Database/HTTP/general IDE suite | unrelated workflows | broad IntelliJ platform tooling | external tools/extensions | D | varies | Low for Rust roadmap | generic | Low | external apps/extensions | external | independent | core bloat | **Reject** | High |
| Axum/reqwest semantic navigation | web framework workflow | 2026.2 framework-aware support | rust-analyzer/general navigation; no equivalent framework engine verified | M/D | subset | Medium for web Rust | Rust/framework | Low-Med for core | extension/LSP ecosystem | External/extension | local-first possible | high semantic upkeep | Later/External | High |
| `.env` semantic rename/navigation | env correctness | 2026.2 Rust integration | generic env files/extensions; cross-file semantic link not verified | P | Medium | Medium | generic/framework | Medium | language extensions/LSP | extension | avoid secret leakage | modest | External/Next generic | Medium |

## 7. Architecture findings in current Zed

Current workspace dependencies explicitly include:

- `cargo_metadata = "0.19"` and `cargo_toml = "0.21"`;
- `dap`, `dap_adapters`, `debug_adapter_extension`, `debugger_tools`, `debugger_ui` crates;
- `dev_container`, `diagnostics`, `editor`, `extension`, `extension_host`, `project`, task-related systems and GPUI.

This strongly supports extending existing project/task/DAP/editor infrastructure instead of creating Rust-only process management.

### Likely implementation surfaces to verify in checkout before coding

Because this research did not have a local Zed checkout, exact type-level ownership must be rechecked at the recorded or implementation SHA. The stable crate-level surfaces are:

- `crates/project` — project/worktree coordination and language-server-facing state;
- `crates/task` and task UI surfaces — task templates/resolution/execution;
- `crates/dap`, `crates/dap_adapters`, `crates/debugger_tools`, `crates/debugger_ui` — debugger protocol/adapters/presentation;
- `crates/editor` — gutter/source annotations and navigation entry points;
- `crates/worktree` / workspace trust integration — security gate;
- `crates/remote*` and `crates/dev_container` — execution placement/transport;
- GPUI component/panel primitives — presentation.

**Do not start implementation using guessed type names.** First task in each milestone is a source trace at the pinned SHA and an architecture note updating exact module/type/action names.

## 8. Existing related work / overlap

Verified public Zed issue signals include:

- **#14203 — “Support 'Show Call Hierarchy' as an LSP action (prepareCallHierarchy)”**: confirms call hierarchy is a generic LSP gap and should not be implemented as Rust-only logic.
- **#5117 — traditional “Find All References” style views**: supports improving grouped usage presentation generically.
- **#32932 — debugger hover variable values**: demonstrates debugger UX is actively evolving and Rust-specific proposals must avoid duplicating generic work.
- **#26916 — Language Server Protocol improvements tracking**: relevant umbrella for generic LSP exposure.

A fresh issue/PR search must be repeated immediately before opening implementation PRs because Zed `main` changes rapidly.

## 9. Prioritization score

Raw attributes are 1–5. Positive weights reward value/frequency/specificity/improvement/alignment/reuse/cross-platform/remote feasibility. Complexity, performance risk and maintenance burden are costs and are subtracted.

Formula (maximum positive pre-cost = 155; minimum cost = 6):

`score = 5*value + 4*frequency + 2*rust_specificity + 4*improvement + 4*zed_alignment + 3*reuse + 2*cross_platform + 2*remote - 3*complexity - 2*performance_risk - 2*maintenance`

| Candidate | V | F | R | Δ | Fit | Reuse | XPlat | Remote | Cx | Perf | Maint | Score |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Worktree Rust project model + Cargo dashboard | 5 | 5 | 5 | 5 | 5 | 5 | 5 | 5 | 3 | 2 | 2 | **121** |
| Unified Cargo execution/config presets | 5 | 5 | 5 | 4 | 5 | 5 | 5 | 5 | 3 | 2 | 2 | **117** |
| Structured Rust test explorer + generic results | 5 | 5 | 4 | 5 | 5 | 4 | 5 | 5 | 4 | 3 | 3 | **109** |
| Generic LSP call hierarchy | 4 | 3 | 2 | 5 | 5 | 5 | 5 | 5 | 3 | 2 | 2 | **101** |
| Coverage collector + generic overlays | 4 | 3 | 4 | 5 | 4 | 3 | 4 | 4 | 4 | 3 | 3 | **85** |
| Cargo dependency insight | 4 | 3 | 5 | 3 | 4 | 4 | 5 | 5 | 3 | 2 | 3 | **96** |
| Rust debug discoverability/presets | 4 | 3 | 4 | 3 | 4 | 5 | 4 | 4 | 3 | 2 | 3 | **91** |
| Profiling/flamegraph native UI | 3 | 2 | 3 | 5 | 3 | 2 | 2 | 2 | 5 | 4 | 5 | **47** |
| Project/scratch creation | 3 | 2 | 4 | 3 | 4 | 4 | 5 | 5 | 2 | 1 | 2 | **88** |
| General IDE breadth | 2 | 2 | 1 | 3 | 1 | 1 | 3 | 3 | 5 | 3 | 5 | **32** |

The top three also unlock later capabilities; that dependency leverage is why they form **Now**.

## 10. Now / Next / Later / External / Reject

### Now — foundational, maximum three

1. **Worktree-scoped Rust project model + Cargo dashboard.** Establish one cached representation of Cargo workspaces/targets/features/toolchain/metadata health and a progressively disclosed UI/actions surface.
2. **Unified Cargo execution/configuration presets.** One reusable configuration model resolves into existing tasks/debug scenarios for run/test/debug/bench and later coverage/profile.
3. **Structured Rust test explorer backed by generic execution results.** Rust is the first provider, but the suite/case/result/cancel/rerun model should be language-neutral.

### Next

- Generic LSP call hierarchy and richer reference grouping.
- Generic source-analysis overlays + Rust coverage adapter (`cargo llvm-cov` preferred after validation).
- Cargo dependency/feature provenance insight.
- Rust debug discoverability: Cargo-target launch UI, attach/process UX and capability-backed presets.
- Lightweight project/crate creation using rust-analyzer command or `cargo new` after current command exposure is verified.

### Later

- Native profiling/flamegraph/call-tree UI.
- Advanced generic refactoring presentation beyond current LSP/code actions.
- Macro expansion history/comparison or macro-authoring UI.
- Framework-aware axum/reqwest semantic modelling unless an extension/protocol supplies it cheaply.

### External

- Platform profilers initially; open SVG/HTML/artifacts cleanly from tasks.
- `cargo audit`, `cargo deny`, `cargo outdated`, unused-dependency tools as explicit task/tool integrations.
- `evcxr` scratch/REPL.

### Reject for the Rust roadmap

- Database browser, HTTP client and broad IntelliJ-style tool suite in Zed core.
- Any duplicate Rust semantic/indexing engine beside rust-analyzer.
- Any service that requires hosted/cloud execution.

## 11. Key unresolved unknowns

1. Hands-on RustRover 2026.2 exact sequences for feature toggling, failure recovery and certain debugger capabilities must be recorded on macOS/Linux/Windows before parity claims.
2. The exact current rust-analyzer binary bundled/downloaded by Zed v1.15.0 must be captured from a real Zed installation; the report records the latest public release index visible during research, not Zed's installed binary.
3. Stable machine-readable per-test output across `cargo test`, rust-analyzer runnables and cargo-nextest needs protocol validation before choosing a parser. Do not commit to parsing human terminal text in architecture.
4. Exact extension-API ability to add arbitrary panels/source overlays must be checked at implementation SHA. UI-heavy foundations are currently assumed to require Zed core unless the API has evolved.
5. Windows remote-server support is not yet documented as supported by Zed; Windows is supported as a local client, and WSL is supported. Remote feature acceptance must reflect this platform degradation.

## 12. Primary sources

- Zed Rust docs: https://zed.dev/docs/languages/rust
- Zed Rust product page: https://zed.dev/languages/rust
- Zed tasks: https://zed.dev/docs/tasks
- Zed debugger: https://zed.dev/docs/debugger
- Zed remote development: https://zed.dev/docs/remote-development
- Zed dev containers: https://zed.dev/docs/dev-containers
- Zed worktree trust: https://zed.dev/docs/worktree-trust
- Zed GitHub source/release: https://github.com/zed-industries/zed
- RustRover 2026.2 What's New: https://www.jetbrains.com/rust/whatsnew/
- RustRover Quick Start: https://www.jetbrains.com/help/rust/quick-start-guide-rustrover.html
- RustRover Cargo run/debug config: https://www.jetbrains.com/help/rust/cargo-run-debug-configuration.html
- RustRover tests: https://www.jetbrains.com/help/rust/performing-tests.html
- RustRover doctests: https://www.jetbrains.com/help/rust/rust-doctest-support.html
- RustRover coverage: https://www.jetbrains.com/help/rust/code-coverage.html
- RustRover debugging: https://www.jetbrains.com/help/rust/debugging-code.html
- rust-analyzer releases: https://github.com/rust-lang/rust-analyzer/releases
