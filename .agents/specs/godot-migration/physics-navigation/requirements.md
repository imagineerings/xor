# Requirements: Physics and Navigation

## Introduction

Zed should not port Godot physics or navigation runtimes by copying or delegating to them. Godot-origin physics and navigation concepts may be represented as Zed-native metadata and docs inputs; executable behavior requires an approved native Zed owner. There is no Godot server compatibility shim or fallback task that launches Godot.

### Requirement 1: Physics Runtime Boundary

#### Acceptance Criteria

1. **1.1** IF a feature requires physics or navigation execution that Zed does not natively own THEN THE system SHALL classify it as unresolved, intentionally excluded, or requiring an architecture decision.
2. **1.2** WHEN physics or navigation metadata is represented in Zed THEN THE system SHALL use records owned by existing Zed project, worktree, docs, task, or diagnostic components rather than Godot server runtime records or a parallel registry.

### Requirement 9: Native Zed Ownership

#### Acceptance Criteria

1. **9.1** Supported physics/navigation storage, execution, UI, persistence, cancellation, recovery, and lifecycle SHALL be owned by named Zed components.
2. **9.2** THE system SHALL NOT launch, embed, wrap, proxy, link, or communicate with Godot physics or navigation servers.
3. **9.3** Godot-compatible resource and property data MAY be imported, but outputs SHALL be Zed-native records/resources and supported simulation SHALL execute inside Zed.
4. **9.4** Metadata, interfaces, task records, or documentation SHALL NOT count as executable physics/navigation support.
5. **9.5** Validation SHALL run with Godot absent and inspect process, loader, package, dependency, simulation lifecycle, and deterministic outcomes.

### Requirement 2: Metadata and Docs

#### Acceptance Criteria

1. **2.1** WHEN physics or navigation metadata is present THEN THE system SHALL expose it for inspection and documentation lookup.
