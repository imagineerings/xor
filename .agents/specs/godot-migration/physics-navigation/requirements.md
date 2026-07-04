# Requirements: Physics and Navigation

## Introduction

Baymax should not port Godot physics or navigation runtimes. It may index metadata and provide external simulation task fallbacks.

### Requirement 1: Physics Runtime Boundary

#### Acceptance Criteria

1.1 IF a feature requires Godot physics server or navigation server execution THEN THE system SHALL classify it as excluded or external-command only.

### Requirement 2: Metadata and Docs

#### Acceptance Criteria

2.1 WHEN physics or navigation metadata is present THEN THE system SHALL expose it for inspection and documentation lookup.
