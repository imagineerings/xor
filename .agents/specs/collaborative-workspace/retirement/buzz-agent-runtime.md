# Buzz ACP, agent and MCP runtime retirement manifest

Date: 2026-08-25

Status: **PREPARED — REMOVAL HOLD**

This manifest prepares retirement of the duplicate Buzz ACP, agent and MCP execution processes without changing or deleting `projects/buzz`. It preserves the versioned `buzz` compatibility binary and standard ACP/MCP interoperability. It does not authorize source deletion, terminate an external ACP provider, or bypass the Task 47.1 traffic and rollback gates.

## Frozen source baseline

| Source | SHA-256 |
| --- | --- |
| `projects/buzz/crates/buzz-acp/Cargo.toml` | `cf6bf2bf05969995642e35b259286f11f891a2a128fdce9f4b1400f2ab84a2a2` |
| `projects/buzz/crates/buzz-acp/src/main.rs` | `81d7d727d7c598ed7ffc4b500a0cdc9cc82b06a8767313f791636f37e413c46d` |
| `projects/buzz/crates/buzz-agent/Cargo.toml` | `b51eea036e2b5afebd30923c2f9bfb85701ceb108e1a99827abe982b704ad4d4` |
| `projects/buzz/crates/buzz-agent/src/main.rs` | `fee279f5499c14a8459ab1a4feec6c03cd100814425ad90433c4667ec95d665f` |
| `projects/buzz/crates/buzz-dev-mcp/Cargo.toml` | `a0020312a2d1d706170e9970e8d90936239637137c177277799875bbbdecba00` |
| `projects/buzz/crates/buzz-dev-mcp/src/main.rs` | `e367aa0892a861fc05a2d0da81a6a01f1d048af8d6b1e7a51a0e0919bc93cce8` |
| `tools/buzz_compat/Cargo.toml` | `7b48d5bc89f2c150282e695f442a4c3ac2afdd498c398739c27089db0a17354c` |
| `tools/buzz_compat/src/main.rs` | `76827e3ba969d28d0c847fadfc779535f4e105b0da864ec1a3bdaef14aca6962` |

The supplied Buzz checkout is an external source baseline, not a Zed workspace dependency. Its source remains rollback and compatibility evidence until a separately approved deletion.

## Executable and process census

| Binary | Process role | Disposition | Reason |
| --- | --- | --- | --- |
| `buzz-acp` | Connects to the Buzz relay, queues channel events and heartbeats, owns an ACP subprocess pool and optionally configures an MCP subprocess per agent. | Retire after gates. | Its relay admission, queue, session, cancellation and observer responsibilities are now canonical Zed owners. External ACP agents remain supported through the standard protocol; this duplicate supervisor is not required. |
| `buzz-agent` | Serves ACP over stdio, owns in-memory sessions and histories, calls model providers and spawns MCP children. | Retire after gates. | Native Agent execution, session, provider and context-server owners supersede the duplicate loop. Its README explicitly declares no persistence, so there is no durable session store to migrate. |
| `fake-mcp` | Test-only child fixture declared by `buzz-agent`. | Retire with Buzz tests. | It is not a shipped compatibility surface and must not be mistaken for a retained production MCP server. |
| `buzz-dev-mcp` | Serves duplicate shell, file, search, edit, tree, image and todo tools, spawning shell and CLI helpers. | Retire after gates. | Task 28.5 maps compatible requests to native tools and ACP plan state while preserving native permissions, sandboxing and cancellation. Unsupported semantic drift remains fail-closed. |
| `buzz` | Versioned CLI compatibility shim in `tools/buzz_compat`; validates protocol/client versions and forwards to the canonical `zed` collaboration CLI. | **Retain.** | This is the only in-scope compatibility binary required after runtime retirement. It is a client shim and owns no queue, session, provider, tool process or collaboration state. |

The in-scope census is therefore five declared binaries: four retirement candidates and exactly one retained compatibility binary, `buzz`. External user-selected ACP agents such as `goose`, `codex-acp` and `claude-agent-acp` are neither repository binaries nor duplicate owners; protocol support for them remains. No Buzz-specific agent or MCP executable is retained to fill unsupported compatibility cases.

## One-executor process ownership audit

After cutover, the only collaboration executor owner is Zed Agent's `JobExecutionCoordinator`, with durable job authority in Collab. `RemoteExecutionCoordinator` is an adapter around that same coordinator for provider-backed execution; it does not form a second executor. The complete process path is:

1. Collab authenticates and authorizes the community event or job, persists the job, and grants one generation-fenced executor lease.
2. Agent maps an authorized mention or job to exactly one native ACP session through `CollaborationSessionRegistry`.
3. `JobExecutionCoordinator` checks the assigned executor, acquires the canonical lease and session claim, invokes the native local runtime once, and publishes the terminal job transition under the same fence.
4. For remote execution only, `RemoteExecutionCoordinator` resolves protected provider configuration after the canonical claims, delegates launch to Remote, and binds cleanup to the same job/session/provider identity.
5. Native Agent tools and context-server infrastructure retain permission, sandbox, cancellation and child-process ownership. Observer adapters publish bounded lifecycle metadata without a second transcript.

| Former duplicate responsibility | Canonical owner/evidence | Retirement result |
| --- | --- | --- |
| Relay mention and control admission | Collab authorization plus Tasks 28.1 and 28.3 | `buzz-acp` cannot independently admit or consume work. |
| Per-channel queue and agent pool | Durable Collab job/workflow admission, executor leases and Agent coordinator | No in-memory Buzz queue or pool remains a dispatch authority. |
| ACP session create/resume/cancel | Agent `CollaborationSessionRegistry`, proven by Tasks 28.2 and 28.6 | One session binding and executor generation; no Buzz session cache remains authoritative. |
| Tool selection and MCP child lifecycle | Native Agent tool registry, permission prompts, context-server lifecycle and Task 28.5 mappings | `buzz-dev-mcp` and `buzz-agent` MCP children are unnecessary. `replace_all`, recursive tree and remote/data-URL image requests continue to fail closed rather than resurrecting a legacy server. |
| Provider request and remote process lifecycle | Agent provider configuration, `JobExecutionCoordinator`, `RemoteExecutionCoordinator` and Remote L1-L3 lifecycle | Task 33.6 proves discovery, deploy, hostile-output handling and process-tree cleanup without a Buzz executor fallback. |
| Agent output, status and observer frames | Canonical job transitions, native ACP session updates, collaboration activity/audit and bounded NIP-AO observer adapter | No Buzz-local transcript, completion writer or observer archive remains live. |

Task 28.6's 22-case focused adapter matrix and three lifecycle-conformance scenarios cover reentrancy, crash fencing, retry, cancellation, tool ownership, observer deduplication and resource release. Task 33.6's three provider cases cover L1-L3 launch, hostile output and descendant cleanup. These are the executable conformance gates for removing duplicate process ownership; the Buzz crates are deliberately not imported into the Zed graph.

## State disposition audit

| Buzz runtime state | Canonical disposition | Import requirement |
| --- | --- | --- |
| `buzz-acp` channel queues, pool slots, subscriptions, heartbeat state and cached ACP sessions | Ephemeral dispatch state. New work is admitted through durable Collab jobs/workflows; in-flight work must drain or be cancelled before shutdown. | None. Never synthesize durable jobs from an in-memory queue during retirement. |
| `buzz-agent` sessions, bounded histories, active tool calls and cancellation handles | Ephemeral by the runtime's documented no-persistence contract. Canonical Agent sessions/jobs own new work. | None. Drain or cancel; do not copy an in-memory transcript into canonical history. |
| Provider configuration, OAuth caches and credential-shaped environment values | Canonical Agent settings and protected credential references. Tasks 17.9 and 30.5 preserve staged/imported state and receipts. | Only the existing privacy-classified import path; raw secrets must not enter this manifest or a retirement script. |
| Managed-agent/persona/team/memory/usage records consumed by the old runtime | Canonical stores and verified Task 30.5 receipts. | All applicable source/profile receipts must pass before source deletion. |
| `buzz-dev-mcp` todo list | Canonical ACP plan state under Task 28.5, not a second durable store. | None beyond active-session drain. |
| Executable source, protocol fixtures and conformance behavior | Retain as frozen migration/compatibility evidence until Task 47.5 preserves artifacts, licenses and history. | Preserve; do not delete as part of process shutdown. |

No authoritative durable state exists only inside these runtime processes. Ephemeral work is drained or cancelled; durable records use their canonical owners and verified import receipts.

## Repository dependency and launch audit

A dependency search over the root manifest, lockfile and crate/service/tool manifests finds no Zed dependency on `buzz-acp`, `buzz-agent` or `buzz-dev-mcp`. Their remaining repository references are source inventory, specification, compatibility and conformance evidence. The retained `tools/buzz_compat` crate is a workspace tool and forwards to `zed`; it does not execute ACP, MCP or model-provider work.

A deployment-specific retirement change must additionally prove that no service definition, desktop launch configuration, package, container, supervisor or live process starts the four retirement candidates. A source-only search cannot replace that live process check.

## Proposed retirement change

Once every gate below is satisfied, a separately approved retirement change may remove the three Buzz runtime crates and their test-only `fake-mcp` binary from build, package, deployment and launch configuration, then stop their processes after admitted work drains. It must retain `tools/buzz_compat`, standard ACP/MCP interoperability, protocol/conformance fixtures, licenses, source history and migration evidence. This manifest performs no source or process removal.

Required gates:

- Tasks 28.6 and 33.6 remain passing; native tool, session, provider and process-cleanup conformance has no legacy fallback.
- Task 47.1 records zero direct legacy writes and approved agent-runtime adapter-read, adapter-write, active-client, observation-window and rollback-window thresholds pass in the target deployment.
- Every applicable managed-agent source/profile has the verified Task 30.5 import receipt; original source remains intact.
- Live process and launch-configuration inspection finds no new work routed to `buzz-acp`, `buzz-agent` or `buzz-dev-mcp`, then confirms all accepted legacy work has drained or been explicitly cancelled.
- `tools/buzz_compat` continues to build and its version/protocol compatibility tests pass.
- Task 47.5 preserves required artifacts, licenses and source history, and Task 47.6 finds no duplicate process, state or write owner.
- A human explicitly approves the source-retirement/deletion gate.

Until then, disposition is **HOLD**: preserve all Buzz runtime source unchanged, keep the `buzz` compatibility binary, and do not alter production routing, launch configuration or running processes.

## Validation commands

```text
rg -n '^(\[\[bin\]\]|name = "(buzz-acp|buzz-agent|fake-mcp|buzz-dev-mcp|buzz)")' <four Cargo.toml files>
rg -n 'buzz-acp|buzz_agent|buzz-agent|buzz-dev-mcp|buzz_dev_mcp' Cargo.toml Cargo.lock crates/*/Cargo.toml services/*/Cargo.toml tools/*/Cargo.toml
shasum -a 256 <eight frozen source files above>
```
