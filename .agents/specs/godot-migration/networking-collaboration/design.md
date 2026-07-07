# Design: Networking and Collaboration

## Architecture

Keep Sim collaboration, RPC, and debugger infrastructure authoritative. Add optional Godot debug metadata and task fallback models only.

## Components

- `SimGameNetworkBoundary`
- `SimGameDebugMetadata`

## Correctness Properties

### Property 1: No Network Runtime Port

_For any_ Godot multiplayer, ENet, UPNP, or packet-peer feature, Sim SHALL classify it as excluded unless represented as external debug metadata.

**Validates: Requirement 1.1**
