# Design: Rust profiling workflow

## Current implementation baseline

Zed's Tasks/terminal can run explicit user commands on local or remote project hosts. Task templates now support a bounded project-relative artifact declaration, and Cargo UI can compile an explicitly configured external profiling command from an existing Cargo context. Workspace opens a declared visible artifact only after successful completion. There is no bundled profiler, terminal parser, native sample model, flamegraph or call-tree view.

## Design decisions

### D1: Keep external Tasks as the current product path

Profilers remain user-selected and installed. Tasks own command/environment/terminal/cancellation/remote behavior. Documentation may provide examples but must not imply bundled tools or parsed results.

### D2: Add only a declarative artifact convenience if approved

An optional Cargo preset profile action compiles to Tasks. A bounded artifact declaration identifies a visible project-relative output and content kind after successful lifecycle. Workspace opens it through existing safe handlers. No terminal scraping or implicit path discovery occurs.

### D3: Preserve authoritative-host and secret boundaries

Remote Tasks execute remotely. Artifacts are opened/transferred only through existing project visibility and size policy. Environment values remain in Tasks; persisted preset/panel state stores no values or host paths. Unsupported platforms/tools are explicit.

### D4: Require an ADR before native model/UI

The ADR must prove collector/version stability, license/distribution, platform matrix, artifact schema, path mapping, remote size/privacy and cancellation. Until accepted, native types/view tasks remain Later and must not invent a universal profiler abstraction.

### D5: If approved, keep the native result contract minimal and generic

The smallest contract is session/run ID, bounded frames/samples/edges, optional visible source, aggregate weights and truncation. Parsing is background/cancellable; UI is accessible/virtualized and language-neutral. Execution still remains Tasks.

## Cross-pack dependencies

- `cargo-execution` supplies pure preset/Task compilation if a shortcut is approved.
- `rust-tools-platform` supplies feature gating and host parity for Rust-specific registration.
- Call hierarchy, coverage and structured tests remain separate data models.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D1 | Existing Tasks/remote documentation and tests |
| 1.2, 1.3 | D2 | Optional pure plan/artifact opener tests |
| 1.4, 1.5 | D1, D3 | No-installer/privacy/remote tests |
| 2.1 | D4 | Approved ADR and tool matrix |
| 2.2, 2.3 | D4, D5 | Generic model/task lifecycle tests if approved |
| 2.4 | D5 | Deterministic cross-platform/privacy/accessibility suite |

## Persistence and failure behavior

External task history remains Tasks-owned. Artifact declarations may persist with user/project task settings under existing policy; native result data, if ever added, is session-only initially. Missing tools/files, failed tasks, disconnects and oversized artifacts are explicit and do not erase unrelated task output.
