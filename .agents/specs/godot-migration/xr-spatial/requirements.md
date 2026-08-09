# Requirements: XR and Spatial Tooling

## Introduction

Sim should not port XR runtimes. Godot-origin XR and spatial concepts are represented as native Sim spatial metadata, docs lookup inputs, and preview routes for the generative game engine. OpenXR, WebXR, and VR runtimes remain excluded; there is no XR compatibility shim.

### Requirement 1: XR Runtime Boundary

#### Acceptance Criteria

1. **1.1** IF a feature requires OpenXR, WebXR, or VR runtime migration THEN THE system SHALL classify it as excluded.
2. **1.2** WHEN XR/spatial metadata is represented in Sim THEN THE system SHALL use records owned by existing Sim project, preview, media, docs, settings, and platform components rather than an XR runtime adapter registry.

### Requirement 9: Native Sim Ownership

#### Acceptance Criteria

1. **9.1** Supported XR/spatial metadata, preview, UI, persistence, and lifecycle behavior SHALL be owned by named existing Sim components.
2. **9.2** THE system SHALL NOT launch, wrap, proxy, embed, link, or communicate with Godot or a Godot-hosted XR runtime.
3. **9.3** Godot-compatible action maps and spatial metadata MAY be imported, but outputs SHALL be Sim-native records and supported execution SHALL remain inside Sim.
4. **9.4** XR runtime behavior without an approved native Sim owner SHALL remain intentionally excluded or architecture-decision blocked.
5. **9.5** Validation SHALL run with Godot absent and inspect process, loader, package, device/runtime, and dependency state.

### Requirement 2: Spatial Metadata

#### Acceptance Criteria

1. **2.1** WHEN spatial asset metadata is available THEN THE system SHALL expose it for inspection and preview routing.
