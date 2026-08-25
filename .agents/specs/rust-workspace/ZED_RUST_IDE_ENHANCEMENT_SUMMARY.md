# Zed Rust IDE Enhancement Summary

**Evaluation date:** 2026-08-13  
**Reference:** RustRover 2026.2 (build 262.8665.323), Zed v1.15.0 stable, Zed `main` `0307288d...`.

## Ten most important findings

1. **Do not rebuild Rust intelligence.** Zed's correct semantic foundation is rust-analyzer; most high-value gaps are orchestration, configuration and presentation.
2. **Zed already has the right execution primitives.** Tasks, terminals, DAP/debugger infrastructure, project/worktree services, remote execution and worktree trust should remain owners of execution.
3. **Cargo state is the missing connective tissue.** A cached worktree-scoped Cargo model can power dashboard, presets, tests, dependency insight, coverage and profiling without separate subsystems.
4. **Run/debug configuration is capable but fragmented.** A typed Cargo execution spec should compile into existing Tasks/DAP rather than replace their JSON/configuration systems.
5. **Structured tests are the largest everyday UX gap.** RustRover's outcome—hierarchical status/duration/output/rerun/debug—is valuable, but Zed should implement the result model generically and use Rust as the first provider.
6. **Test protocol stability is a hard gate.** Do not ship a brittle parser chosen from one Cargo/libtest/nextest version; conduct the protocol spike first and degrade to terminal tasks when structure cannot be trusted.
7. **Coverage should come after generic overlays/results.** `cargo llvm-cov` is a promising collector, but Rust-specific gutter rendering would be architectural debt; source-analysis overlays should be reusable.
8. **Call hierarchy is a generic LSP gap, not a Rust feature.** Existing Zed issue #14203 supports implementing standard LSP call hierarchy once and letting rust-analyzer provide Rust data.
9. **Remote and trust are architectural constraints, not follow-up work.** Cargo/test/debug/coverage commands execute where the project lives and must respect restricted worktrees from the first PR.
10. **Do not chase broad IntelliJ parity.** Database browsers, HTTP clients, native profiler suites and framework-specific semantic engines are poor first-roadmap investments; use external tools/extensions or defer them.

## Top three recommendations

### 1. Worktree-scoped Rust project model + Cargo dashboard — Now

Highest leverage: it creates one cached Cargo workspace/target/features/toolchain view and becomes the shared source for every later Rust workflow while fitting Zed's project/task/remote architecture.

### 2. Unified Cargo execution/configuration presets — Now

Highest workflow multiplier: package/target/profile/features/args/env/toolchain state is authored once, then resolves through existing Run/Test Tasks and DAP Debug instead of being duplicated across JSON and commands.

### 3. Structured Rust test explorer backed by generic execution results — Now

Highest direct daily-user value: developers gain test hierarchy, status, duration, output, navigation, rerun-failed and debug while Zed gains a language-neutral result platform reusable beyond Rust.

## Recommended implementation order

1. Pin implementation SHA and trace exact current Zed types/actions.
2. Build/validate representative Rust fixture and complete the structured-test protocol spike.
3. Land worktree Rust project model with trust/remote/cache tests, no UI.
4. Add compact Cargo dashboard that generates existing Tasks.
5. Land typed Cargo execution spec and task/DAP resolution.
6. Add ephemeral then persisted presets.
7. Land generic structured execution-result model/UI.
8. Add Rust test provider.
9. Cross-platform/remote/performance/security hardening.
10. Next: generic LSP call hierarchy, generic coverage overlays + Rust collector, dependency insight.

## Most important feature not to add

**Do not add a second Rust semantic/indexing engine or an IntelliJ-style monolithic Rust subsystem.** It would duplicate rust-analyzer, compete for Cargo/proc-macro/build resources, complicate remote execution and create long-term semantic divergence. Zed should expose/orchestrate existing standards and tools instead.

## Major risks

- Machine-readable test protocols may be incomplete or unstable across the supported Rust matrix.
- A Cargo metadata cache can accidentally create process storms if invalidation is too broad.
- UI-heavy capabilities may require Zed core because the current extension API may not expose arbitrary panel/overlay primitives; verify at implementation SHA.
- Project-shared presets can leak secrets if environment serialization is naïve.
- DAP features vary substantially by adapter/platform; Rust UX must capability-gate instead of promising unsupported operations.
- Windows as an SSH remote server is currently outside Zed's documented support; acceptance must not imply otherwise.

## Human product decisions

- Cargo dashboard placement: Project panel mode, small dedicated panel, or modal/command-first.
- Project preset schema/location and which fields are shareable by default.
- Session-only versus persisted test history for v1.
- Initial coverage surface: gutter/summary only or a generic analysis panel.

## Detailed specifications

- [Research and comparison matrix](research.md)
- [Specification catalog](README.md)
- [Architecture/design](design.md)
- [Sequenced implementation tasks](tasks.md)
