# Buzz React/Tauri desktop retirement manifest

Date: 2026-08-25

Status: **PREPARED — REMOVAL HOLD**

This manifest prepares retirement of Buzz desktop 0.5.11 without deleting or changing `projects/buzz`. It does not authorize source retirement, terminate a supported compatibility window or claim that deployment-specific Task 47.1 traffic and rollback-window evidence has passed.

## Frozen source baseline

| Source | SHA-256 |
| --- | --- |
| `projects/buzz/desktop/src-tauri/src/lib.rs` | `1a1501c98087a156ae8a81174f53e1a03314f34db57be517b0370c8fa937f38f` |
| `projects/buzz/desktop/src-tauri/src/native_websocket.rs` | `f795663a4f6aba1d3dd7f8577fe1839fde7eb1da1000780bbbc73f0d04f9ba12` |
| `crates/zed/src/migration/buzz/desktop_state.rs` | `154a66f5d79fc2e7c44dbe7beb1ac8c7634c65b15bec757635a34c41bfd43f87` |
| `crates/zed/src/migration/buzz/agent_staging.rs` | `8fb1e13f73e99188e57d3e9b838ae74f1a26c27d64b7863ff762b0f651c6b957` |
| `crates/zed/src/migration/buzz/agent_state.rs` | `f6a71af559237dc21879ab7c4a73f1d480adbe7a516e9996eacef5dd33d558f9` |

Both the React package and Tauri crate declare version `0.5.11`, matching the compatibility matrix.

## Tauri command census and disposition

The main application handler registers 303 commands and the native WebSocket plugin registers 4, for 307 runtime registrations. The source has 313 `#[tauri::command]` annotations: the six `mesh_*` names each have a real implementation in `commands/mesh_llm.rs` and a mutually exclusive implementation in `mesh_llm_stubs.rs`. Resolving that feature branch removes six duplicate annotations and yields the exact 307 registrations. Every runtime command belongs to one row below.

| Command family | Runtime count | Buzz source groups | Canonical owner or retained boundary | Retirement disposition |
| --- | ---: | --- | --- | --- |
| Service, communication and identity | 109 | `builderlab.rs`, `native_websocket.rs`, `commands/{canvas,channel*,channels,dms,identity*,join_policy,messages/**,profile,relay*,social}.rs` | Collab tenant/RPC/Nostr admission, canonical community/channel/message/identity/social repositories and the versioned client adapter | Retire the Tauri forwarding shell; frozen clients continue only through the published adapter, never a Buzz database writer. Builderlab is an external client integration, not an embedded server owner. |
| Agent, persona and team | 64 | `commands/{agent*,agents,engrams,global_agent_config,personas/**,team_snapshot,teams}.rs`, `managed_agents/runtime_commands.rs` | Zed `agent`, `agent_settings`, ACP/MCP adapters and canonical managed-agent persistence | Retire the React/Tauri presentation and duplicate local runtime only under Task 47.3; keep required external shims and imported source evidence. |
| Media, huddle, pairing and mesh | 60 | `commands/{link_preview,media*,mesh_llm,pairing}.rs`, `huddle/**`, feature-selected `mesh_llm_stubs.rs` | Canonical Collab media/huddle/pair boundaries, native Zed audio/TTS and remote-agent mesh scheduling | Retire desktop IPC wrappers; do not treat feature stubs, device selection or companion-window control as a server capability. |
| Project, Git and workspace | 31 | `commands/{project_*,workspace}.rs`, `terminal_runtime.rs` | Existing Zed `Project`, `GitStore`, workspace, terminal and review owners plus NIP-34 compatibility adapters | Retire duplicate Tauri orchestration. Repository/worktree bytes and terminal processes remain with Zed's native owners, not a desktop import. |
| Desktop state and archive | 14 | `archive/**`, `commands/{legacy_storage,observer_archive,agent_metric_archive}.rs` | Task 17.5 deterministic desktop-state/archive import and Tasks 17.9/30.5 agent staging/import | Retire only after each applicable profile has a verified import receipt and retained source. No archive command remains a serving owner. |
| Workflow | 11 | `commands/workflows.rs` | Canonical collaboration workflow definition, scheduler/admission, run, approval and audit owners | Retire Tauri workflow IPC; it is a client adapter and owns no workflow queue or executor after cutover. |
| Desktop shell and platform | 18 | remaining deep-link, notification, clipboard, updater, window, tray and OS integration commands | Native GPUI/Zed window, notification, clipboard, deep-link and release owners | Retire with the React/Tauri shell. These commands have no server-side data or authority. |
| **Total** | **307** |  |  |  |

## State dependency audit

| Former desktop state | Complete migration/disposition evidence | Tauri dependency after retirement |
| --- | --- | --- |
| General configuration and device presentation settings | Task 17.5 accepts bounded snapshot versions 1–2, imports all non-secret general configuration and the explicit theme, text, layout, notification, presence, archive-default and link-preview keys, and binds source/target hashes. Native collaborative presentation and restart behavior passed Task 10.7. | None. Zed settings and workspace persistence are final owners. |
| Drafts, read state and manual unread | Task 17.5 imports draft v1/v2 records, attachment/mention metadata, NIP-RS contexts, publishable/source-time maps and forced-unread state deterministically. Sent drafts and cache-only entries are explicitly counted, not promoted into duplicate authority. | None. Native draft/read owners consume the import output; Buzz local storage remains rollback evidence until verified. |
| Local archive, scopes and save subscriptions | Task 17.5 validates archive schemas 1–4, marker sets, raw signed events, scope foreign keys and subscriptions while preserving the source. Canonical event verification and archive owners receive the normalized output. | None. The 11 archive IPC commands are not retained as a serving store. |
| Managed agents, personas, teams and snapshots | Task 17.9 stages versioned, privacy-classified records with protected-credential references and execution disabled. Task 30.5 writes only explicitly materialized canonical memory, managed-agent snapshot and usage records, reads them back, records idempotent receipts and retains unmapped source evidence. | None for data custody. Runtime retirement remains separately audited by Task 47.3. |
| Agent observer frames and usage metrics | Tasks 17.9 and 30.5 retain kind-24200 evidence and import matching kind-44200 usage without inventing plaintext or a second observer store. | None. The two default/archive commands are presentation and query adapters only. |
| Sprout workspace/onboarding local storage | `get_legacy_workspace_storage` is a one-time, read-only predecessor seeder. Project/worktree state is already owned by Zed and existing users deliberately default to Editor under Task 5.5; predecessor onboarding flags confer no canonical authorization or data ownership. Task 17.5 counts non-authoritative cache entries instead of copying them. | None. No predecessor workspace or onboarding record is required to render or operate the native workspace. |
| Identity and credentials | Credential-shaped snapshot fields fail Task 17.5 closed and use the verified protected-credential/identity import paths. Public profiles and signed events are canonical protocol records rather than Tauri state. | None. Removing Tauri must not delete the retained source or protected credential until its owning import receipt exists. |
| Local repositories, worktrees and terminal sessions | These are native filesystem/process resources, not Buzz desktop records. The target architecture keeps Zed `Project`, `GitStore`, worktree and terminal ownership throughout migration. | None. Retirement removes duplicate UI orchestration, not repositories, worktrees or terminal data. |

The audit therefore finds no server capability whose only implementation is a Tauri command and no authoritative desktop state that must remain served by Tauri. The command layer is either a presentation/platform wrapper, a client adapter to a canonical owner, duplicate runtime scope reserved for Task 47.3, or a read-only source covered by the versioned importers.

## Repository dependency audit

A repository search over `Cargo.toml`, `crates`, `services`, `tools`, `script`, `.github`, `package.json` and `pnpm-lock.yaml` found no Zed build, package or release dependency on `projects/buzz/desktop`. The remaining references are intentional and must survive desktop retirement:

- desktop-state and agent-state importers and the Task 17.6 recovery test;
- protected identity-import provenance;
- the `buzz-desktop` client entry and tests in the published compatibility matrix.

## Proposed retirement change

Once every gate below is satisfied, a separately approved source-retirement change may remove Buzz desktop build/package/release inputs and stop shipping its React bundle and Tauri binary. It must not delete imported canonical state, original migration evidence, compatibility fixtures, protocol documentation, license notices or Git history. This manifest itself performs no removal.

Required gates:

- Task 10.7 native theme, zoom, narrow-window, reduced-motion and restart parity remains passing.
- Tasks 17.5, 17.9 and 30.5 have exact successful receipts for every applicable retained source/profile; no source file has been removed.
- Task 47.1 records zero direct legacy writes and approved desktop adapter-read, adapter-write, active-client, observation-window and rollback-window thresholds pass in the target deployment.
- The published compatibility window is narrowed through its versioned policy process; frozen clients are not silently stranded.
- Task 47.3 confirms no agent/runtime process still depends on the desktop binary.
- Task 47.5 preserves artifacts, licenses and source history, and Task 47.6 confirms no duplicate owner or build dependency.
- A human explicitly approves the source-retirement/deletion gate.

Until then, disposition is **HOLD**: preserve `projects/buzz/desktop` unchanged and do not alter production routing or packaging.

## Validation commands

```text
rg -n '#\[tauri::command\]' projects/buzz/desktop/src-tauri/src -g '*.rs'
awk main generate_handler census: 303
native_websocket generate_handler census: 4
runtime registrations after resolving six mesh real/stub duplicates: 307
rg -n 'projects/buzz/desktop|buzz/desktop|buzz-desktop' Cargo.toml crates services tools script .github package.json pnpm-lock.yaml
shasum -a 256 <five frozen source files above>
```
