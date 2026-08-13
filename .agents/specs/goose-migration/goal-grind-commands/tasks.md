# Implementation Plan: Goal and Grind Commands

## Approach

Implement one dependency-ordered increment at a time. The persistent goal is established before any automatic loop; the reusable grind-turn contract is established before orchestration; cancellation/lifecycle semantics are completed before UI integration and final persistence regressions.

## Tasks

- [x] 1. Extend the existing native command catalog and argument dispatch
  - _id: goal-grind-native-command-contract_
  - _priority: P1_
  - _value: high_
  - _wave: 1_
  - _Depends on: none_
  - _reads: .agents/specs/goose-migration/goal-grind-commands/requirements.md, .agents/specs/goose-migration/goal-grind-commands/design.md, projects/goose/crates/goose/src/agents/execute_commands.rs, projects/goose/crates/goose/src/agents/state_machine/ops_retry.rs, crates/agent/src/agent.rs, crates/agent_ui/src/conversation_view/thread_view.rs_
  - _writes: crates/agent/src/agent.rs_
  - _validation: cargo test -p agent goal_command_parse && cargo test -p agent grind_command_parse && cargo test -p agent native_commands_are_available && ./script/clippy -p agent_
  - _Requirements: 1.4, 1.6, 2.1, 2.2, 2.3, 5.1, 5.6_
  - Outcome: `/goal` and `/grind` are reserved, advertised NativeAgent commands with input metadata and exact, separately testable argument semantics.
  - Design: D1
  - Done when: Catalog/collision tests show each command once, argument tests cover show/set/clear/default/override/invalid/overflow forms, and external ACP catalogs remain outside the builder.
  - _Evidence: `cargo test -p agent goal_command_parse`, `cargo test -p agent grind_command_parse`, `cargo test -p agent native_commands_are_available`, and `./script/clippy -p agent` passed on 2026-08-11._

- [x] 2. Persist `/goal` through Thread and ThreadStore with transient output
  - _id: goal-grind-persisted-goal_
  - _priority: P1_
  - _value: high_
  - _wave: 2_
  - _blocked_by: goal-grind-native-command-contract_
  - _Depends on: 1_
  - _reads: .agents/specs/goose-migration/goal-grind-commands/requirements.md, .agents/specs/goose-migration/goal-grind-commands/design.md, crates/agent/src/agent.rs, crates/agent/src/thread.rs, crates/agent/src/db.rs, crates/agent/src/thread_store.rs, crates/acp_thread/src/acp_thread.rs_
  - _writes: crates/agent/src/agent.rs, crates/agent/src/thread.rs, crates/agent/src/db.rs_
  - _validation: cargo test -p agent goal_command && cargo test -p agent goal_persistence && cargo test -p agent clear_conversation_preserves_goal && ./script/clippy -p agent_
  - _Requirements: 1.1, 1.2, 1.3, 1.4, 4.1, 4.2, 4.5, 4.6, 4.8_
  - Outcome: Goal set/show/replace/clear is atomic, persistent across reload including goal-only sessions, model-free, and compatible with `/clear`.
  - Design: D2, D7
  - Done when: Focused tests prove successful and failed persistence, backward-compatible DB loading, clear keywords, replacement, goal-only clear, `/clear` preservation, transient output, and zero provider calls.
  - _Evidence: Goal command, persistence, clear-preservation, and DB compatibility tests plus `./script/clippy -p agent` and the feature-spec validator passed on 2026-08-11._

- [x] 3. Add the grind-only Thread turn contract and satisfaction tool
  - _id: goal-grind-thread-turn-contract_
  - _priority: P1_
  - _value: high_
  - _wave: 3_
  - _blocked_by: goal-grind-persisted-goal_
  - _Depends on: 2_
  - _reads: .agents/specs/goose-migration/goal-grind-commands/requirements.md, .agents/specs/goose-migration/goal-grind-commands/design.md, crates/agent/src/thread.rs, crates/agent/src/db.rs, crates/agent/src/tools.rs, crates/agent/src/tests/test_tools.rs_
  - _writes: crates/agent/src/thread.rs_
  - _validation: cargo test -p agent grind_turn && cargo test -p agent grind_satisfaction && cargo test -p agent grind_transient_context && ./script/clippy -p agent_
  - _Requirements: 2.5, 2.6, 4.4, 4.6_
  - Outcome: One existing `Thread::resume` turn can receive transient goal control, expose a grind-only satisfaction tool, and persist the real Resume/assistant/tool records without persisting control context.
  - Design: D3
  - Done when: Request-capture and reload tests prove goal control/tool availability only during grind, explicit satisfaction reporting, normal profile/security behavior for other tools, persisted tool results, and absence of control/command output from serialized messages.
  - _Evidence: `cargo test -p agent grind_turn`, `cargo test -p agent grind_satisfaction`, `cargo test -p agent grind_transient_context`, and `./script/clippy -p agent` passed on 2026-08-11._

- [x] 4. Implement the bounded foreground grind driver in NativeAgent
  - _id: goal-grind-bounded-driver_
  - _priority: P1_
  - _value: high_
  - _wave: 4_
  - _blocked_by: goal-grind-thread-turn-contract_
  - _Depends on: 3_
  - _reads: .agents/specs/goose-migration/goal-grind-commands/requirements.md, .agents/specs/goose-migration/goal-grind-commands/design.md, crates/agent/src/agent.rs, crates/agent/src/thread.rs, crates/acp_thread/src/acp_thread.rs_
  - _writes: crates/agent/src/agent.rs_
  - _validation: cargo test -p agent grind_success && cargo test -p agent grind_turn_bound && cargo test -p agent concurrent_grind && cargo test -p agent status_command && ./script/clippy -p agent_
  - _Requirements: 1.5, 2.1, 2.2, 2.4, 2.6, 2.7, 2.8, 2.9, 3.7, 4.7_
  - Outcome: `/grind` owns one foreground, per-session, invocation-identified loop that stops on satisfaction or the displayed default/override bound and reports accurate transient progress.
  - Design: D4, D7
  - Done when: Tests prove first-turn satisfaction, multi-turn continuation, exact default 5 and hard 20 bounds, no 21st request, one active loop, goal-mutation rejection while active, cleanup/reinvoke, and `/status` active/inactive progress without hidden prompt content.
  - _Evidence: Satisfaction, missing/invalid input, default/hard-bound, concurrent-loop, goal-mutation, status, and `./script/clippy -p agent` validations passed on 2026-08-11._

- [x] 5. Terminate grind at cancellation, failure, attention, and session boundaries
  - _id: goal-grind-termination-boundaries_
  - _priority: P1_
  - _value: high_
  - _wave: 5_
  - _blocked_by: goal-grind-bounded-driver_
  - _Depends on: 4_
  - _reads: .agents/specs/goose-migration/goal-grind-commands/requirements.md, .agents/specs/goose-migration/goal-grind-commands/design.md, crates/agent/src/agent.rs, crates/agent/src/thread.rs, crates/acp_thread/src/acp_thread.rs, crates/agent/src/tests/test_tools.rs_
  - _writes: crates/agent/src/agent.rs, crates/agent/src/thread.rs_
  - _validation: cargo test -p agent grind_cancel && cargo test -p agent grind_provider_failure && cargo test -p agent grind_tool_failure && cargo test -p agent grind_permission && cargo test -p agent grind_session_close && ./script/clippy -p agent_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7, 4.3, 4.4, 4.8_
  - Outcome: The existing cancellation/event/session paths terminate automatic continuation at every approved boundary, preserve finalized records, and never restart after permission response, close, or reload.
  - Design: D4, D5, D6
  - Done when: Deterministic GPUI tests cover Stop during provider/tool execution, after turn completion, and before continuation; provider/refusal/max-token/tool/permission/elicitation failures; ACP/session disappearance; final close; inactive reload; and no later provider request.
  - _Evidence: Provider/tool/boundary cancellation, refusal/max-token/provider error, failed-tool persistence, permission/elicitation attention, close/reload inactivity, and `./script/clippy -p agent` validations passed on 2026-08-11._

- [x] 6. Integrate argument-bearing native commands with autocomplete and submission
  - _id: goal-grind-ui-command-submission_
  - _priority: P1_
  - _value: high_
  - _wave: 6_
  - _blocked_by: goal-grind-termination-boundaries_
  - _Depends on: 5_
  - _reads: .agents/specs/goose-migration/goal-grind-commands/requirements.md, .agents/specs/goose-migration/goal-grind-commands/design.md, crates/agent_ui/src/conversation_view/thread_view.rs, crates/agent_ui/src/conversation_view.rs, crates/agent_ui/src/message_editor.rs, crates/acp_thread/src/acp_thread.rs, crates/agent/src/agent.rs_
  - _writes: crates/agent_ui/src/conversation_view/thread_view.rs, crates/agent_ui/src/conversation_view.rs_
  - _validation: cargo test -p agent_ui goal_grind_command && cargo test -p agent_ui native_command_queued && cargo test -p agent_ui slash_command && ./script/clippy -p agent_ui_
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_
  - Outcome: The existing completion/submission path consumes `/goal` and `/grind` arguments while preserving all argumentless-command queue and unknown-command behaviors.
  - Design: D1, D8
  - Done when: Full conversation tests show native autocomplete, complete argument delivery, no queued goal/limit text, unchanged rich `/compact` remainder ordering/failure behavior, preserved unknown rich input, zero model calls, and unchanged external ACP handling.
  - _Evidence: Native autocomplete/argument metadata, single-prompt and queued `/goal`/`/grind` delivery, argumentless rich-remainder success/failure, unknown-command preservation, slash-command regressions, and `./script/clippy -p agent_ui` passed on 2026-08-11._

- [x] 7. Close reload, final-save, action-record, and cross-command regressions
  - _id: goal-grind-persistence-regressions_
  - _priority: P1_
  - _value: high_
  - _wave: 7_
  - _blocked_by: goal-grind-ui-command-submission_
  - _Depends on: 6_
  - _reads: .agents/specs/goose-migration/goal-grind-commands/requirements.md, .agents/specs/goose-migration/goal-grind-commands/design.md, .agents/specs/goose-migration/developer-experience/coverage-audit.md, crates/agent/src/agent.rs, crates/agent/src/thread.rs, crates/agent/src/db.rs, crates/agent/src/thread_store.rs, crates/acp_thread/src/acp_thread.rs_
  - _writes: crates/agent/src/agent.rs, crates/agent/src/thread.rs, crates/agent/src/db.rs, .agents/specs/goose-migration/goal-grind-commands/requirements.md, .agents/specs/goose-migration/goal-grind-commands/design.md, .agents/specs/goose-migration/goal-grind-commands/tasks.md_
  - _validation: cargo test -p agent goal_grind_reload && cargo test -p agent close_session_saves_thread && cargo test -p agent flush_threads_on_quit && cargo test -p agent clear_command && cargo test -p acp_thread local_command && ./script/clippy -p agent -p acp_thread_
  - _Requirements: 1.6, 2.9, 3.5, 3.7, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.8, 5.6_
  - Outcome: Goal/grind behavior remains correct across reload, app quit, final close, `/clear`, action replay, local-output exclusion, and existing native command owners.
  - Design: D2, D6, D7
  - Done when: Integration tests preserve goal/messages/draft/tool/action records, clear keeps goal/actions, reopened sessions are inactive, local output is absent from model context, coverage/spec traceability is current, and all focused validation passes.
  - _Evidence: Goal/grind reload and transient-context exclusion, close and quit final snapshots, failed-tool records, clear preservation, ACP local output, and `./script/clippy -p agent -p acp_thread` validations passed on 2026-08-11._

## Completion checks

- Mark each checkbox only after its implementation and focused `_validation` command succeeds.
- After Task 7 run all tests for affected crates, `./script/clippy` for affected crates, formatting checks, both feature-spec validators, and `git diff --check`.
- Confirm every acceptance criterion appears in design traceability and at least one task.
- Do not commit, push, or open a pull request.
