# Requirements: Model Serving and Packaging

## Introduction

Sim should package world-model execution as diagnosable local, persistent, and remote worker modes without silently downloading large assets.

### Requirement 1: Environment Diagnostics

#### Acceptance Criteria

1.1 WHEN local serving is configured THEN THE system SHALL validate Python, packages, checkpoints, GPU, disk, and process settings.
1.2 IF a required component is missing THEN THE system SHALL show actionable diagnostics.

### Requirement 2: Worker Execution Modes

#### Acceptance Criteria

2.1 WHEN persistent serving is configured THEN THE system SHALL model session lifecycle, cache, and shutdown behavior.
2.2 WHEN remote serving is configured THEN THE system SHALL validate endpoint, authentication, capability, and quota metadata.

### Requirement 3: Packaging Control

#### Acceptance Criteria

3.1 IF setup requires model downloads or heavy dependencies THEN THE system SHALL require explicit user action and dependency review.
