# Design: Physics and Navigation

## Architecture

Represent physics/navigation files and docs through existing Sim project, worktree, docs, task, and diagnostic owners. Runtime behavior remains excluded or decision-blocked until an approved existing or proposed Sim runtime owner executes it natively. Godot physics and navigation servers are reference concepts only and are neither embedded nor delegated to.

## Components

- Existing project/worktree metadata and diagnostics for imported resources.
- Existing task/docs surfaces for native owner diagnostics and explicit unsupported outcomes.

## Correctness Properties

### Property 1: Runtime Exclusion

_For any_ Godot physics or navigation runtime feature, Sim SHALL not embed the runtime implementation.

**Validates: Requirement 1.1, 1.2**

### Property 2: Native Metadata

_For any_ indexed physics or navigation source, Sim SHALL expose docs symbols and metadata without treating a task/interface declaration as runtime execution.

**Validates: Requirement 2.1**

### D-NATIVE: Native simulation path

The compatibility boundary accepts resource/property data into Sim-owned records. Executable support requires an approved native runtime owner with deterministic lifecycle tests; otherwise the behavior remains excluded or decision-blocked. No fallback launches Godot.

**Validates: Requirement 9.1, 9.2, 9.3, 9.4, 9.5**


## Audit traceability reconciliation

### D-TRACE: Preserve legacy design while exposing complete criterion coverage

This reconciliation table preserves the existing design decisions and IDs while making every acceptance criterion visible to the current feature-spec validator. Capability-level gaps and ownership corrections are defined by the Godot full-port coverage catalog.

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 1.2 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
| 2.1 | D-TRACE and existing design properties | Owner-spec scenario and failure-path validation |
