# Requirements: Networking and Collaboration

## Introduction

Zed should not port Godot multiplayer networking. Godot-origin networking concepts are represented only as native Zed boundary metadata or native Zed debug metadata for the generative game engine, while Zed collaboration, RPC, and debugger systems remain authoritative. There is no Godot networking compatibility shim.

### Requirement 1: Reuse Zed Networking

#### Acceptance Criteria

1. **1.1** IF a Godot networking feature duplicates Zed networking or collaboration systems THEN THE system SHALL not port it.
2. **1.2** WHEN Godot-origin networking metadata is represented in Zed THEN THE system SHALL use records owned by existing Zed `net`, `http_client`, `rpc`, `collab`, `dap`, or diagnostics components rather than a parallel Godot protocol stack.

### Requirement 9: Native Zed Ownership

#### Acceptance Criteria

1. **9.1** Supported filesystem, HTTP, networking, multiplayer, collaboration, and debug behavior SHALL execute through named existing Zed owners with Zed-owned state and lifecycle.
2. **9.2** THE system SHALL NOT launch, wrap, proxy, or communicate with a Godot networking runtime or hidden Godot instance.
3. **9.3** Godot-compatible path, request, packet, RPC, and debug metadata MAY be translated at explicit boundaries, but execution SHALL remain inside Zed.
4. **9.4** Unsupported gameplay protocols SHALL remain intentionally excluded or architecture-decision blocked and SHALL NOT be claimed through metadata, an interface, or external delegation.
5. **9.5** Validation SHALL run with Godot absent and inspect process, network, package, and linkage state for delegation.

### Requirement 2: Debug Metadata

#### Acceptance Criteria

1. **2.1** WHEN Godot debug metadata is available THEN THE system SHALL model it without embedding game-network runtime protocols.
