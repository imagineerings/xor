# Design: Goal and Grind Commands

## Overview

This feature extends Sim's current native-agent command and turn paths. `Thread` gains one persisted optional goal and one transient grind-turn control context. `NativeAgent::Session` gains one transient active-grind record. `/goal` mutations are persisted atomically before their local acknowledgement; `/grind` runs a foreground bounded loop that calls the same `Thread::resume` and event-forwarding machinery used by normal native turns.

The design intentionally separates durable intent from temporary spending consent:

- `Thread.goal` is serialized in `DbThread` and survives reload.
- `Session.active_grind` contains invocation identity, authorized limit, completed-turn count, and cancellation state; it is never serialized and is absent after reopen.
- Each actual continuation is represented by the existing persisted `Message::Resume` marker, while the goal/control instruction is added only to that turn's model request.
- A grind-only native tool lets the model explicitly report satisfaction without a second provider request or parsing user-visible text.

## Audit findings

Goose exposes `/goal` and `/grind` from its built-in command catalog and supports `off`, `clear`, and `none`. Its legacy and state-machine implementations store goal/grind values on the agent, hide command acknowledgements from model context, inject model-only continuation messages, and rely on the agent's maximum-turn loop. The audited grind nudge repeats whenever the model ends without tools; it does not provide Sim's required persisted per-thread goal, per-invocation consent record, hard limit of twenty, reload non-resumption, or immediate stop-on-permission/tool-failure contract.

Sim already owns every necessary boundary: catalog and dispatch in `NativeAgent`, persisted messages and tool results in `Thread`, transient presentation in `AcpThread`, cancellation in `Thread::cancel`, tool authorization in `handle_thread_events`, queued input in `ThreadView`, and final storage in `ThreadStore`. No Goose state-machine or execution-manager type is needed.

## Design elements

### D1: Extend the single native command source

`NATIVE_COMMANDS` remains the built-in source for native command names, descriptions, collision reservation, active-session available-command updates, and autocomplete. Represent whether a native command consumes unstructured arguments in this catalog definition rather than adding a UI list.

`/goal` and `/grind` are advertised as native commands with unstructured input hints. `Command::parse` remains the only native dispatch parser. Small helpers interpret goal forms and the exact grind grammar after dispatch:

- `/goal` -> show;
- `/goal clear|off|none` -> clear;
- every other non-empty remainder -> set/replace;
- `/grind` -> default limit 5;
- `/grind max_turns=<decimal>` -> checked limit 1..=20;
- all other grind remainders -> visible local validation failure.

Unqualified native names continue to win collisions. MCP prompts and skills use their existing qualification rules. External ACP catalogs do not call this builder and remain untouched.

### D2: Store goal with Thread and persist before acknowledgement

Add `goal: Option<SharedString>` to `Thread` and a `#[serde(default)]` counterpart to `DbThread`. `Thread::from_db` and `to_db` round-trip the field. Goal state is included when deciding whether a native thread has persistable state, so a goal-only session can survive reload.

Goal mutations use a narrow command path modeled on atomic `/clear` persistence:

1. require the session to exist, be idle, and have no active grind;
2. snapshot the current `DbThread` and draft through the existing owners;
3. order behind `Session.pending_save`;
4. write the modified goal with `ThreadStore::save_thread`, including the empty/no-goal form when clearing a previously persisted goal;
5. confirm the same session is still eligible;
6. update live `Thread.goal` and append transient local output.

This avoids acknowledging a goal that failed to persist and avoids leaving stale goal-only database state after clear. Ordinary later thread saves carry the goal automatically. `/clear` calls `DbThread::clear_conversation` and `Thread::clear_conversation` without changing the goal.

### D3: Reuse Thread turns with transient control context

`Thread` receives a transient optional grind-turn context containing the active goal and a shared satisfaction flag. It is configured immediately before `Thread::resume` and cleared when that turn stops or is cancelled. It is not part of `DbThread`.

While this context exists:

- `Thread::enabled_tools` inserts one native goal-satisfaction tool regardless of profile tool allowlists, because the user explicitly enabled it by invoking `/grind`;
- the tool remains subject to provider tool support and never requests filesystem, network, trust, sandbox, or user permission;
- `Thread::build_completion_request` appends a model-only control message identifying the goal, remaining bounded context, and instruction to call the tool only after the goal is fully satisfied;
- `Thread::resume` persists its existing `Message::Resume` marker and all normal assistant/tool result messages;
- the satisfaction tool sets the shared flag and returns a normal successful tool result, so its call/result are retained by the existing message and replay path.

After the turn completes, the driver reads and clears the satisfaction flag. A report counts only when the enclosing turn itself completes successfully; cancellation or later failure remains a failure termination.

### D4: Keep bounded consent in NativeAgent::Session

Each open native session may hold at most one `ActiveGrind`:

- unique invocation ID;
- authorized `max_turns`;
- `completed_turns`;
- cancellation-requested flag.

Starting `/grind` validates arguments and goal first, synchronously installs `ActiveGrind`, and appends a transient acknowledgement that displays the authorized limit before the first provider request. The returned ACP prompt task owns the foreground loop. It never detaches the provider loop.

For each iteration the driver:

1. verifies that the same session and invocation still exist and are not cancelled;
2. stops if `completed_turns == max_turns`;
3. installs one `Thread` grind-turn context and calls `Thread::resume` through the native turn event path;
4. increments completed progress only for a provider turn that actually started;
5. interprets the collected turn outcome;
6. stops on satisfaction or any termination boundary; otherwise rechecks session/cancellation state before the next iteration.

Cleanup compares the invocation ID before removing state, preventing stale async cleanup from clearing a later grind. Progress output is derived from `ActiveGrind`; hidden control-message content is never formatted by `/status`.

### D5: Extend the existing event forwarder with a bounded policy

Do not introduce a second turn engine. Factor or parameterize `NativeAgentConnection::handle_thread_events` so normal turns retain current behavior and grind turns additionally collect:

- whether a tool update reached `Failed`;
- whether a `ToolCallAuthorization` or elicitation/user-attention event occurred;
- the final ACP stop reason;
- provider/event-stream errors.

On tool failure or user-attention request, forward the existing ACP event first so the record/request remains visible, mark the grind termination, and call the existing `Thread::cancel` path. The response to a later permission selection may update the visible request, but it cannot recreate the cleared grind invocation. Provider errors and non-success stop reasons are returned without another continuation.

Because `Thread` emits tool updates asynchronously, a grind turn also checks finalized tool results before selecting the next provider round. A failed result is flushed through the normal message/action persistence path and ends that grind turn at the existing tool-result boundary, while the event forwarder records the invocation stop reason. While a grind turn awaits tool results, the existing cancellation receiver remains part of the wait so Stop or an attention-triggered cancellation can terminate automatic continuation without waiting for a permission response or a long-running tool.

Normal, non-grind event handling and tool-cancellation cleanup remain unchanged.

### D6: Cancel before unload and persist final records

`NativeAgentConnection::cancel` first marks the active grind invocation cancelled, then calls the existing `Thread::cancel`. This covers provider/tool execution and the gap between turns. The loop checks the flag before every continuation.

Final session close performs the same marking and cancellation. When an active grind exists, close waits for turn cancellation/flush before taking the final `Thread::to_db` snapshot and saving it through the current database/`ThreadStore` path. It then removes the session and transient grind state. Ordinary non-grind close retains its existing path.

Loading or reopening constructs `Session { active_grind: None }` regardless of persisted goal. `pending_sessions` remains the sole load-coalescing mechanism and no continuation is scheduled from `from_db` or `register_session`.

### D7: Keep command and progress output outside model context

Goal show/set/clear results, grind validation/acknowledgement/progress/termination, and `/status` are appended with the existing `AcpThread::push_local_command_output` path. They never add a `Thread::Message`.

`/status` adds:

- `Goal: <text>` or `Goal: none`;
- `Grind: inactive` or `Grind: active (<completed>/<max> turns completed)`.

It does not include the internal grind control instruction, satisfaction-tool schema, provider prompts, reasoning, or command history.

### D8: Use catalog input metadata to preserve both argument and queue behavior

`ThreadView` continues to inspect the active `AvailableCommand` catalog. For a native command with unstructured input metadata, it submits the complete resolved content as one command turn with command presentation semantics; it does not strip and queue the text argument. For an argumentless native command, it retains `send_command_queueing_remainder` exactly as today.

This makes `/goal build the index` and `/grind max_turns=8` reach `Command::parse` intact while preserving `/compact then explain it` as bare `/compact` plus a queued ordinary prompt. Unknown-command validation and rich editor preservation remain owned by `MessageEditor` and the current conversation error path.

## State and sequence

```text
/goal <text>
  Command::parse -> persist DbThread.goal -> Thread.goal -> transient acknowledgement

/grind [max_turns=n]
  Command::parse -> validate goal/limit/idle -> Session.active_grind
  -> transient consent acknowledgement
  -> repeat at most n times:
       Thread grind context -> Thread::resume -> existing provider/tools/persistence
       -> satisfaction / failure / attention / cancel / close / continue
  -> clear matching active_grind -> transient termination output
```

## Failure semantics

| Boundary | Required result | Later automatic turn? |
| --- | --- | --- |
| Missing goal or invalid grind arguments | Transient actionable failure; no thread mutation or provider call | No |
| Concurrent grind | Keep current invocation; reject new command | Existing invocation only |
| Goal persistence failure | Keep live goal unchanged; surface command failure | No |
| Provider/event-stream error | Preserve finalized messages; clear grind state; surface existing error plus transient stop status when possible | No |
| Refusal or max tokens | Preserve recorded turn; clear grind state | No |
| Tool failure | Forward failed action record, cancel current turn, clear grind | No |
| Permission or elicitation | Surface existing request, cancel current turn, clear grind | No, including after response |
| User Stop | Mark invocation cancelled before calling `Thread::cancel` | No |
| Limit reached | Report completed/authorized bound | No |
| Session/ACP disappearance | End task without replacement output or session recreation | No |
| Final close | Cancel, flush, snapshot, save, unload | No |

## Persistence rules

| State | Persisted owner | Reload behavior |
| --- | --- | --- |
| Goal | `DbThread.goal` via `Thread::to_db` and `ThreadStore` | Restored |
| Actual continuation marker | Existing `Message::Resume` | Restored in model history; not replayed as a visible user entry |
| Assistant messages and tool calls/results | Existing `Thread.messages` and action paths | Restored/replayed normally |
| Draft, title, model, profile, usage | Existing `DbThread` fields | Unchanged |
| Edit/action records | Existing `ActionLog`/thread owners | Preserved; never cleared by goal/grind or `/clear` |
| Active grind ID/progress/cancellation | `NativeAgent::Session.active_grind` | Never restored |
| Hidden grind control context | Transient `Thread` grind-turn context/request message | Never serialized |
| Command acknowledgement/status | `AcpThread` local output | Never model context; not required after reload |

## Traceability

| Criterion | Design coverage | Verification type | Planned check / expected signal |
| --- | --- | --- | --- |
| 1.1 | D1, D2, D7 | Integration | Set or replace persists before transient success and makes zero provider calls. |
| 1.2 | D1, D7 | Integration | Show returns current or absent state with zero provider calls. |
| 1.3 | D1, D2, D7 | Integration | Each exact clear alias persists absence and acknowledges locally. |
| 1.4 | D1 | Unit | Whitespace shows; a clear keyword with extra text becomes the full goal. |
| 1.5 | D2, D4 | Integration | Active grind rejects goal mutation without changing durable state. |
| 1.6 | D1 | Regression | Native builder owns both names; external ACP catalog is unchanged. |
| 2.1 | D1, D4 | Integration | Bare grind displays and enforces five turns. |
| 2.2 | D1, D4 | Boundary | Valid overrides 1 and 20 enforce their exact counts. |
| 2.3 | D1, D4, D7 | Negative | Missing goal and every invalid grammar/bound make zero provider calls. |
| 2.4 | D4 | Concurrency | A second invocation is rejected while exactly one loop runs. |
| 2.5 | D3 | Integration | Resume, transient request control, and grind-only tool appear for one turn. |
| 2.6 | D3, D4 | Integration | Satisfaction tool plus successful turn stops before another request. |
| 2.7 | D4 | Integration | Unsatisfied successful turn starts exactly one next turn while budget remains. |
| 2.8 | D4 | Boundary | Completed count reaches limit and no later provider request starts. |
| 2.9 | D4, D6 | Lifecycle | Prompt task owns loop; no detach, implicit increase, reopen, or auto-restart. |
| 3.1 | D4, D5, D6 | Cancellation | Stop at provider, tool, post-turn, and pre-next-turn gates prevents continuation. |
| 3.2 | D5 | Failure | Provider error/refusal/max-token/cancel returns terminal outcome once. |
| 3.3 | D5 | Failure | Failed tool record is forwarded/persisted before existing cancellation ends grind. |
| 3.4 | D5 | Permission | Permission/elicitation is surfaced and later response cannot restart grind. |
| 3.5 | D6 | Lifecycle | Close cancels, flushes, final-saves, unloads, and makes no next request. |
| 3.6 | D4, D6 | Failure | Missing session/ACP entity ends the task without recreation or false success. |
| 3.7 | D4, D6 | State transition | Every terminal outcome clears only the matching invocation and permits explicit reinvoke. |
| 4.1 | D2 | Persistence | Set/replace/reload returns identical goal through goal and status output. |
| 4.2 | D2 | Persistence | Goal-only clear/reload restores absence rather than stale database state. |
| 4.3 | D4, D6 | Persistence | Reopen restores goal with inactive progress and zero automatic calls. |
| 4.4 | D3, D5, D6 | Persistence | Resume, assistant, tool result, and action records replay through existing owners. |
| 4.5 | D2, D6 | Regression | Clear removes conversation but retains goal and action records. |
| 4.6 | D3, D7 | Context isolation | Captured later model request contains no command or local output. |
| 4.7 | D4, D7 | Snapshot | Status shows goal and completed/limit only, excluding control prompt. |
| 4.8 | D2, D6 | Persistence | Quit/final close stores latest goal, messages, draft, and finalized actions. |
| 5.1 | D1 | Catalog | Goal/grind appear once with native category/input and collision qualification. |
| 5.2 | D1, D8 | UI integration | Existing available-command completion provider supplies both names. |
| 5.3 | D1, D8 | UI integration | Full argument-bearing text is sent once as a command and never queued. |
| 5.4 | D8 | Regression | Argumentless command retains rich queued remainder ordering and failure behavior. |
| 5.5 | D8 | Regression | Unknown rich input remains editable with zero dispatch/model calls. |
| 5.6 | D1, D8 | Regression | Skill/MCP dispatch and external ACP advertisements remain unchanged. |

## Requirements traceability

This compatibility table mirrors the detailed traceability table for the migration feature-spec validator.

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | D1, D2, D7 | Goal set persistence and zero-model test. |
| 1.2 | D1, D7 | Goal show test. |
| 1.3 | D1, D2, D7 | Goal clear aliases and reload test. |
| 1.4 | D1 | Goal whitespace/parser test. |
| 1.5 | D2, D4 | Active-grind mutation rejection test. |
| 1.6 | D1 | Native/external catalog ownership test. |
| 2.1 | D1, D4 | Default-bound test. |
| 2.2 | D1, D4 | Override-bound test. |
| 2.3 | D1, D4, D7 | Invalid/missing argument tests. |
| 2.4 | D4 | Concurrent grind test. |
| 2.5 | D3 | Grind turn request/tool/persistence test. |
| 2.6 | D3, D4 | Satisfaction termination test. |
| 2.7 | D4 | Unsatisfied continuation test. |
| 2.8 | D4 | Limit termination test. |
| 2.9 | D4, D6 | Foreground consent/restart lifecycle test. |
| 3.1 | D4, D5, D6 | Cancellation boundary tests. |
| 3.2 | D5 | Provider termination tests. |
| 3.3 | D5 | Tool-failure termination test. |
| 3.4 | D5 | Permission/elicitation termination test. |
| 3.5 | D6 | Close cancellation/final-save test. |
| 3.6 | D4, D6 | Entity disappearance test. |
| 3.7 | D4, D6 | Cleanup and reinvoke test. |
| 4.1 | D2 | Goal reload test. |
| 4.2 | D2 | Goal-only clear reload test. |
| 4.3 | D4, D6 | Inactive reopen/no-restart test. |
| 4.4 | D3, D5, D6 | Turn/tool/action persistence test. |
| 4.5 | D2, D6 | Clear preservation test. |
| 4.6 | D3, D7 | Model-context exclusion test. |
| 4.7 | D4, D7 | Status progress/redaction test. |
| 4.8 | D2, D6 | Quit/final-close persistence test. |
| 5.1 | D1 | Native catalog/collision test. |
| 5.2 | D1, D8 | Autocomplete integration test. |
| 5.3 | D1, D8 | Argument-consuming submission test. |
| 5.4 | D8 | Queued remainder regression. |
| 5.5 | D8 | Unknown-command regression. |
| 5.6 | D1, D8 | Skill/MCP/external ACP regression. |

## Testing strategy

- Unit-test exact goal and grind argument parsing, including whitespace, aliases, bounds, overflow, and unexpected tokens.
- Test `DbThread` backward-compatible deserialization, goal-only save, replace/clear/reload, clear-preserves-goal, and inactive state after reopen.
- Use the existing fake language model and test tools to drive satisfaction, no-satisfaction bounds, provider errors, tool failures, permission, cancellation, and session close.
- Add boundary hooks or test-only gates at provider start, tool execution, turn completion, and before-continuation checks so GPUI cancellation tests are deterministic; use GPUI executor timers if a timeout is needed.
- Extend native command catalog, collision, status, local-output/model-context, app-quit, final-close, and action-record tests in `agent`.
- Extend native command submission/autocomplete/unknown/queue regressions in `agent_ui`.
