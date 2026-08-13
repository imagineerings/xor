# Requirements: Goal and Grind Commands

## Problem

Sim's native agent can run and cancel one persisted conversation turn, but it has no native way to retain a user-defined objective or to grant bounded consent for automatic continuation. Goose demonstrates useful `/goal` and `/grind` concepts, but its process-global goal fields and effectively unconditional grind nudges do not provide the persistence, restart, spending, permission, and lifecycle guarantees required by Sim.

Users need a Sim-native session goal and an explicitly bounded grind loop that reuse the current command, thread, tool, cancellation, and persistence owners. Command acknowledgements must remain local UI state, while every real model turn and tool/action record remains part of the ordinary persisted thread.

## Scope

### In scope

- Sim-native `/goal` set, show, replace, and clear forms.
- Sim-native `/grind` with a default five-turn limit and an explicit `max_turns=<n>` override capped at twenty.
- Goal persistence with `Thread` and `ThreadStore`; transient grind progress with the open `NativeAgent` session.
- Goal-satisfaction reporting through a grind-only native tool attached to the existing `Thread` turn path.
- Existing cancellation, tool/action persistence, permission, provider, session close/reload, autocomplete, and queued-input owners.
- `/status` goal and grind progress fields.

### Out of scope

- Commands advertised by external ACP agents.
- A second slash-command parser, registry, execution manager, or conversation state machine.
- Background, scheduled, unbounded, or automatically restarted provider work.
- Recipe runtime or recipe-backed commands.
- `/doctor`, terminal-only commands, MCP Apps, `/help`, or unrelated Goose command parity.
- Changes to Sim instruction, skill, project-root, trust, sandbox, or path-security behavior.

## Glossary

- **Goal:** The optional user-authored objective persisted with one native-agent thread.
- **Grind:** One foreground, user-authorized sequence of automatically continued native-agent turns for the current goal.
- **Turn limit:** The maximum number of model turns authorized by one `/grind` invocation, inclusive of the first grind turn.
- **Goal satisfaction report:** A call to a native, grind-only tool by which the model declares that the active goal is satisfied.
- **User attention:** A tool permission or elicitation that requires a user response before work can proceed.
- **Transient output:** A local command acknowledgement or status entry displayed through `AcpThread` but absent from `Thread` messages and later model requests.

## Requirements

### Requirement 1: Native goal command

**User story:** As a Sim user, I want a persistent goal for one native-agent session, so that I can inspect and reuse the objective without placing command text in model context.

#### Acceptance criteria

1. **1.1** WHEN `/goal <text>` is submitted to Sim's `NativeAgent`, THEN THE system SHALL trim the argument, set or replace the session goal with the remaining non-empty text, persist it through the existing thread persistence path, and show a transient confirmation without invoking the model.
2. **1.2** WHEN `/goal` is submitted without an argument, THEN THE system SHALL show the current goal or explicitly report that no goal is set, without invoking the model.
3. **1.3** WHEN `/goal clear`, `/goal off`, or `/goal none` is submitted as the complete argument, THEN THE system SHALL clear and persist the goal and show a transient confirmation without invoking the model.
4. **1.4** IF `/goal` receives only whitespace after the command token, THEN THE system SHALL treat it as the show form; IF a clear keyword has additional text, THEN THE system SHALL treat the complete non-empty argument as a replacement goal rather than partially interpreting it.
5. **1.5** WHILE a grind is active, THE system SHALL reject a direct goal mutation until that grind terminates, without changing the persisted goal or starting another model turn.
6. **1.6** THE goal command SHALL be owned only by the existing native command catalog, `Command::parse` dispatch, `NativeAgent`, `Thread`, and `ThreadStore`; external ACP command catalogs SHALL remain unchanged.

### Requirement 2: Bounded grind execution

**User story:** As a Sim user, I want to authorize bounded automatic continuation toward my current goal, so that the agent can keep working without creating unbounded or background provider spending.

#### Acceptance criteria

1. **2.1** WHEN `/grind` is submitted with an active goal and no argument, THEN THE system SHALL display that up to five turns are authorized and SHALL start one foreground grind using that limit.
2. **2.2** WHEN `/grind max_turns=<n>` is submitted with an active goal and an integer from 1 through 20, THEN THE system SHALL display that exact limit and SHALL use it for only that invocation.
3. **2.3** IF `/grind` has no active goal, has an unknown argument, has multiple arguments, has a non-integer value, has zero, or exceeds 20, THEN THE system SHALL show an actionable transient failure, invoke no model, and leave goal and grind state unchanged.
4. **2.4** WHILE one grind is active for a session, THE system SHALL reject another `/grind` invocation and SHALL never run two continuation loops for that session.
5. **2.5** WHEN a grind turn begins, THEN THE system SHALL use the existing `Thread` turn path, persist a continuation marker through the ordinary message path, expose the active goal to that request through transient model-only control context, and make a native goal-satisfaction tool available only for that grind turn.
6. **2.6** WHEN the model reports goal satisfaction through the grind-only tool and the current turn completes successfully, THEN THE system SHALL stop the grind before another provider request and show a transient satisfied status.
7. **2.7** IF a completed grind turn does not report satisfaction and the turn limit remains, THEN THE system SHALL start the next turn without requiring another user message.
8. **2.8** WHEN the configured turn limit is reached without satisfaction, THEN THE system SHALL stop before another provider request and show the completed-versus-authorized bound.
9. **2.9** THE `/grind` invocation SHALL constitute consent only for the displayed turn limit; THE system SHALL NOT continue in the background, raise the limit implicitly, or start a later grind without a new invocation.

### Requirement 3: Failure, cancellation, and attention boundaries

**User story:** As a Sim user, I want a grind to stop safely at every failure or authorization boundary, so that automatic continuation never runs while control belongs to me.

#### Acceptance criteria

1. **3.1** WHEN Stop cancels a grind during provider streaming, tool execution, after a turn, or before the next turn starts, THEN THE existing native cancellation path SHALL terminate the current work and prevent every later continuation for that invocation.
2. **3.2** IF a provider request fails or ends with refusal, maximum-token termination, or cancellation, THEN THE grind SHALL stop immediately, preserve completed conversation content, and SHALL NOT start another provider request.
3. **3.3** IF any tool call reports failure during a grind turn, THEN THE system SHALL cancel that turn through the existing cancellation path, preserve its tool/action record, stop the grind, and require a new user action before further work.
4. **3.4** IF a tool permission or elicitation requires user attention during a grind turn, THEN THE system SHALL surface the existing request, cancel automatic continuation, and SHALL NOT resume the grind automatically after the user responds.
5. **3.5** WHEN a native session closes while grinding, THEN THE system SHALL cancel the active turn, prevent continuation, persist all finalized conversation and tool/action records, and remove transient grind state.
6. **3.6** IF a session or its ACP thread disappears at any grind boundary, THEN THE loop SHALL terminate without a replacement session, detached continuation, or false success status.
7. **3.7** AFTER any success, failure, attention, cancellation, limit, or closure termination, THE session SHALL no longer be marked as grinding and a later grind SHALL require a new explicit invocation.

### Requirement 4: Persistence, visibility, and reload

**User story:** As a Sim user reopening a session, I want my goal retained but interrupted automatic work stopped, so that persistence never becomes implicit spending authority.

#### Acceptance criteria

1. **4.1** WHEN a goal is set or replaced and the session reloads, THEN THE restored `Thread` SHALL expose the same goal through `/goal` and `/status`.
2. **4.2** WHEN a goal is cleared and the session reloads, THEN THE restored `Thread` SHALL have no goal, including when the goal was the only persisted session state.
3. **4.3** WHEN a session is reopened after an active or interrupted grind, THEN THE goal SHALL remain available but grind state and progress SHALL be inactive and SHALL NOT restart automatically.
4. **4.4** WHEN an actual grind turn or tool call occurs, THEN THE existing `Thread` message, `ActionLog`, database, and `ThreadStore` paths SHALL retain the same conversation and action records they retain for an ordinary native-agent turn.
5. **4.5** WHEN `/clear` executes while no grind is active, THEN THE system SHALL clear conversation state through the existing clear path while preserving the current goal and all edit/action records.
6. **4.6** THE `/goal`, `/grind`, and `/status` command text, acknowledgements, validation failures, progress, and termination summaries SHALL remain transient `AcpThread` output and SHALL NOT appear in `Thread` messages or a later model request.
7. **4.7** WHEN `/status` executes, THEN THE output SHALL show the active goal or `none`, whether grinding is active, and progress as completed turns over the authorized limit; it SHALL NOT expose the transient grind control prompt or other hidden model context.
8. **4.8** WHEN the app quits or the final session reference closes, THEN THE existing final-save path SHALL persist the latest goal, messages, draft, and finalized tool/action state before unloading.

### Requirement 5: Catalog, autocomplete, and submission regressions

**User story:** As a Sim user, I want the new commands to behave like first-class native commands without breaking existing slash-command input behavior.

#### Acceptance criteria

1. **5.1** WHEN the native command catalog is built, THEN `/goal` and `/grind` SHALL each appear exactly once with the existing native category and unstructured input metadata; matching MCP prompts or skills SHALL remain reachable through existing qualification rules.
2. **5.2** WHEN autocomplete is opened for a native session, THEN `/goal` and `/grind` SHALL be sourced from the existing available-command update and completion provider; no UI-owned command list SHALL be added.
3. **5.3** WHEN `/goal <text>` or `/grind max_turns=<n>` is submitted, THEN THE existing conversation submission flow SHALL send the complete text as one native command and SHALL NOT queue its argument as an ordinary follow-up.
4. **5.4** WHEN an argumentless native command such as `/compact` is followed by text, THEN THE existing bare-command-plus-queued-remainder behavior SHALL remain unchanged, including rich text, mentions, attachments, success ordering, and failure behavior.
5. **5.5** IF an unknown slash command is submitted, THEN THE existing editor-preservation, suggestion/error, zero-dispatch, and zero-model-call behavior SHALL remain unchanged and SHALL list the newly advertised native commands where appropriate.
6. **5.6** THE feature SHALL preserve existing skill and MCP prompt dispatch, including native-name collision precedence, without changing commands advertised by external ACP agents.

## Constraints

- Reuse `NativeAgent`, `AcpThread`, `Thread`, `pending_sessions`, and `ThreadStore`.
- Extend the current native slash-command catalog, `Command::parse`, autocomplete, and conversation submission flow. Do not add another parser, command registry, execution manager, or conversation state machine.
- The persisted goal belongs to `Thread`/`DbThread`. Active grind consent and progress are transient open-session state and must not be serialized.
- The grind-only goal-satisfaction tool may be exposed only while a grind turn is running. Its actual invocation and result are ordinary persisted tool records.
- Reuse the current cancellation, permission, elicitation, provider, tool, and session-close paths. Do not bypass trust, sandbox, path, or authorization decisions.
- Preserve all edit/action records when clearing or terminating a grind.
- All turn accounting uses saturating or checked bounds; no invocation can authorize more than 20 provider turns.

## Evidence and ownership

- Goose audit: `projects/goose/crates/goose/src/agents/execute_commands.rs` (`COMMANDS`, `command_starts_turn`, goal/grind handlers), `projects/goose/crates/goose/src/agents/state_machine/ops_retry.rs`, `ops_maxturns.rs`, and `projects/goose/crates/goose/src/agents/agent.rs` turn-limit/cancellation paths.
- Sim catalog/dispatch/session owner: `crates/agent/src/agent.rs` (`NATIVE_COMMANDS`, `Command::parse`, `NativeAgentConnection::prompt`, `Session`, `pending_sessions`, `close_session`).
- Sim persistence/turn owner: `crates/agent/src/thread.rs`, `crates/agent/src/db.rs`, and `crates/agent/src/thread_store.rs`.
- Sim visible/transient thread owner: `crates/acp_thread/src/acp_thread.rs`.
- Sim UI owner: `crates/agent_ui/src/conversation_view/thread_view.rs`, `crates/agent_ui/src/conversation_view.rs`, and the existing message-editor completion/unknown-command path.
- Migration coverage owner record: `../developer-experience/coverage-audit.md` capability `DCC-CMD-012`.

## Approved product decisions

- Default grind limit: 5 turns.
- Hard maximum: 20 turns; out-of-range overrides are rejected rather than clamped.
- A grind runs only in the foreground lifetime of the `/grind` prompt task.
- Goal satisfaction is an explicit model declaration through a grind-only native tool, not a second provider evaluation or a text heuristic.
- `/clear` preserves the current goal; only the approved `/goal` clear forms remove it.
- A tool failure or user-attention request ends the current grind rather than allowing the model to recover automatically within that grind.

## Open questions

None. The product, spending, persistence, restart, cancellation, permission, and bounded-execution decisions required for this scope are approved above.
