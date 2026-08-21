# Requirements: Unified Authoring App

## Introduction

Zed should provide a unified cross-platform game authoring application for game project assets, diffusion graphs, world-model runtime previews, and generated artifacts.

### Requirement 1: Unified Workspace

#### Acceptance Criteria

1. **1.1** WHEN a game project opens THEN THE system SHALL present project assets, graphs, world-model requests, generated artifacts, and run/export tasks in one workspace model.
2. **1.2** WHEN an item is selected THEN THE system SHALL route it to the correct editor, preview, inspector, or task view.

### Requirement 2: Runtime Preview

#### Acceptance Criteria

1. **2.1** WHEN world-model preview is requested THEN THE system SHALL use worker diagnostics and generated artifact provenance.
2. **2.2** IF preview cannot run THEN THE system SHALL show actionable diagnostics.

### Requirement 9: Native Zed Ownership

#### Acceptance Criteria

1. **9.1** Supported workspace, editor, preview, inspector, task, artifact, persistence, cancellation, recovery, and lifecycle behavior SHALL be owned by existing Zed workspace/project/editor/media/task components.
2. **9.2** THE authoring app SHALL NOT embed, launch, wrap, proxy, link, or communicate with a Godot editor, engine, runtime, server, library, or command-line tool.
3. **9.3** Godot-compatible project, scene, resource, command, preview, run, and export concepts MAY appear at explicit interoperability boundaries, but UI state and execution SHALL remain Zed-owned.
4. **9.4** The app SHALL compose existing owners rather than create a parallel Godot-specific workspace, preview router, inspector, task system, artifact registry, or runtime.
5. **9.5** Every supported or fully specified authoring capability SHALL validate with Godot absent and inspect process, package, loader, runtime dependency, persistence, and lifecycle state.
