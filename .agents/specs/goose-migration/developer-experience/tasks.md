# Implementation Plan: Developer Context and Commands

## Approach

Extend the current native-agent flow in place. Commands are added to the catalog and dispatcher that already serve autocomplete; local results use the existing `AcpThread` conversation; developer context remains in `UserAgentsMd`, `ProjectContext`, and the existing skill integration; roots remain owned by `Project` and worktrees; lifecycle work is limited to missing regressions for `pending_sessions` and `ThreadStore`.

The tasks are intentionally sequenced because several touch `crates/agent/src/agent.rs` and the conversation UI. Recipe commands and MCP Apps have no tasks in this pack.

## Dependency waves

- Wave 1: Task 1 establishes native local command output and the catalog.
- Wave 2: Task 2 completes MCP prompt command discovery and argument execution in the same owner.
- Wave 3: Tasks 3-4 complete clear and conversation submission behavior.
- Wave 4: Tasks 5-6 complete and verify Sim-native developer context.
- Wave 5: Task 7 closes session-lifecycle regression coverage after the shared agent test surface is stable.

## Tasks

- [x] 1. Add Sim-native `/skills` and `/status` commands to the existing catalog and dispatcher
  - Reserve `clear`, `skills`, and `status` alongside `compact` when qualifying MCP prompt collisions; advertise the approved native commands with descriptions and native category metadata.
  - Route `/skills` and `/status` from `NativeAgentConnection::prompt` using the existing `Command::parse` result and existing `ProjectState`, `Thread`, `UserAgentsMd`, and `ProjectContext` snapshots.
  - Add the narrow `AcpThread` operation needed to append a distinct, transient local command result without adding a `Thread::Message` or model context.
  - Format `/skills` from the existing trusted skill catalog, including source-qualified invocation names when duplicates exist.
  - Format `/status` from the selected model/provider, token usage, visible worktrees, selected instruction sources, skill count, and current developer-context issue count; label unavailable values and omit instruction bodies.
  - Add focused tests for catalog contents, native/MCP/skill collisions, output formatting, unavailable status fields, distinct local entries, and zero provider completions.
  - _Requirements: 1.1, 1.3, 1.4, 1.6, 1.9, 1.10, 1.11, 4.1_
  - _Design: D-COMMAND-DISPATCH, D-LOCAL-OUTPUT_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/developer-experience/requirements.md, .agents/specs/goose-migration/developer-experience/design.md, projects/goose/crates/goose/src/agents/execute_commands.rs, crates/agent/src/agent.rs, crates/agent/src/thread.rs, crates/acp_thread/src/acp_thread.rs, crates/agent_settings/src/user_agents_md.rs, crates/prompt_store/src/prompts.rs_
  - _Writes: crates/agent/src/agent.rs, crates/acp_thread/src/acp_thread.rs_
  - _Validation: `cargo test -p agent native_command`, `cargo test -p agent status_command`, `cargo test -p agent skills_command`, `cargo test -p acp_thread local_command`, then `./script/clippy -p agent -p acp_thread`_
  - _Evidence: All focused tests and clippy validation passed on 2026-08-11._

- [x] 2. Complete dynamically named MCP prompt commands in the existing catalog and dispatcher
  - Extend `NativeAgent::build_available_commands_for_project` so prompts with zero, one, or multiple declared arguments are all advertised through the existing command catalog; retain native/MCP qualification and active-session catalog refresh.
  - Carry each prompt's declared argument name, description, and required/optional status through the available-command input metadata supported by the current ACP types so autocomplete can explain valid input without a second registry.
  - Add one shared argument helper called by `NativeAgentConnection::prompt`: keep the existing unambiguous single-argument remainder form, support shell-style quoted `name=value` tokens for declared arguments, and reject duplicate, unknown, malformed, missing-required, or unexpected arguments before invoking a server.
  - Reuse `ContextServerRegistry::find_prompt` and `NativeAgent::send_mcp_prompt` for valid execution, persistence, returned prompt messages, cancellation, and server/protocol/content errors; do not add literal `/prompt` or `/prompts` commands.
  - Test zero-, one-, optional-, and multi-argument catalogs and invocations; collisions; quoted/empty values; every validation failure; server failure; returned message handling; and zero MCP/model calls on invalid input.
  - _Requirements: 1.1, 1.4, 1.7, 1.13, 1.14, 1.15, 1.16_
  - _Design: D-COMMAND-DISPATCH, D-MCP-PROMPTS_
  - _Depends on: 1_
  - _Reads: .agents/specs/goose-migration/developer-experience/requirements.md, .agents/specs/goose-migration/developer-experience/design.md, projects/goose/crates/goose/src/agents/state_machine/ops_toolcalling.rs, projects/goose/crates/goose/src/acp/response_builder.rs, crates/agent/src/agent.rs, crates/agent/src/tools/context_server_registry.rs, crates/context_server/src/context_server.rs, crates/acp_thread/src/acp_thread.rs_
  - _Writes: crates/agent/src/agent.rs_
  - _Validation: `cargo test -p agent mcp_prompt_command`, `cargo test -p agent available_commands`, then `./script/clippy -p agent`_
  - _Evidence: All focused tests and clippy validation passed on 2026-08-11._

- [x] 3. Implement `/clear` through existing thread and persistence owners
  - Add one `Thread` mutation that resets model-visible messages, compaction/detailed summary state, current token accounting, and conversation-only bookkeeping while preserving session ID, title, project, model selection, profile, tools, settings, and records of edits or other real-world actions.
  - Add the corresponding `AcpThread` mutation that removes all visible entries, emits the existing removal/token events, and permits a distinct transient confirmation entry afterward.
  - Add an explicit fallible save path for an empty existing session through `NativeAgent` and the current thread database/store integration; do not change the policy for never-used empty sessions.
  - Execute clear only after the existing command queue reaches idle. Do not mutate live state or emit success if empty-session persistence fails.
  - Verify that a cleared session reloads empty, preserves its metadata and model selection, reports zero current usage, and does not expose the confirmation to the next model request.
  - Add failure coverage for storage errors and for a session disappearing at the persistence-to-foreground boundary.
  - _Requirements: 1.6, 1.8, 1.11, 1.13, 5.5_
  - _Design: D-CLEAR, D-LOCAL-OUTPUT_
  - _Depends on: 2_
  - _Reads: .agents/specs/goose-migration/developer-experience/requirements.md, .agents/specs/goose-migration/developer-experience/design.md, crates/agent/src/agent.rs, crates/agent/src/thread.rs, crates/agent/src/db.rs, crates/agent/src/thread_store.rs, crates/acp_thread/src/acp_thread.rs_
  - _Writes: crates/agent/src/agent.rs, crates/agent/src/thread.rs, crates/agent/src/thread_store.rs, crates/acp_thread/src/acp_thread.rs_
  - _Validation: `cargo test -p agent clear_command`, `cargo test -p agent clear_conversation`, `cargo test -p acp_thread clear_entries`, then `./script/clippy -p agent -p acp_thread`_
  - _Evidence: All focused tests and clippy validation passed on 2026-08-11._

- [x] 4. Complete native command autocomplete, unknown-command, and queued-input behavior
  - Reuse the existing available-command and available-skill data in `MessageEditor`; do not add command syntax or a registry to the UI.
  - Verify autocomplete shows `/compact`, `/clear`, `/skills`, and `/status` exactly once under the native category and never invents `/help`.
  - Exercise full conversation submission for an unknown command: keep the editor text and rich content intact, show the existing error callout with available commands or a suggestion, and assert the native agent and fake model receive no prompt.
  - Exercise `leading_native_command` and `send_command_queueing_remainder` for every approved native command with trailing text, mentions, and attachments; run the bare command first and dispatch the remainder exactly once after success.
  - Ensure a failed native command does not fast-track its queued remainder and that correcting the preserved input can be resubmitted normally.
  - _Requirements: 1.5, 1.7, 1.12, 1.13_
  - _Design: D-COMMAND-DISPATCH, D-UNKNOWN-QUEUE_
  - _Depends on: 3_
  - _Reads: .agents/specs/goose-migration/developer-experience/requirements.md, .agents/specs/goose-migration/developer-experience/design.md, crates/agent_ui/src/message_editor.rs, crates/agent_ui/src/conversation_view/thread_view.rs, crates/agent_ui/src/conversation_view.rs, crates/acp_thread/src/acp_thread.rs, crates/agent/src/agent.rs_
  - _Writes: crates/agent_ui/src/message_editor.rs, crates/agent_ui/src/conversation_view/thread_view.rs_
  - _Validation: `cargo test -p agent_ui slash_command`, `cargo test -p agent_ui native_command`, `cargo test -p agent_ui queued_command`, then `./script/clippy -p agent_ui`_
  - _Evidence: All focused tests and clippy validation passed on 2026-08-11._

- [x] 5. Surface project-instruction failures through the existing context issue UI
  - Preserve the current replacement-snapshot and dismissal behavior while generalizing the skill-loading issue data/event enough to represent a project-instruction read failure with its worktree and relative source path.
  - Carry `RulesLoadingError` out of `build_project_context` instead of discarding it at the existing TODO; keep valid worktree instructions and skills active when one source fails.
  - Render project-instruction failures in the existing conversation issue callout, retain the existing global `UserAgentsMd` settings/error notification, and avoid displaying instruction contents or unrelated absolute paths.
  - Clear a resolved issue on the next project-context refresh and allow a later recurrence after recovery to be shown again.
  - Make the combined unresolved issue count available to `/status` without creating another diagnostics store.
  - _Requirements: 2.5_
  - _Design: D-CONTEXT_
  - _Depends on: 4_
  - _Reads: .agents/specs/goose-migration/developer-experience/requirements.md, .agents/specs/goose-migration/developer-experience/design.md, crates/agent/src/agent.rs, crates/agent_settings/src/user_agents_md.rs, crates/agent_ui/src/conversation_view/thread_view.rs_
  - _Writes: crates/agent/src/agent.rs, crates/agent_ui/src/conversation_view/thread_view.rs_
  - _Validation: `cargo test -p agent project_instruction_issue`, `cargo test -p agent_ui context_loading_issue`, then `./script/clippy -p agent -p agent_ui`_
  - _Evidence: All focused tests and clippy validation passed on 2026-08-11._

- [x] 6. Add end-to-end developer-context and worktree regressions
  - Verify personal `AGENTS.md`, the selected per-worktree instruction file, and trusted `.agents/skills` feed the existing prompt/catalog owners without `.goosehints`, `.simhints`, or a second context section.
  - Verify prompt precedence: personal instructions first, root-labelled project instructions second, and only skill metadata in the available-skills catalog until the existing skill invocation path loads a body.
  - Cover multiple visible worktrees, deterministic root labels, same-named skills, selected-instruction precedence, and no duplicate root/source state.
  - Cover open-session refresh after personal instruction edits, project instruction add/change/delete, worktree add/remove/rename/rescan, worktree trust changes, and project skill changes.
  - Assert project instructions and skills are opened through their owning worktree/`ProjectPath` APIs and that current restricted-workspace, ignore, trust, sandbox, and permission decisions are not broadened.
  - Verify `/status` observes the refreshed root, instruction, skill, and issue snapshots on an already-open session.
  - _Requirements: 2.1, 2.2, 2.3, 2.6, 2.7, 2.8, 4.1, 4.3, 4.4, 4.5_
  - _Design: D-CONTEXT, D-PATHS_
  - _Depends on: 5_
  - _Reads: .agents/specs/goose-migration/developer-experience/requirements.md, .agents/specs/goose-migration/developer-experience/design.md, crates/agent_settings/src/user_agents_md.rs, crates/prompt_store/src/prompts.rs, crates/agent/src/agent.rs, crates/agent/src/thread.rs, crates/agent/src/templates.rs, crates/agent/src/templates/system_prompt.hbs, crates/project/src/project.rs_
  - _Writes: crates/agent_settings/src/user_agents_md.rs, crates/prompt_store/src/prompts.rs, crates/agent/src/agent.rs, crates/agent/src/templates.rs_
  - _Validation: `cargo test -p agent_settings user_agents_md`, `cargo test -p agent project_context`, `cargo test -p agent system_prompt`, then `./script/clippy -p agent_settings -p prompt_store -p agent`_
  - _Evidence: All focused tests and clippy validation passed on 2026-08-11._

- [x] 7. Close the confirmed NativeAgent session-lifecycle regression gap
  - Preserve the existing concurrent-success and reference-count test for `open_thread`/`pending_sessions`; do not introduce an execution manager.
  - Add a concurrent shared-load failure test proving that all waiters receive the failure, only one load is attempted, and `pending_sessions` is cleared.
  - Add a retry assertion proving a later open is evaluated anew after the failed shared load rather than reusing poisoned state.
  - Retain and run final-close persistence coverage for latest messages and draft state through `ThreadStore`.
  - If a regression exposes a defect, fix it only in `NativeAgent`, `pending_sessions`, or `ThreadStore`; do not add provider/extension restoration or a generic lifecycle abstraction.
  - _Requirements: 5.1, 5.3, 5.4, 5.5_
  - _Design: D-LIFECYCLE_
  - _Depends on: 6_
  - _Reads: .agents/specs/goose-migration/developer-experience/requirements.md, .agents/specs/goose-migration/developer-experience/design.md, crates/agent/src/agent.rs, crates/agent/src/thread_store.rs, crates/agent/src/db.rs_
  - _Writes: crates/agent/src/agent.rs, crates/agent/src/thread_store.rs_
  - _Validation: `cargo test -p agent loaded_sessions_keep_state_until_last_close`, `cargo test -p agent pending_session`, `cargo test -p agent close_session_saves_thread`, then `./script/clippy -p agent`_
  - _Evidence: All focused tests and clippy validation passed on 2026-08-11._

## Completion checks

- Keep every checkbox unchecked until the corresponding implementation and validation are complete.
- Run the focused validations recorded on every task, then run all affected crate tests and `./script/clippy` for `agent`, `acp_thread`, `agent_ui`, `agent_settings`, and `prompt_store`.
- Re-run the feature-spec validator after implementation updates any acceptance or task metadata.
- Do not add recipe commands, MCP Apps, `.goosehints`, `.simhints`, `SourceRoot`, `Source`, a command registry, or an execution-manager abstraction while implementing this pack.
