# Design: Platform and Export

## Architecture

Use existing Sim task, project, settings, platform, and bundling integration points for approved native packaging and deployment behavior. Godot export presets are parsed as compatibility input only; Sim owns requests, execution, artifacts, diagnostics, cancellation, cleanup, and dependency review. Unsupported targets remain explicit and are never implemented by launching Godot.

## Components

- Existing project/settings persistence for imported preset data.
- Existing task lifecycle for Sim-owned packaging and deployment work.
- Existing platform bundling/signing owners selected by an approved platform-tier decision.

## Correctness Properties

### Property 1: Native Sim Export Task

_For any_ supported export request, Sim SHALL create and execute a native packaging request through existing Sim owners and SHALL NOT migrate or invoke Godot platform templates.

**Validates: Requirement 1.1, 1.2, 2.1**

### D-NATIVE: Native export path

The compatibility boundary ends after preset parsing. Sim-owned task/platform services perform packaging, signing, deployment, cancellation, artifact collection, and cleanup. Hermetic validation runs the exported artifact without Godot and inspects its contents and dependencies.

**Validates: Requirement 2.2, 9.1, 9.2, 9.3, 9.4, 9.5**


## Audit traceability reconciliation

### D-TRACE: Preserve legacy design while exposing complete criterion coverage

This reconciliation table preserves the existing design decisions and IDs while making every acceptance criterion visible to the current feature-spec validator. Capability-level gaps and ownership corrections are defined by the Godot full-port coverage catalog.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 1.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
