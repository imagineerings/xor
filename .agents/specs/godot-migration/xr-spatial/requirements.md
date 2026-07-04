# Requirements: XR and Spatial Tooling

## Introduction

Baymax should not port XR runtimes. It may index XR/spatial metadata and docs where useful for authoring.

### Requirement 1: XR Runtime Boundary

#### Acceptance Criteria

1.1 IF a feature requires OpenXR, WebXR, or VR runtime migration THEN THE system SHALL classify it as excluded.

### Requirement 2: Spatial Metadata

#### Acceptance Criteria

2.1 WHEN spatial asset metadata is available THEN THE system SHALL expose it for inspection and preview routing.
