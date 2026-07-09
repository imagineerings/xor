# Requirements: XR and Spatial Tooling

## Introduction

Sim should not port XR runtimes. Godot-origin XR and spatial concepts are represented as native Sim spatial metadata, docs lookup inputs, and preview routes for the generative game engine. OpenXR, WebXR, and VR runtimes remain excluded; there is no XR compatibility shim.

### Requirement 1: XR Runtime Boundary

#### Acceptance Criteria

1.1 IF a feature requires OpenXR, WebXR, or VR runtime migration THEN THE system SHALL classify it as excluded.
1.2 WHEN XR/spatial metadata is represented in Sim THEN THE system SHALL use native `SimGame*` records and diagnostics rather than XR runtime adapter records.

### Requirement 2: Spatial Metadata

#### Acceptance Criteria

2.1 WHEN spatial asset metadata is available THEN THE system SHALL expose it for inspection and preview routing.
