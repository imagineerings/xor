# Requirements: ACP Thread Prompt Reentrancy

## Problem

`AcpThread` starts an agent connection prompt while the thread entity is still being updated. Zed's native local slash commands synchronously append transient output to that same `AcpThread`, so submitting commands such as `/status`, `/goal`, or `/grind` through the normal conversation path attempts a second entity update and panics. The later crash-server `BrokenPipe` warning is a consequence of the process panic, not a separate failure.

## Scope

- In scope: the `AcpThread` prompt-dispatch ownership boundary, Zed-native local slash commands, and regression coverage through the real native connection.
- In scope: preservation of prompt completion, transient command output, queued input, cancellation, permission/elicitation, persistence, and session-close behavior.
- Out of scope: changing slash-command syntax or behavior, external ACP command catalogs, GPUI entity-map checks, or introducing a new dispatcher or state machine.

## Requirements

### Requirement 1: Non-reentrant prompt dispatch

**System outcome:** Agent prompt dispatch must allow a connection to update its owning thread without re-entering an active entity update.

#### Acceptance criteria

1. WHEN `AcpThread` starts an agent prompt THEN THE system SHALL invoke the connection prompt only after the active `AcpThread` update boundary has ended.
2. WHEN a Zed-native local command appends transient output during prompt dispatch THEN THE system SHALL update the `AcpThread` exactly once without a re-entrant entity update panic.
3. THE system SHALL preserve the existing client-user-message-ID prompt path and the fallback `AgentConnection::prompt` path.

### Requirement 2: Command and lifecycle preservation

**User story:** As a Zed user, I want native commands to retain their existing behavior after the crash fix, so that output, continuation, and control boundaries remain predictable.

#### Acceptance criteria

1. WHEN a local native command completes THEN THE system SHALL return its existing prompt response and retain its transient command output.
2. WHEN a prompt is cancelled, requires permission or elicitation, fails, closes, or completes with queued input THEN THE system SHALL retain the existing cancellation, attention, persistence, session-close, and queued-input flows.
3. THE system SHALL NOT catch or suppress the GPUI panic, weaken entity-map checks, or defer command execution beyond the normal prompt task lifecycle.

## Constraints

- Reuse `AcpThread::run_turn`, the existing `AgentConnection` traits, `NativeAgent`, `Thread`, and `ThreadStore`.
- Use the app context outside the entity lease to start the prompt task.
- Do not change documented `/goal`, `/grind`, or developer-command semantics.

## Open questions

None. The existing entity ownership model and prompt task lifecycle determine the safe boundary.
