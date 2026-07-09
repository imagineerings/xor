# Design: Networking and Collaboration

## Architecture

Keep Sim collaboration, RPC, and debugger infrastructure authoritative. Add native Sim network boundary records and optional native Sim debug metadata only. Godot multiplayer, ENet, UPNP, and packet-peer semantics are source concepts, not protocol adapters.

## Components

- `SimGameNetworkBoundary`
- `SimGameDebugMetadata`

## Correctness Properties

### Property 1: No Network Runtime Port

_For any_ Godot multiplayer, ENet, UPNP, or packet-peer feature, Sim SHALL classify it as excluded unless represented as external debug metadata.

**Validates: Requirement 1.1, 1.2**

### Property 2: Native Debug Metadata

_For any_ debug endpoint metadata, Sim SHALL preserve endpoint records for task/debug workflows without embedding game-network runtime protocols.

**Validates: Requirement 2.1**
