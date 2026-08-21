# Design: ACP Thread Prompt Reentrancy

## Overview

The crash is an ownership cycle in `AcpThread::send_inner`: its turn task updates the `AcpThread` and, from inside that update closure, invokes the selected connection prompt. Zed's native connection handles local commands synchronously and appends their transient output by updating the session's same `AcpThread`. GPUI correctly rejects that second lease.

The fix captures the connection and optional client-user-message-ID dispatcher while the turn is constructed, then starts the selected prompt task through `AsyncApp::update` without leasing `AcpThread`. The returned task is awaited by the unchanged `run_turn` future. All response, stop, error, cancellation, and persistence handling therefore remains owned by the existing flow.

## Decisions

### D1: Start connection prompts outside the thread lease

- Choice: Select and invoke the prompt implementation in an app-context update after the `AcpThread` update has returned.
- Rationale: Native local commands are allowed to synchronously update their visible `AcpThread`; the caller must not hold that entity's lease while crossing the connection boundary.
- Alternatives considered: suppressing the panic, weakening GPUI checks, deferring local output, or adding a command-specific special case. These would hide the ownership violation or fragment command behavior.
- Consequences: Prompt invocation timing remains within the same foreground task poll, but the `AcpThread` is available to connection-owned updates.

### D2: Keep the existing turn state machine intact

- Choice: Leave `run_turn`, cancellation, stop/error events, checkpoints, queued-input processing, and thread persistence unchanged.
- Rationale: The defect occurs before the connection task is awaited; those downstream owners are not the source of the nested lease.
- Consequences: Existing command and lifecycle regression suites remain applicable without new state or synchronization.

### D3: Reproduce through the real native connection

- Choice: Add a GPUI test that calls `AcpThread::send_command` with `NativeAgentConnection` and verifies `/status` output and response.
- Rationale: Direct native-command tests called the connection trait and skipped the faulty `AcpThread` caller boundary, while UI tests used a recording connection that did not update the thread.
- Consequences: The test deterministically panics before D1 and covers the production entity cycle after the fix.

## Components and flow

1. `AcpThread::send_inner` builds the request, captures the connection dispatch handles, and enters `run_turn`.
2. The turn task performs optimistic-message and checkpoint updates as before.
3. The task starts `AgentSessionClientUserMessageIds::prompt` or `AgentConnection::prompt` from the app context with no active `AcpThread` lease.
4. `NativeAgentConnection` may append local command output to the session's `AcpThread` safely.
5. `run_turn` awaits and processes the returned response through the existing completion path.

## Failure and recovery

- Prompt errors still propagate to `run_turn`, which emits the existing error state and cancels pending entries.
- Cancellation still owns the running turn task and native-thread cancellation path.
- Permission and elicitation requests still stop grind continuation through existing events and state.
- Session closure still cancels and persists through `NativeAgent`; no prompt is restarted or detached by this fix.

## Traceability

| Criterion | Design coverage | Verification type | Planned check / expected signal |
| --- | --- | --- | --- |
| 1.1 | D1 / prompt dispatch boundary | GPUI integration | Native command sent through `AcpThread` completes without `double_lease_panic` |
| 1.2 | D1, D3 / native local output | GPUI regression | `test_native_local_command_output_does_not_reenter_acp_thread` passes and output contains `## Status` |
| 1.3 | D1 / dispatch selection | Existing unit and integration regression | `acp_thread` and `agent` crate tests pass |
| 2.1 | D2, D3 / response and output | GPUI regression | Response is `EndTurn` and transient output is appended |
| 2.2 | D2 / unchanged lifecycle owners | Existing regression | `agent`, `acp_thread`, and `agent_ui` crate tests pass, including goal/grind and queued-input tests |
| 2.3 | D1 / source ownership fix | Static review | Diff changes prompt invocation boundary only; GPUI checks and command implementations are unchanged |
