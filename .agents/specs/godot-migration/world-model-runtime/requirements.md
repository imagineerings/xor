# Requirements: World Model Runtime

## Introduction

Sim should migrate `projects/world-model` as an external/Python-backed world foundation model harness for interactive game-world generation.

### Requirement 1: Generation Request Model

#### Acceptance Criteria

1.1 WHEN a request is created THEN THE system SHALL capture prompt, source image, controls, model profile, seed, and output target.

### Requirement 2: Camera and Action Controls

#### Acceptance Criteria

2.1 WHEN WASD/IJKL controls are provided THEN THE system SHALL validate syntax and frame-count semantics.

### Requirement 3: Persistent Sessions

#### Acceptance Criteria

3.1 WHEN fast inference is configured THEN THE system SHALL model persistent worker sessions and cache metadata.

### Requirement 4: Generated Artifacts

#### Acceptance Criteria

4.1 WHEN outputs are imported THEN THE system SHALL attach provenance.
