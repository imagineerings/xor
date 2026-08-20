# Agent, provider and MCP threat model

## Scope and authority

This review covers the execution boundaries created when collaboration events reach Sim's native ACP runtime, model providers, MCP servers, native tools, managed subprocesses, signed jobs, remote-provider binaries, remote substrates and shared compute. It implements the security review required by acceptance criteria 11.1, 11.5, 19.1 and 19.2 for CAP-021, CAP-022 and CAP-034.

The canonical owners remain those approved in the collaborative-workspace design:

- `crates/agent`, `crates/acp_thread`, `crates/agent_servers` and existing tool-permission/UI paths own sessions, ACP protocol, prompts, tool authorization, transcript state and cancellation.
- `crates/sandbox` and the final native tool implementations own operating-system, filesystem and network enforcement. A compatibility adapter never becomes an authorization authority.
- `crates/credentials_provider` and `crates/sim_credentials_provider` own secret retrieval. Agent, persona, provider and project configuration persist references, never secret values.
- `crates/remote` owns provider discovery/protocol/process bounds and remote transport. `crates/agent` binds one canonical job/session executor to it.
- `crates/collaboration_domain` and `crates/collab` own signed job authorization and the exactly-one executor lease. Presence is status, not authority or a management channel.

This document does not redesign tenant admission/signing (Task 4.1), workflows/webhooks (Task 4.5), media (Task 4.3), push (Task 4.6), voice (Task 4.7) or mesh trust policy (Task 4.8/ADR-006). It does cover how those systems may enter an agent executor and requires them to cross the same permission and lifecycle boundaries.

## Source evidence

- `projects/buzz/docs/remote-agents.md` defines the hostile provider boundary, provider output caps, secret separation, presence-only status, reconciliation and shutdown invariants, and eight explicit implementation defects.
- `.agents/specs/goose-migration/security-permissions/requirements.md` requires adversarial-input inspection, egress inspection, deterministic permissions, conservative optional read-only judgment and private, scoped permission persistence.
- `.agents/specs/goose-migration/security-permissions/design.md` keeps Sim's existing permission and sandbox owners canonical.
- `.agents/specs/goose-migration/security-permissions/tasks.md` remains the implementation plan for content inspection and general permission hardening. Collaborative Workspace reuses it; it does not create a second scanner, permission store or confirmation UI.
- Current Sim integration points include `crates/agent/src/tool_permissions.rs`, `crates/agent/src/tools/tool_permissions.rs`, `crates/acp_thread/src/connection.rs`, `crates/agent_servers/src/acp.rs`, `crates/sandbox/src/sandbox.rs` and `crates/remote/src/remote_client.rs`.

## Protected assets

1. Human intent represented by a scoped permission decision.
2. Project/worktree files, Git state, terminals, processes, network access and external accounts.
3. Sim account credentials, Nostr signing keys, provider tokens, MCP credentials and remote-substrate credentials.
4. Canonical ACP transcript, agent session, job, lease, result, presence and activity provenance.
5. Private persona, team, environment, memory, snapshot and usage state.
6. Tenant/community isolation and the nonexistence of private conversations, jobs or resources.
7. Host and remote compute availability, including process, CPU, memory, storage, descriptor and queue capacity.
8. Auditability: truthful permission, execution, cancellation, failure and cleanup outcomes with no secret-bearing logs.

## Adversaries and trust assumptions

- Collaboration content, repository files, terminal output, Web content, model output and protocol frames are hostile even when signed or produced by a member.
- ACP agents, MCP servers, model providers, remote-provider binaries, custom runtime images and mesh peers may be buggy or malicious. A valid signature identifies a principal; it does not make content or code safe.
- A remote provider must receive some execution secrets to deploy an agent. After that deliberate handoff, Sim cannot contain a malicious provider or substrate administrator. Sim must make the trust decision explicit, minimize the values sent, pin the executable identity used for negotiation and deployment, and prevent echoed secrets from reaching state or UI.
- The operating system sandbox, project authorization, credentials provider and canonical job lease are trusted enforcement points. UI labels, model judgments, tool annotations, presence and provider claims are not.
- Content inspection can reduce risk but has false positives and false negatives. Scanner absence, timeout or failure can never grant tool, filesystem, network, credential or execution authority.
- A user-approved persistent permission is trusted only within its normalized user/project/tool/argument scope and expiry. It is not transferable across tenants, worktrees, identities, sessions or materially different arguments.

## Security invariants

### INV-AWP-01 — instructions never confer authority

Text from a message, repository, model, MCP server, tool result or provider response cannot grant a permission, select a credential, change tenant, widen a sandbox or acquire an executor lease. It can request an action only.

### INV-AWP-02 — authorize at the final executor

Every tool call is revalidated at the native executor after compatibility translation. Deterministic hard denials, stored denials, session mode, normalized tool/argument policy and user confirmation run before execution. Optional model judgment may classify only submitted request IDs as strictly read-only; it may never approve writes, open-world requests or destructive actions. Failure or ambiguity yields confirmation or denial, never automatic approval.

### INV-AWP-03 — bounded at every crossing

Every protocol frame, prompt, schema, argument, stream, command output, provider stdout/stderr, queue and deadline has a configured bound before allocation or persistence. Truncation is explicit and cannot turn a partial structured value into success. An absent bound is a configuration/readiness failure owned by Task 4.4.

### INV-AWP-04 — secrets are late-bound and non-durable

Runnable configuration stores credential references only. The canonical credentials provider resolves a secret immediately before the narrow operation that needs it. Child environments start from an allowlist, authoritative identity variables override user input, and output redaction includes every resolved value and every launch-layer secret. Secret values never enter transcripts, activity, provider configuration, public events, snapshots, fingerprints, command lines or error strings.

### INV-AWP-05 — cancellation is a tree

Session/job cancellation closes pending permission requests, stops input, cancels tool/network work, signals every owned child, waits within a shared grace budget, force-kills survivors, closes pipes/subscriptions, joins reader/reaper tasks and emits exactly one terminal result. Dropping a future is not proof of cleanup. A task that outlives its owner must be deliberately retained or detached with logged errors.

### INV-AWP-06 — exactly one authoritative executor

A collaboration event, job or session maps to one canonical execution identity and, where distributed, one executor lease. Duplicate/replayed input may resume or observe that execution; it may not launch a second runtime. Presence, observer frames and provider `agent_id` values cannot acquire or transfer ownership.

### INV-AWP-07 — local and remote policy equivalence

Local, provider-backed and mesh-backed execution use the same identity, permission envelope, tool adapters, cancellation state and terminal-result rules. Substrate capabilities and limitations are surfaced explicitly. Unsupported remote semantics fail before mutation; they do not silently fall back to local or another provider.

### INV-AWP-08 — hostile output stays data

Model, MCP, terminal and provider output is bounded, structurally validated, redacted and escaped before storage or rendering. A non-zero provider exit is failure regardless of parseable stdout. Output cannot synthesize permission responses, lifecycle transitions or terminal success without the corresponding canonical operation ID and state transition.

## Threat register

| Threat ID | Attack or failure | Required control | Negative/recovery evidence |
|---|---|---|---|
| T-AWP-001 | Indirect prompt injection in messages, files, search results or tool output asks the agent to bypass policy | Treat content as instructions only; deterministic permission and sandbox checks remain authoritative; optional adversary scanner reports without granting authority | Goose security tasks 1–5 and 9–10; Tasks 28.3, 28.5, 28.6, 45.2 |
| T-AWP-002 | A model or MCP server forges an allow response, tool identity or read-only annotation | Correlate stable request/tool IDs, validate submitted IDs and known schemas, and decide at the final executor | Tasks 28.5, 28.6; Goose permission tasks 7–10 |
| T-AWP-003 | Shell interpolation, chaining or malformed arguments hide a destructive command | Parse the concrete command, reject unsupported/ambiguous syntax, preserve hard denials and show literal arguments for confirmation | Task 28.5; existing `crates/agent/src/tool_permissions.rs`; Task 45.2 |
| T-AWP-004 | Path traversal, symlink replacement or encoded path escapes the project after approval | Resolve against canonical worktree roots at execution, distinguish display path from authority path, and recheck mutation targets at use time | Task 28.5; existing `crates/agent/src/tools/tool_permissions.rs`; Task 45.2 |
| T-AWP-005 | A read-looking network or search tool exfiltrates secrets or reaches private/local services | Separate network permission, destination policy and credential scope from read-only classification; apply egress/SSRF controls at the HTTP owner | Tasks 28.5, 28.6, 45.2; Goose security tasks 3–5 and 9–10 |
| T-AWP-006 | An infinite prompt, stream, schema, recursive value or output exhausts memory/disk/UI | Enforce bytes/items/depth/deadline before accumulation; truncate with an explicit incomplete marker and cancel upstream work | Tasks 28.4–28.6, 33.2, 33.6, 45.2 |
| T-AWP-007 | Tool/model/provider output contains keys, control sequences, deceptive links or forged lifecycle JSON | Literal-value and pattern redaction, structured parsing, terminal/control escaping, canonical provenance and generic untrusted rendering | Tasks 28.4, 32.4–32.6, 33.2, 33.6, 45.2 |
| T-AWP-008 | Child keeps stdout/stderr open, forks grandchildren, hangs after cancellation or deadlocks full pipes | Concurrent bounded pipe readers, process group/job ownership, deadline, graceful signal, force kill and joined cleanup | Tasks 28.6, 33.2, 33.3, 33.6, 45.5 |
| T-AWP-009 | Cancellation races with permission grant, tool completion or remote result and produces an effect after cancel | Operation IDs plus a monotonic cancelling/terminal state; close authorization first; executor checks cancellation immediately before the effect; reconcile one terminal outcome | Tasks 28.2, 28.6, 31.5–31.7, 33.5, 33.6 |
| T-AWP-010 | An always-allow decision leaks across project, user, tool version or changed arguments | Normalize and scope the stored decision; minimize readable context; expire and atomically persist it; corruption recovers visibly and fail-closed | Goose permission tasks 6–10; Task 28.5 |
| T-AWP-011 | Optional model judge is injected, times out, returns unknown IDs or labels a write as read-only | Send minimum labeled data; accept only submitted IDs; allow only strict read-only; every error/unknown/ambiguity approves nothing | Goose permission tasks 7, 9 and 10; Task 28.5 |
| T-AWP-012 | A malicious MCP server advertises colliding tools or mutates a schema after approval | Namespace server/tool identity, freeze validated schema per request, reject duplicate/unknown/version-changed tools and reauthorize normalized arguments | Tasks 28.5, 28.6, 45.2 |
| T-AWP-013 | MCP or ACP executable discovery selects a shadowed/replaced binary | Show resolved provenance, reject malformed/duplicate ambiguity under policy, bind approval to executable identity/version and revalidate at launch | Tasks 28.6, 33.1, 33.6 |
| T-AWP-014 | Provider binary changes between compatibility probe and nsec-bearing deploy | Resolve once, stage private non-writable bytes, digest, run `info` and `deploy` on the same staged artifact, reject absent/incompatible protocol before secret transfer | Tasks 33.1, 33.2, 33.6 |
| T-AWP-015 | Provider emits malformed/oversized/secret-bearing output or exits non-zero after printing success | One JSON object, strict schema, 1 MiB stdout, 64 KiB stderr, concurrent drain, redaction and non-zero-is-failure | Tasks 33.2, 33.6 |
| T-AWP-016 | Provider config or public agent state persists a credential | Flat scalar provider config, at most 20 fields/64 KiB, secret-shaped key rejection, reference-only agent config and exhaustive public-projection redaction | Tasks 29.3, 29.4, 33.4, 33.6 |
| T-AWP-017 | User env overrides identity, owner, permission, presence, cancellation or inactivity policy | Reserved authoritative keys, POSIX-shaped env names, shared launch resolver and authoritative-last precedence; no legacy re-merge | Tasks 29.3, 33.4, 33.6 |
| T-AWP-018 | Full desktop environment leaks into ACP, MCP or provider children | Start from a minimal allowlist; explicitly add required paths and approved ambient substrate selectors; never inherit unrelated credentials | Tasks 28.6, 33.1, 33.4, 33.6 |
| T-AWP-019 | Stale or forged presence is treated as a kill/control/authorization channel | Accept self-signed, tenant-scoped bounded presence as status only; use canonical session/job commands for control; show staleness and disconnect | Tasks 21.4, 33.5, 33.6 |
| T-AWP-020 | Replay or a provider race creates two agent/job executors | Stable source IDs, idempotent session mapping, exactly-one lease, identity-derived reconciliation and strict live no-op | Tasks 28.2, 31.1–31.7, 33.3, 33.5, 33.6 |
| T-AWP-021 | Delegation cycles or fan-out consumes unbounded jobs, tokens or compute | Authorize depth/scope, cap children/resources, preserve ancestry, cancel descendants and reject cycles before lease acquisition | Tasks 31.3, 31.6, 45.5 |
| T-AWP-022 | Mesh advertisement spoofing, revoked capacity or failure triggers execution on an unapproved peer | ADR-006 trust policy, signed/fenced advertisement, canonical lease and no silent fallback | Tasks 2.6, 4.8, 41.1–41.5, 45.2 |
| T-AWP-023 | Intentional shutdown is restarted or force-kill skips offline/final cleanup | Pin clean-exit semantics; supervisor never restarts an intentional exit; one shared shutdown deadline reserves finalization time | Tasks 33.3, 33.6, 45.5 |
| T-AWP-024 | Private memory, prompt, tool arguments or raw observer data enters public activity or logs | Keep ciphertext/private state with canonical owners; project redacted semantic summaries; raw encrypted/private values remain inaccessible | Tasks 29.4, 30.2, 30.6, 32.1–32.6, 45.2 |
| T-AWP-025 | A scanner/classifier outage is configured fail-open and is mistaken for permission | Scanner result is advisory to the authority pipeline; no scanner mode bypasses deterministic permission, sandbox, tenant or credential controls | Goose security tasks 4–5 and 9–10; Tasks 28.5, 45.2 |

## Executor-boundary checklist

Every boundary below has an input bound, output bound, permission control, cancellation/cleanup contract, secret rule and assigned tests. Numeric limits not already frozen by Buzz are set by Task 4.4; until configured, the path is unavailable rather than unlimited.

### EX-01 — collaboration ingress to ACP prompt

- **Owner:** `crates/agent/src/collaboration_mention.rs` and the canonical session mapping in `crates/agent/src/collaboration_session.rs`.
- **Untrusted input:** signed messages, mentions, thread/job coordinates, attachments and quoted context.
- **Input bound:** validate event/frame size first; authorize tenant, membership, target identity and supported mention type; cap expanded context, attachments and queued prompts before session allocation.
- **Output bound:** one idempotent prompt command or a typed denial/busy result per source ID; no raw private event in public activity.
- **Permission control:** message authors may request a prompt only within membership/team policy; they acquire no tool permission. Tool authority remains with EX-05–EX-08.
- **Cancellation and cleanup:** a revoked membership, deleted source, session cancellation or disconnect stops undispatched work; replay resolves to the existing command/session.
- **Secret control:** ingress never resolves credentials and cannot carry authoritative environment values.
- **Assigned tests:** Tasks 28.2, 28.3 and 28.6 cover unauthorized actor, duplicate event, busy session, cancellation and exactly-one execution.

### EX-02 — native ACP agent-server process and wire

- **Owner:** `crates/agent_servers/src/acp.rs`, `crates/acp_thread/src/connection.rs` and native session ownership in `crates/agent`.
- **Untrusted input:** agent executable bytes, JSON-RPC/ACP frames, session updates, permission requests, terminal requests and stderr.
- **Input bound:** allowlisted executable/config, negotiated protocol, bounded line/frame/depth/schema and deadline; malformed IDs or updates fail the scoped request, not the app.
- **Output bound:** bound stderr history, streaming accumulator, terminal output and transcript item size; accept only known correlated updates and preserve explicit truncation.
- **Permission control:** ACP permission requests are untrusted requests routed through Sim's existing authorization UI/store; agent-provided option labels never decide authority.
- **Cancellation and cleanup:** session cancel closes pending responders/permission prompts, sends protocol cancellation, kills owned terminals/child process after grace, drains pipes and joins reader tasks.
- **Secret control:** child environment is allowlisted; credentials are injected only for approved runtime needs and stderr/transcript redaction runs before persistence.
- **Assigned tests:** Tasks 28.2, 28.4 and 28.6 plus Goose permission tasks 6–10 cover malformed frames, permission cancellation, crash, stderr redaction and process cleanup.

### EX-03 — model-provider request and streaming response

- **Owner:** existing Sim language-model/provider and credentials-provider integrations used by the native agent runtime.
- **Untrusted input:** prompt/context assembled from users, repositories, tools and private agent state; provider stream/output and usage metadata.
- **Input bound:** cap prompt bytes/tokens, attachments, tool schemas and request deadline before network send; select credentials by explicit provider/account reference.
- **Output bound:** cap tokens, bytes, tool-call count, nesting and streaming buffer; reject malformed or unknown tool calls without speculative repair that widens authority.
- **Permission control:** model output can propose EX-05 calls only. It cannot approve its own call, select a broader credential or alter the session permission mode.
- **Cancellation and cleanup:** cancellation aborts the HTTP/stream request, closes the response body and prevents late chunks from mutating terminal session state.
- **Secret control:** send only provider credentials and content required for the selected request; disclose external transmission; redact provider errors and never log headers/bodies containing secrets.
- **Assigned tests:** Goose security tasks 2–5, 7 and 9–10; Tasks 28.6 and 45.2 cover egress, malformed streams, cancellation and unknown tool calls.

### EX-04 — MCP server discovery, launch and protocol

- **Owner:** existing MCP configuration and `crates/agent_servers` integration; Buzz mappings are compatibility only.
- **Untrusted input:** server executable/config, advertised capabilities/tools/resources, schemas, notifications and server output.
- **Input bound:** validate executable provenance and protocol version; cap server count, tool/schema bytes, nesting, notifications and handshake/request deadlines; reject duplicate names and schema mutation.
- **Output bound:** bounded stdout/stderr and protocol messages; tool/resource payloads carry explicit truncation and remain untrusted content.
- **Permission control:** registration never grants execution. Every request crosses EX-05 and the final native executor; server annotations are advisory.
- **Cancellation and cleanup:** session/project removal cancels requests, closes transport, signals the process group, force-kills after grace and joins pipe tasks.
- **Secret control:** per-server credential references resolve into a minimal environment; one server never receives another server's or the agent identity's secrets by default.
- **Assigned tests:** Tasks 28.5 and 28.6 cover tool collisions, malformed schemas, timeout, cancellation, secret-bearing stderr and orphan cleanup.

### EX-05 — Buzz MCP compatibility mapping and permission decision

- **Owner:** `crates/agent/src/buzz_tool_compat.rs`, existing `crates/agent/src/tool_permissions.rs`, ACP confirmation and permission persistence.
- **Untrusted input:** Buzz shell/read/edit/search/tree/image/todo names, arguments, annotations and permission options.
- **Input bound:** exact tool/version map; typed argument schema; bounded strings/lists/depth; unknown fields/tools or invalid paths are denied errors.
- **Output bound:** one typed native tool result with explicit success/denial/failure/cancel/truncation; no compatibility-specific transcript store.
- **Permission control:** normalize the native tool and concrete arguments, then apply hard denial, stored denial, confirm, allow and default precedence. Optional judge can only return validated strictly read-only IDs.
- **Cancellation and cleanup:** cancellation closes the permission request and prevents dispatch; after dispatch it propagates to the selected native executor.
- **Secret control:** permission-readable context is minimized/redacted before private scoped persistence; compatibility results are redacted before observer publication.
- **Assigned tests:** Task 28.5 requires tool-by-tool success, denial, invalid path, bounded output and cancellation; Task 28.6 and Goose permission tasks 6–10 cover persistence and lifecycle.

### EX-06 — filesystem, project and Git-affecting native tools

- **Owner:** existing native tools in `crates/agent/src/tools/`, `project`, `git` and `crates/sandbox`.
- **Untrusted input:** paths, ranges, patches, replacement text, symlinks and repository state selected by an agent.
- **Input bound:** canonicalize within the current worktree/project, constrain file/range/patch sizes, reject traversal/symlink escape and revalidate the target immediately before mutation.
- **Output bound:** cap file bytes, matches, directory entries, diagnostics and diff size with continuation metadata.
- **Permission control:** external/sensitive/write/delete/move operations require their native policy and user decision; read permission never upgrades to write.
- **Cancellation and cleanup:** edits use atomic/native transaction behavior where available; cancellation before commit has no effect, and an interrupted committed effect reports its actual outcome rather than rollback fiction.
- **Secret control:** protect settings, skills, credential files and external paths; redact secret-looking file content from persisted permission context and public activity.
- **Assigned tests:** Task 28.5 and Task 45.2 cover traversal, symlink race, sensitive settings, denial, bounded reads/diffs and truthful cancellation outcomes.

### EX-07 — terminal, shell and long-lived terminal processes

- **Owner:** native terminal tool, terminal entity and `crates/sandbox`.
- **Untrusted input:** command, arguments, cwd, environment requests, stdin and agent-directed process control.
- **Input bound:** reject hidden interpolation/substitution in permission-protected commands, parse supported chaining, bound command/env/stdin and require an explicit cwd within scope.
- **Output bound:** enforce byte/line/time limits while draining stdout and stderr concurrently; truncation does not imply successful exit.
- **Permission control:** preserve non-overridable catastrophic denials, then deterministic terminal policy and confirmation. Sandbox/network/write relaxations are separately explicit.
- **Cancellation and cleanup:** own the process group/job, close stdin, TERM, bounded wait, KILL descendants, reap and emit one exit/cancel state; terminal IDs are scoped to the session.
- **Secret control:** minimal environment; secrets never appear on command lines; output and errors are redacted before transcript/activity/log storage.
- **Assigned tests:** Task 28.5, Task 28.6 and Task 45.2 cover command ambiguity, denial, output flood, hanging descendants, cancellation and secret echo.

### EX-08 — network, fetch, Web/search and other external tools

- **Owner:** existing native HTTP/network tool owners and sandbox network policy.
- **Untrusted input:** URLs, redirects, headers, request bodies, remote content and response metadata.
- **Input bound:** allow supported schemes, canonicalize host/IP after resolution, apply private-range/redirect policy at every hop, and cap request/body/response/decompression/time.
- **Output bound:** cap bytes/items/text extraction and treat active content as inert data before model or UI use.
- **Permission control:** external network access and credential use require destination-scoped policy; optional read-only judgment cannot approve open-world requests.
- **Cancellation and cleanup:** abort request/stream, release connection/body and discard late results after terminal cancellation.
- **Secret control:** destination-scoped credentials only; strip cross-origin auth on redirect; egress inspection blocks/redacts configured sensitive data.
- **Assigned tests:** Task 28.5, Goose security tasks 2–5 and 9–10, and Task 45.2 cover SSRF, redirect, oversized response, cancellation and exfiltration.

### EX-09 — local managed-agent pool and child lifecycle

- **Owner:** Sim native agent/session runtime and `agent_servers`; no Buzz harness remains as a second owner.
- **Untrusted input:** runtime command/args, persona/team configuration, child output and lifecycle signals.
- **Input bound:** resolve one approved runtime descriptor, cap parallelism/turn/idle lifetime and validate environment key/value shape before spawn.
- **Output bound:** bound per-turn response, logs, usage and observer summaries; unknown lifecycle output maps to a generic untrusted event.
- **Permission control:** each child inherits the session permission envelope but not prior one-shot grants; spawning additional agents/jobs requires explicit delegation policy.
- **Cancellation and cleanup:** one cancellation tree owns pool slots, active turns, children, terminals and subscriptions; intentional exit is terminal and cleanup has a shared deadline.
- **Secret control:** shared launch resolver enforces authoritative-last identity/policy keys and minimal per-runtime credentials; public persona/team projections exclude runnable secrets.
- **Assigned tests:** Tasks 28.6, 29.3, 29.4 and 45.5 cover parallel bounds, restart, cancellation, public projection and child cleanup.

### EX-10 — signed job and delegated-child execution

- **Owner:** `crates/collaboration_domain` job policy, `crates/collab` repository/lease and `crates/agent` job executor.
- **Untrusted input:** signed job transitions, ancestry, requested resources, team/member claims and child-job requests.
- **Input bound:** exact state machine/version, authorized scope, maximum ancestry/depth/children/resources and idempotent operation IDs before lease acquisition.
- **Output bound:** bounded progress frequency/payload and exactly one terminal result; partial child failures are summarized without leaking private child context.
- **Permission control:** signer/team authorization permits job transitions, not unrestricted tools. The leased session still applies EX-05–EX-08.
- **Cancellation and cleanup:** parent cancel recursively marks/propagates to children and runtime; lease expiry/recovery cannot create a second terminal effect.
- **Secret control:** jobs carry secret references/capabilities scoped to the executor; signed/public events never contain values.
- **Assigned tests:** Tasks 31.1–31.7 and 45.5 cover illegal transitions, cycles, excessive depth, concurrent leases, cancellation, retry and partial failure.

### EX-11 — remote-provider binary discovery and invocation

- **Owner:** `crates/remote/src/agent_provider_{discovery,protocol,lifecycle}.rs` with credential binding in `crates/agent`.
- **Untrusted input:** executable candidate, `info` schema, provider stdout/stderr/exit status, config and substrate conditions.
- **Input bound:** provider ID grammar; flat scalar config at most 20 fields and 64 KiB; one typed request; `info` deadline 10s and deployment deadline no greater than the frozen 600s compatibility ceiling.
- **Output bound:** exactly one JSON response; stdout at most 1 MiB and stderr at most 64 KiB, drained concurrently; strict response schema and non-zero-is-failure.
- **Permission control:** user explicitly approves provider executable/trust and target configuration; provider results cannot acquire job/session/tool authority.
- **Cancellation and cleanup:** cancel closes stdin, terminates the staged provider process group, drains/reaps it and reconciles any known operation state; no detached pipe readers.
- **Secret control:** resolve once, privately stage/digest, negotiate supported protocol on the staged bytes before secret transfer, deploy the same bytes, then remove staging. Config contains no secrets; request/launch secrets are non-durable and all candidate values feed redaction.
- **Assigned tests:** Tasks 33.1–33.4 and 33.6 cover shadowing, incompatible/missing version, binary swap, malformed/oversized/secret output, hang, cancellation and config/secret separation.

### EX-12 — remote substrate harness and agent process

- **Owner:** canonical remote-agent lifecycle bound to the same Sim job/session identity; substrate adapter implements but does not own transcript or permission state.
- **Untrusted input:** remote image/runtime, substrate state, container/process output, presence and provider-reported instance ID.
- **Input bound:** approved immutable image/provenance, resource/lifetime limits, nonempty identity/owner and bounded launch environment before mutation; unsupported local-loopback mesh provider is rejected.
- **Output bound:** bounded lifecycle/status/result frames with provenance; substrate logs are redacted and externally retained according to policy, never treated as control.
- **Permission control:** remote runtime receives the same permission envelope and tool restrictions as local execution; substrate capability differences are visible and cannot silently broaden access.
- **Cancellation and cleanup:** canonical cancellation reaches the harness signal target, reserves finalization time, cleans children and produces offline/terminal state; supervisor never restarts intentional clean exit.
- **Secret control:** unique immutable generation secret, minimal environment, no service-account token by default, non-root/no privilege escalation/drop capabilities and no host namespace/path access.
- **Assigned tests:** Tasks 33.3–33.6 and 45.5 cover identity refusal, at-most-one instance, stale presence, SIGTERM/offline, restart policy, residue and complete cleanup.

### EX-13 — relay-mesh/shared-compute execution

- **Owner:** `crates/remote` mesh protocol/scheduler plus the canonical job lease and remote execution binding.
- **Untrusted input:** peer identity, signed advertisement, resource claims, version, job offer/result and partition/reconnect sequence.
- **Input bound:** ADR-006-approved peers/capabilities, advertisement size/expiry, job resource ceiling and scheduler queue/fairness limits before lease.
- **Output bound:** bounded progress/result and liveness; stale/replayed/unknown-version messages are rejected and never refresh authority.
- **Permission control:** peer eligibility plus canonical job authorization and lease are all required; lack of capacity or provider failure never silently falls back.
- **Cancellation and cleanup:** revoke/cancel/partition expires the lease according to policy, stops work where reachable and reconciles late results without duplicate effects.
- **Secret control:** use job-scoped capabilities, not account/Nostr root keys; advertisements and logs contain no credentials or private task payload.
- **Assigned tests:** Tasks 2.6, 4.8, 41.1–41.5, 45.2 and 45.5 cover spoofing, replay, revoke, partition, fairness, no fallback and cleanup.

### EX-14 — observer, activity, transcript and result publication

- **Owner:** canonical ACP transcript and `agent_ui` activity projection; NIP-AO is an adapter, not a second transcript.
- **Untrusted input:** provider/model/tool/lifecycle output and compatibility observer frames arriving out of order or duplicated.
- **Input bound:** accept known correlated source/operation IDs, bounded semantic fields and encrypted/raw fallback metadata; reject unauthorized observers.
- **Output bound:** bounded/redacted semantic summary plus progressive detail; raw private/encrypted content is not made public or searchable.
- **Permission control:** observer access is separately authorized and read-only; activity actions route back through canonical commands and permissions.
- **Cancellation and cleanup:** reducer reconciles cancellation, timeout, disconnect and late terminal updates into one truthful state without resurrecting execution.
- **Secret control:** redact before persistence and again before compatibility publication; terminal/provider errors never carry resolved secret values.
- **Assigned tests:** Tasks 28.4, 28.6, 32.1–32.6 and 45.2 cover redaction, unauthorized observer, reorder, duplicate, cancellation and late terminal results.

## Provider compatibility and lifecycle boundary

Buzz provider protocol v1 defines `info` and `deploy`, deliberately has no `undeploy`, and treats relay presence as the only post-deploy status signal. Task 33.3 names a canonical `deploy/inspect/terminate` lifecycle. This is a compatibility tension, not permission to add undocumented legacy wire calls:

- The Sim lifecycle abstraction may expose inspect and terminate to callers.
- A Buzz v1 adapter reports inspection from canonical job/session state plus self-signed presence with an explicit staleness marker; it never presents presence as authoritative substrate state.
- Termination first uses the owner-authorized canonical cancellation/`!shutdown` path. Provider/substrate cleanup is invoked only when a negotiated provider version explicitly supports it and the user authorized the destructive scope.
- If reliable termination/inspection is unavailable, the UI reports that limitation and recovery action. It cannot return a fabricated success.
- Deleting a configuration never silently orphans a live remote process. Compatibility-era orphan confirmation and cleanup evidence are required by Task 33.6.

No architecture or milestone change is made here. Task 33.3 must preserve the above distinction when it defines the canonical adapter behavior.

## Frozen Buzz remote-provider defects

The following are requirements for the port, not behaviors to reproduce:

| Defect | Buzz evidence | Required consolidated disposition | Test owner |
|---|---|---|---|
| RP-DEF-01 | Windows provider suffix pollutes the provider ID | Normalize platform executable suffixes before ID validation | 33.1, 33.6 |
| RP-DEF-02 | Provider inherits the full desktop environment and GUI PATH may omit auth helpers | Use a minimal explicit environment and diagnose missing approved helpers without leaking unrelated credentials | 33.1, 33.4, 33.6 |
| RP-DEF-03 | Deploy bypasses the shared launch resolver; launch-layer secrets miss redaction | One local/remote resolver; redact secrets from every launch tier; reject remote-only semantic drift | 33.4, 33.6 |
| RP-DEF-04 | Inactivity reaper is absent and the obvious maintenance tick is pool-readiness gated | Pool-independent lifetime timer and cancellation tree; local default remains disabled | 33.3, 33.6 |
| RP-DEF-05 | Deploy does not check protocol version before sending the private key | Same-staged-bytes pre-secret negotiation gate | 33.1, 33.2, 33.6 |
| RP-DEF-06 | Intentional clean-exit code is emergent, not tested | Pin intentional exit as terminal-success semantics before any restart-on-failure policy | 33.3, 33.6 |
| RP-DEF-07 | Shutdown cleanup can exceed the declared 60s grace and starve offline finalization | One bounded shutdown deadline with a reserved finalization slice and forceful child cleanup | 33.3, 33.6, 45.5 |
| RP-DEF-08 | Cleared numeric provider fields may serialize as strings | Typed config coercion; empty numeric value is omitted/defaulted or rejected visibly, never silently reinterpreted | 33.4, 33.6 |

## Cross-cutting negative-test checklist

- [ ] A signed but unauthorized mention cannot create/resume a session or resolve a credential (28.3, 45.2).
- [ ] Prompt-injected content cannot bypass hard denial, confirmation, sandbox, network destination or credential scope (28.5, 28.6, 45.2).
- [ ] Unknown, duplicate, schema-mutated and malformed MCP tools fail closed without a panic or partial execution (28.5, 28.6).
- [ ] Cancellation while waiting for permission produces no effect and no later grant can revive the call (28.6).
- [ ] Cancellation during file, terminal, network and provider work cleans all owned resources and reports the actual effect boundary (28.5, 28.6, 33.6).
- [ ] Output floods, infinite streams, recursive JSON and daemonized pipe holders remain within configured memory/time/file-descriptor bounds (28.6, 33.2, 33.6).
- [ ] Model, MCP, child and provider output that echoes every injected secret is redacted from logs, state, transcript, activity and UI (28.6, 29.4, 33.2, 33.4, 33.6, 45.2).
- [ ] Permission persistence is private, atomic, scoped, expiring, corruption-safe and contains minimized/redacted context (Goose permission tasks 6–10).
- [ ] Optional model judge error, injection, unknown ID, timeout or cancellation approves no request; a write is never auto-approved (Goose permission tasks 7, 9–10).
- [ ] Provider binary shadowing, same-inode rewrite and pathname swap cannot redirect the secret-bearing deploy after negotiation (33.1, 33.6).
- [ ] Missing/incompatible provider protocol is rejected before any private key or launch secret crosses the boundary (33.1, 33.2, 33.6).
- [ ] Provider non-zero exit, malformed JSON, oversized streams, secret echo, hang and forked child all fail visibly and cleanly (33.2, 33.6).
- [ ] Local and remote instances receive equivalent permission/launch policy; unsupported capabilities fail pre-mutation with no silent fallback (28.6, 33.4, 33.6).
- [ ] Duplicate/replayed events, concurrent jobs and concurrent deploys yield exactly one executor and one terminal effect (28.2, 31.5–31.7, 33.3, 33.5, 33.6).
- [ ] Remote cancellation, SIGTERM, inactivity and intentional shutdown clean children, publish terminal/offline state within budget and are not restarted (33.3, 33.6, 45.5).
- [ ] Presence expiry/disconnect cannot authorize, transfer, terminate or silently restart work (33.5, 33.6).
- [ ] Delegation depth/resource/cycle limits and parent cancellation hold under partial failure and retry (31.3, 31.6, 45.5).
- [ ] Revoked/spoofed/stale mesh capacity never receives work and provider failure never silently falls back (41.1–41.5, 45.2).
- [ ] Public activity, search and observer output contain neither raw encrypted agent state nor private prompt/tool/provider data (29.4, 30.6, 32.1–32.6, 45.2).

## Operational limits handed to Task 4.4

Task 4.4 must assign an owner, metric and alert for every non-provider placeholder below. It may tighten the frozen Buzz provider caps; loosening them requires an explicit security review.

| Limit family | Minimum required dimension |
|---|---|
| ACP/model | prompt bytes/tokens, frame bytes/depth, queued prompts, output bytes/tokens/tool calls, stream idle/total deadline |
| MCP | servers/session, tools/server, schema bytes/depth, request bytes, response bytes, notifications/rate, handshake/request/shutdown deadline |
| Native tools | path/file/patch/match/diff/items limits, terminal command/env/stdin/stdout/stderr limits, network request/response/decompression/redirect limits |
| Local runtime | parallel turns/children/terminals, per-turn duration, idle lifetime, cleanup grace and forced-reap bound |
| Jobs | delegation depth, children, queue age, resource/token budget, lease/recovery/cancellation bounds |
| Provider | config ≤20 flat scalar fields and ≤64 KiB; stdout ≤1 MiB; stderr ≤64 KiB; `info` ≤10s; deploy ≤600s unless a stricter approved value replaces it |
| Remote substrate | CPU/memory/storage/process bounds, startup and shutdown deadlines, presence TTL/staleness and orphan cleanup age |
| Mesh | advertisement/job size, peers, queue/capacity, lease/expiry, partition recovery and fairness |

## Assumptions and residual risks

1. Exact non-provider numeric limits remain Task 4.4 work. This threat model makes an unset bound fail readiness; it does not invent unapproved product SLOs.
2. Once a user deliberately authorizes a provider binary or remote substrate to receive an agent key, a malicious provider/administrator can steal it. Rotation/revocation and explicit provenance reduce recovery cost but cannot erase that trust boundary.
3. Prompt-injection and sensitive-data classifiers are probabilistic. Deterministic authorization, sandboxing, credential scope and output bounds carry the safety claim.
4. Some tool effects are not transactionally reversible. Cancellation guarantees truthful effect reporting and cleanup, not magical rollback after the external effect boundary.
5. Remote presence can be stale within its configured TTL. It remains an availability hint only.
6. ADR-006 must decide mesh trust, eligibility and resources before EX-13 can execute work. Until then shared compute is unavailable, not permissive.
7. Production log shipping for disposable remote generations is an operational prerequisite, but shipped logs must use generation/operation provenance and the same redaction policy.

## Documented planning contradiction

Tasks 41.2 and 41.3 currently declare a direct `_Depends on: 2.5_`, which is ADR-005 push-platform scope, even though both are mesh/shared-compute leaves. This appears to be an extraneous dependency and semantic metadata drift. Gate safety is still preserved transitively: Task 41.2 depends on 41.1, which depends on ADR-006 (Task 2.6) and the mesh threat review (Task 4.8), and Task 41.3 depends on 41.2. Task 4.2 does not silently remove the approved dependency; it records the mismatch for task-plan approval. ADR-005 must not be interpreted as governing shared-compute trust or as a substitute for ADR-006/Task 4.8.

## Requirements traceability

| Acceptance criterion | Controls | Executor boundaries | Implementation/test leaves |
|---|---|---|---|
| 11.1 | INV-AWP-01–INV-AWP-06, INV-AWP-08 | EX-01–EX-10, EX-14 | 28.1–28.6, 29.3–29.4, 31.1–31.7, 32.1–32.6 |
| 11.5 | INV-AWP-05–INV-AWP-07 | EX-09–EX-13 | 31.2, 31.5–31.7, 33.1–33.6, 41.3–41.5, 45.5 |
| 19.1 | Threat register T-AWP-001–T-AWP-025 | EX-01–EX-14 | 28.5–28.6, 29.4, 30.6, 31.3–31.7, 33.1–33.6, 41.1–41.5, 45.2 |
| 19.2 | INV-AWP-02–INV-AWP-05, INV-AWP-07–INV-AWP-08 and the boundary checklist | EX-01–EX-14 | 4.4, 28.5–28.6, 29.3–29.4, 33.1–33.6, 41.1–41.5, 45.2 |

## Review completion criteria

This review remains satisfied only while:

- every new agent or remote executor is added to the EX checklist with all six controls and negative-test ownership;
- no compatibility adapter executes directly or persists a parallel permission/session/result store;
- every retained provider/MCP compatibility boundary has version, hostile-output, cancellation, secret-redaction and cleanup tests;
- every numeric bound is either configured by Task 4.4 or causes the path to remain unavailable;
- Task 45.2 reports passing negative evidence for every T-AWP threat before cutover.
