# Requirements: Physics and Navigation

## Introduction

Sim should not port Godot physics or navigation runtimes. Godot-origin physics and navigation concepts are represented as native Sim metadata, docs lookup inputs, and optional native Sim fallback task records for the generative game engine. There is no Godot server compatibility shim.

### Requirement 1: Physics Runtime Boundary

#### Acceptance Criteria

1.1 IF a feature requires Godot physics server or navigation server execution THEN THE system SHALL classify it as excluded or external-command only.
1.2 WHEN physics or navigation metadata is represented in Sim THEN THE system SHALL use native `SimGame*` records and diagnostics rather than Godot server runtime records.

### Requirement 2: Metadata and Docs

#### Acceptance Criteria

2.1 WHEN physics or navigation metadata is present THEN THE system SHALL expose it for inspection and documentation lookup.
