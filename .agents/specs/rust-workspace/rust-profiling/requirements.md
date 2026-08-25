# Requirements: Rust profiling workflow

## Purpose and status

This pack owns the disposition of Rust profiling proposals. Ordinary explicit user Tasks and external tools are the current supported path. A small Cargo preset/artifact convenience is External-first/Next; a native flamegraph/call-tree experience is Later and requires a separate collector/protocol decision. No native Rust profiling implementation was found.

Canonical IDs are `rust-profiling/<criterion>`.

### Requirement 1: Preserve and improve explicit external profiling [External first]

#### Acceptance criteria

1. **1.1** Users SHALL remain able to define explicit profiling commands in existing Tasks/terminals without a Rust-specific process runner or replacement configuration system.
2. **1.2** IF Zed adds a Profile Cargo preset action, IT SHALL compile through existing Tasks on the authoritative project host, require explicit invocation/trust, preserve structured argv/env, and use only a user-installed/configured profiler.
3. **1.3** WHEN a profile Task declares a resulting visible SVG/HTML or supported local artifact, THE workspace SHALL offer to open it through existing safe file/URL handling without parsing terminal output.
4. **1.4** THE feature SHALL NOT bundle/install platform profilers, fetch dependencies, initiate network activity, expose secrets, use client-local fallback, or claim unsupported platform parity.
5. **1.5** Remote/multiplayer behavior SHALL keep execution/artifact generation on the authoritative host and transfer/open only bounded explicitly selected artifacts permitted by existing project policy.

### Requirement 2: Bound any later native profiling view [Later]

#### Acceptance criteria

1. **2.1** BEFORE native profiling work begins, AN approved ADR SHALL select supported collectors/platforms, stable machine-readable artifacts, licensing/distribution, remote transfer limits, cancellation and fallback.
2. **2.2** IF approved, THE native model/view SHALL be language-neutral, bounded and capable of aggregate samples, call-tree/flamegraph navigation and source locations without becoming a Rust semantic engine.
3. **2.3** Native collection SHALL run as an explicit existing Task on the authoritative host; parsing SHALL occur off the foreground thread and late/cancelled/mismatched artifacts SHALL be rejected.
4. **2.4** Tests SHALL cover supported and unsupported platforms, malformed/oversized artifacts, remote/privacy limits, cancellation, accessibility and large profiles using deterministic fixtures.

## Rejected and non-goal boundaries

Rejected for current milestones: bundled platform profiler suite, IntelliJ parity, always-on sampling, terminal parsing, automatic installation, Rust-only generic viewer, profiling in `CargoWorkspaceStore`, or declaring all platforms equivalent.

## Open questions

1. **External tool convention.** Recommended default: no blessed profiler initially; allow user-authored Tasks plus an optional artifact declaration. Task 2.1 depends on whether product wants a Cargo Profile shortcut.
2. **Native viewer threshold.** Recommended default: do not start until one redistributable/stable artifact format covers two supported platforms or a single-platform product decision is explicit. Tasks 3.1–3.2 depend on this gate.
