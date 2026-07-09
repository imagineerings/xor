# Requirements: Networking and Collaboration

## Introduction

Sim should not port Godot multiplayer networking. Godot-origin networking concepts are represented only as native Sim boundary metadata or native Sim debug metadata for the generative game engine, while Sim collaboration, RPC, and debugger systems remain authoritative. There is no Godot networking compatibility shim.

### Requirement 1: Reuse Sim Networking

#### Acceptance Criteria

1.1 IF a Godot networking feature duplicates Sim networking or collaboration systems THEN THE system SHALL not port it.
1.2 WHEN Godot-origin networking metadata is represented in Sim THEN THE system SHALL use native `SimGame*` records and diagnostics rather than Godot protocol adapter records.

### Requirement 2: Debug Metadata

#### Acceptance Criteria

2.1 WHEN Godot debug metadata is available THEN THE system SHALL model it without embedding game-network runtime protocols.
