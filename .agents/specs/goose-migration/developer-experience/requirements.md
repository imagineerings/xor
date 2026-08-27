# Requirements: Developer Context and Commands

## Problem

Zed already has native-agent slash-command dispatch, command autocomplete, project instructions, skills, worktree-aware paths, and coalesced session loading. The existing migration pack obscures those capabilities behind Goose-specific names and proposes parallel parsers, hint files, source registries, embedded apps, and lifecycle managers. Users instead need a small, complete Zed-native feature that adds useful local commands, makes existing developer context observable and reliable, and closes only verified behavior or regression gaps.

Goose is evidence for useful observable behavior. Its APIs, configuration paths, file names, and internal architecture are not compatibility requirements.

## Scope

### In scope

- Extend the existing native-agent command catalog and dispatch with `/clear`, `/skills`, and `/status`.
- Preserve and verify `/compact`, skill commands, autocomplete, collision handling, unknown-command diagnostics, and queued trailing input.
- Complete the existing dynamically named MCP prompt commands so prompts with zero, one, or multiple declared arguments remain discoverable and safely executable.
- Reuse the personal `AGENTS.md`, per-worktree project instruction files (including `AGENTS.md` and `.rules`), and trusted `.agents/skills` as developer context.
- Surface instruction and skill loading failures through existing Zed diagnostics.
- Reuse visible worktrees and `ProjectPath` for root identity and project-relative path resolution.
- Add missing regression coverage around `NativeAgent::pending_sessions` and `ThreadStore` lifecycle behavior.

### Out of scope

- Recipe execution and recipe-backed commands; these belong to `.agents/specs/goose-migration/recipe-system`.
- MCP Apps or any embedded HTML/web renderer; these remain behind their separate product and security decision.
- A `/help` command justified as Goose parity, or additional Goose commands without an approved Zed behavior and owner.
- `.goosehints`, Goose configuration directories, `.simhints`, or any parallel hints subsystem.
- Referenced-file import syntax for instruction files. Zed has no approved native syntax for this feature; skill references continue to use the existing skill tool and file tools.
- Arbitrary named sources, persistent source registries, or new `SourceRoot`/`Source` abstractions.
- A generic execution manager, provider/extension restoration layer, or new session lifecycle abstraction.
- Changes to commands advertised by external ACP agents.
- Goose's unstable slash-command and source-management ACP custom requests; Zed's native agent already publishes standard ACP available-command updates.
- Zed-native persistent `/goal` and bounded `/grind`; these are owned by `.agents/specs/goose-migration/goal-grind-commands` and are not implemented by this pack.

## Glossary

- **Native command**: A command owned by Zed's `NativeAgent` and advertised with the existing native command category.
- **Developer context**: Personal instructions, project instructions, trusted skill metadata, and visible worktree identity already supplied to the native agent.
- **Project instruction**: The first supported instruction file selected for a visible worktree by Zed's existing instruction-file precedence.
- **Local command output**: Conversation UI content produced without sending a user turn to the selected language model.

## Requirements

### Requirement 1: Native developer commands

**User story:** As a Zed user, I want discoverable local commands for conversation and developer context operations, so that I can inspect or change the current native-agent session without asking the model to do it.

#### Acceptance criteria

1. **1.1** WHEN the first non-whitespace content in a native-agent prompt is a recognized slash command, THEN THE native agent SHALL route it through the existing command dispatch before considering a language-model turn.
2. **1.3** WHEN a user invokes an available skill command, THEN THE native agent SHALL continue to resolve and invoke it through the existing skill catalog, scope, precedence, and permission behavior.
3. **1.4** WHEN native commands, MCP prompts, or trusted project skills have colliding names, THEN THE command catalog SHALL keep the native command unqualified, keep every non-native command reachable through the existing qualification rules, and update active sessions when the underlying catalog changes.
4. **1.5** IF a user submits an unknown slash command, THEN THE conversation UI SHALL preserve the submitted editor content, SHALL NOT send it to the language model, and SHALL show the existing error presentation with the recognized commands or an applicable suggestion.
5. **1.6** THE native-agent command catalog SHALL advertise `/compact`, `/clear`, `/skills`, and `/status` with descriptions and the existing native command category; it SHALL NOT advertise `/help` merely for Goose parity.
6. **1.7** WHEN the user opens slash-command autocomplete, THEN THE new native commands SHALL appear through the existing completion provider without a second parser or command registry.
7. **1.8** WHEN `/clear` executes after the session becomes idle, THEN THE system SHALL remove the model-visible messages, compaction/summary state, current conversation token accounting, and corresponding visible conversation entries; SHALL persist the cleared conversation; SHALL preserve the session identity, project, selected model, title, profile, settings, and records of edits or other real-world actions; and SHALL show a local confirmation.
8. **1.9** WHEN `/skills` executes, THEN THE system SHALL show the currently available skills using the existing catalog's invocation name, description, and source labels, including qualified names where needed to disambiguate duplicates.
9. **1.10** WHEN `/status` executes, THEN THE system SHALL show the selected provider and model, current context-token usage and limit when available, visible worktree labels, loaded personal/project instruction sources, available skill count, and unresolved developer-context issue count; unavailable values SHALL be identified explicitly.
10. **1.11** WHEN `/clear`, `/skills`, or `/status` executes, THEN THE system SHALL render its result as local command output in the existing conversation surface and SHALL NOT include the command or its output in a later language-model request.
11. **1.12** WHEN a user submits trailing text or content blocks after a native command, THEN THE existing native-command submission flow SHALL execute the bare command first and queue the remainder as an ordinary follow-up without dropping text, mentions, or attachments.
12. **1.13** IF a native command cannot complete, THEN THE conversation SHALL remain usable, the user's queued follow-up SHALL not run ahead of the failed command, and the existing conversation error UI SHALL show an actionable diagnostic without a false success result.
13. **1.14** WHEN an MCP server advertises a prompt with zero, one, or multiple declared arguments, THEN THE existing native-agent command catalog SHALL advertise one dynamically named MCP command and SHALL expose the prompt description plus declared argument names, descriptions, and required/optional status through the existing autocomplete metadata.
14. **1.15** WHEN a user invokes a multi-argument MCP prompt command, THEN THE existing native dispatcher SHALL parse explicit `name=value` arguments, support quoted values through one shared helper, reject duplicate, unknown, malformed, or missing required arguments visibly, and SHALL NOT call the MCP server or language model after validation failure.
15. **1.16** WHEN a valid MCP prompt command executes, THEN THE system SHALL invoke the existing context-server prompt owner, preserve the user's original command in conversation persistence where the current owner requires it, deliver the returned prompt messages through the existing thread/model flow, and surface server, protocol, or content errors through the existing conversation error UI; the feature SHALL NOT add literal `/prompt` or `/prompts` wrapper commands.

### Requirement 2: Zed-native developer context

**User story:** As a Zed user, I want the native agent to use the project's existing instructions and skills consistently, so that I can understand and trust the context influencing its behavior.

#### Acceptance criteria

1. **2.1** WHEN a native-agent project context is built, THEN THE system SHALL reuse the personal Zed `AGENTS.md`, the existing per-worktree project-instruction selection, and trusted global/project `.agents/skills` discovery rather than loading a separate hint format.
2. **2.2** WHEN a project opens, THEN THE system SHALL build developer context from every visible worktree in the existing visible-worktree order and label worktree-scoped context with the owning root.
3. **2.3** WHEN the system prompt is rendered, THEN THE personal `AGENTS.md` SHALL precede project instructions, project instructions SHALL retain their higher precedence, and skill metadata SHALL remain in the existing available-skills catalog with skill bodies loaded only through the existing skill invocation path.
4. **2.5** IF a personal instruction, project instruction, or skill cannot be read or parsed, THEN THE system SHALL omit only the unusable source, continue with valid sources, and show a source-labelled diagnostic through Zed's existing settings or conversation issue UI.
5. **2.6** IF multiple visible worktrees contain project instructions or same-named skills, THEN THE system SHALL preserve deterministic worktree labels and the existing instruction and skill precedence without silently merging unrelated roots.
6. **2.7** WHILE a worktree is not trusted for project-local skills, THE system SHALL exclude its `.agents/skills` entries from model and command catalogs while retaining Zed's existing restricted-workspace protections for project instructions and tool execution.
7. **2.8** WHEN personal instructions, selected project instructions, visible worktrees, worktree trust, or `.agents/skills` change, THEN active native-agent sessions SHALL use the refreshed context on the next applicable command or model turn without restarting the session.

### Requirement 4: Project-root ownership and path safety

**User story:** As a Zed user with one or more worktrees, I want developer context to use the same project roots and path rules as the editor, so that context cannot drift from the files and permissions I opened.

#### Acceptance criteria

1. **4.1** THE developer context and `/status` output SHALL derive root labels and paths from `Project::visible_worktrees` and existing worktree metadata, with no separately persisted root list.
2. **4.3** WHEN a project instruction or project skill is opened, THEN THE system SHALL resolve it through the owning worktree and `ProjectPath`-based project APIs rather than concatenating an independently configured source path.
3. **4.4** WHEN visible worktrees are added, removed, renamed, or rescanned, THEN THE developer context and `/status` output SHALL reflect the current project roots without stale or duplicate entries.
4. **4.5** THE developer-context feature SHALL preserve existing trust, ignore, sandbox, permission, and path-containment decisions and SHALL NOT broaden access outside a visible worktree or the user's existing global Zed context directories.

### Requirement 5: Native session lifecycle regression contract

**User story:** As a Zed user reopening a session, I want concurrent callers to share the existing native session lifecycle, so that command and context state is not duplicated or lost.

#### Acceptance criteria

1. **5.1** WHEN concurrent callers open the same unloaded session, THEN `NativeAgent` SHALL share the existing `pending_sessions` initialization rather than construct duplicate native threads.
2. **5.3** WHEN shared initialization succeeds, THEN every waiter SHALL receive the same `AcpThread` entity and the session reference count SHALL require a matching final close before unloading.
3. **5.4** IF shared initialization fails, THEN every waiter SHALL receive the failure, the pending entry SHALL be removed, and a later open attempt SHALL be able to retry rather than observing poisoned state.
4. **5.5** WHEN the final reference to a loaded session closes, THEN the existing `ThreadStore` persistence path SHALL save the latest conversation and draft state before the session is unloaded.

## Constraints

- Extend `Command::parse`, `NativeAgent::build_available_commands_for_project`, `NativeAgentConnection::prompt`, `MessageEditor`, and the existing conversation submission path; any MCP argument helper SHALL be called by that dispatch and catalog path rather than becoming another slash parser or registry.
- Use existing `AcpThread` entries and conversation errors for local output and failures. Transient `/skills`, `/status`, and `/clear` confirmation output need not be persisted, but it must never enter model context.
- Keep instruction-file names and precedence owned by `prompt_store::RULES_FILE_NAMES`; do not add migration-specific configuration.
- Keep skill discovery, qualification, trust filtering, and invocation owned by the existing agent-skills integration.
- Keep project roots and path resolution owned by `Project`, `Worktree`, `ProjectPath`, and existing project APIs.
- Keep session lifecycle owned by `NativeAgent`, `pending_sessions`, and `ThreadStore`.
- All user-facing errors must avoid disclosing instruction contents or unrelated absolute paths.

## Evidence and ownership

- Zed command dispatch: `crates/agent/src/agent.rs` — `build_available_commands_for_project`, `Command::parse`, `NativeAgentConnection::prompt`, `send_compact_command`, `send_skill_invocation`.
- Zed command UI: `crates/agent_ui/src/message_editor.rs` — `validate_slash_commands`; `crates/agent_ui/src/conversation_view/thread_view.rs` — `leading_native_command`, `send_command_queueing_remainder`.
- Zed context: `crates/agent_settings/src/user_agents_md.rs` — `UserAgentsMd`; `crates/prompt_store/src/prompts.rs` — `RULES_FILE_NAMES`, `ProjectContext`, `WorktreeContext`; `crates/agent/src/agent.rs` — `build_project_context`, `load_worktree_rules_file`; `crates/agent/src/templates/system_prompt.hbs`.
- Zed project ownership: `crates/project/src/project.rs` — `ProjectPath`, `Project::absolute_path`, `Project::find_project_path`, `Project::visible_worktrees`.
- Zed session lifecycle: `crates/agent/src/agent.rs` — `NativeAgent::open_thread`, `pending_sessions`, `close_session`, `save_thread`; `crates/agent/src/thread_store.rs`.
- Goose command evidence only: `projects/goose/crates/goose/src/agents/execute_commands.rs`; `projects/goose/crates/goose/src/agents/state_machine/{ops_slash_command,ops_toolcalling,ops_compaction,ops_skills,ops_retry,ops_llm}.rs`; `projects/goose/crates/goose/src/slash_commands/`; `projects/goose/crates/goose/src/acp/server/slash_commands.rs`; `projects/goose/crates/goose/src/acp/response_builder.rs`; `projects/goose/crates/goose-cli/src/session/{input,mod}.rs`; and `projects/goose/ui/desktop/src/acp/autocomplete.ts`.
- Goose context/source evidence only: `projects/goose/crates/goose/src/hints/{load_hints,import_files}.rs`; `projects/goose/crates/goose/src/agents/prompt_manager.rs`; `projects/goose/crates/goose/src/{source_roots,sources}.rs`; `projects/goose/crates/goose/src/acp/server/sources.rs`; and the desktop settings/source/skill views cited in `coverage-audit.md`.
- Goose lifecycle/app evidence only: `projects/goose/crates/goose/src/execution/manager.rs`; relevant `projects/goose/crates/goose/src/acp/server/` session, prompt-run, resource, tool, app, and proxy handlers; and the desktop MCP App surfaces cited in `coverage-audit.md`.
- The complete classification, security/lifecycle findings, and cross-spec ownership record is `coverage-audit.md`.

## Retired or moved criterion IDs

| Criterion IDs | Disposition |
| --- | --- |
| 1.2 | Recipe commands remain owned by `.agents/specs/goose-migration/recipe-system`; this pack has no recipe task. |
| 2.4 | Referenced-file imports are removed from this feature because Zed has no approved native instruction import syntax. |
| 3.1-3.5 | MCP Apps are excluded and remain behind the separate product/security decision. |
| 4.2 | Arbitrary named sources are removed; visible worktrees and existing skill/instruction owners cover this feature. |
| 5.2 | Generic provider/extension restoration and cancellation are removed; no corresponding gap was confirmed in Zed's session model. |

Goose's `/doctor` remains owned by `agent-infrastructure`; dynamically named recipe commands and the CLI-local literal `/recipe` flow remain owned by `recipe-system`; terminal-only `/help`, `/?`, theme, model/mode, edit, new-session, and related presentation commands remain owned by `text-ui` if that product is approved. Zed-native `/goal` and bounded `/grind` are approved and owned by `.agents/specs/goose-migration/goal-grind-commands`. Nested access-triggered context, instruction imports, source CRUD/import/export APIs, agent/check source catalogs, and MCP Apps still require separate product or security decisions recorded in `coverage-audit.md`.
