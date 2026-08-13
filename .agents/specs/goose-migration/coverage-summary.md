# Goose-to-Sim Migration Coverage Summary

## Overall coverage

- **Catalog accounting:** 152 of 152 audited capabilities have one exclusive classification, source evidence, ownership, a remaining gap, verification, confidence, and open questions (100%).
- **Migration-plan coverage estimate:** **87.2%** across in-scope `C1`-`C5` capabilities. Formula: `(C1 + C2 + C3 + 0.5 × C4) / (C1 + C2 + C3 + C4 + C5) = (22 + 35 + 51 + 17.5) / 144`.
- **Fully reusable, extendable, or fully specified:** 108 of 144 in-scope capabilities (75.0%).
- **Estimated implementation coverage:** **27.4%**, using full credit for `C1` and half credit for `C2`; `C3`/`C4`/`C5` receive no implementation credit. This is intentionally conservative because this audit did not implement product code.
- **Explicitly excluded/internal:** eight capabilities (`C6=3`, `C7=5`).

| Classification | Count | Meaning |
| --- | ---: | --- |
| C1 | 22 | Reusable in Sim without changes |
| C2 | 35 | Existing Sim behavior should be extended |
| C3 | 51 | Fully covered by a migration specification, not evidenced as implemented |
| C4 | 35 | Partially covered by a migration specification |
| C5 | 1 | Missing from migration specifications |
| C6 | 3 | Intentionally excluded with rationale |
| C7 | 5 | Upstream/internal infrastructure with no direct port |

## Coverage by domain

| Domain prefix | Capabilities | C1 | C2 | C3 | C4 | C5 | C6 | C7 |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| AGT — agent/extensions/context | 20 | 6 | 6 | 6 | 1 | 0 | 0 | 1 |
| PRV — providers/local inference | 20 | 6 | 3 | 6 | 5 | 0 | 0 | 0 |
| ACP — ACP transports/methods/SDKs | 15 | 2 | 1 | 3 | 6 | 1 | 1 | 1 |
| CFG — configuration/feature gates/TLS | 5 | 2 | 3 | 0 | 0 | 0 | 0 | 0 |
| AUT — authentication/OIDC | 3 | 1 | 1 | 1 | 0 | 0 | 0 | 0 |
| SEC — security/permissions | 7 | 0 | 2 | 1 | 3 | 0 | 0 | 1 |
| RCP — recipes/distribution | 11 | 0 | 0 | 5 | 6 | 0 | 0 | 0 |
| MCP — bundled MCP tools | 7 | 0 | 0 | 6 | 1 | 0 | 0 | 0 |
| SES — sessions/import/export/share | 10 | 1 | 5 | 2 | 2 | 0 | 0 | 0 |
| SCH — scheduling | 3 | 0 | 0 | 3 | 0 | 0 | 0 | 0 |
| GTW — gateway | 3 | 0 | 0 | 3 | 0 | 0 | 0 | 0 |
| DCT — dictation | 3 | 0 | 0 | 1 | 2 | 0 | 0 | 0 |
| CLI — headless CLI | 8 | 0 | 0 | 4 | 4 | 0 | 0 | 0 |
| TUI — terminal UI | 2 | 0 | 0 | 1 | 0 | 0 | 1 | 0 |
| DUI — desktop UI | 17 | 4 | 9 | 3 | 1 | 0 | 0 | 0 |
| OBS — observability/analytics | 6 | 0 | 4 | 1 | 1 | 0 | 0 | 0 |
| EVL — evaluation | 3 | 0 | 1 | 1 | 1 | 0 | 0 | 0 |
| DOC — documentation | 2 | 0 | 0 | 2 | 0 | 0 | 0 | 0 |
| DX — developer workflows | 2 | 0 | 0 | 0 | 2 | 0 | 0 | 0 |
| SVC — external/developer services | 2 | 0 | 0 | 2 | 0 | 0 | 0 | 0 |
| REL — release/governance | 3 | 0 | 0 | 0 | 0 | 0 | 1 | 2 |

## Missing capability

- **ACP-013 — multi-language SDK:** Goose publishes Rust/UniFFI-backed Python and Kotlin artifacts, Maven packages, and Python wheels. No Sim public-binding commitment or migration requirement exists. A product decision is required before specifying API/ABI stability, generated-code ownership, distribution, and support.

## Partially covered or ambiguous capabilities

- **Agent:** AGT-011 plugin discovery/install/update/conversion still lacks complete trust, precedence, pinning, and uninstall criteria.
- **Providers:** PRV-011 consumer services; PRV-015 custom provider discovery/reload/secrets; PRV-016 full llama.cpp/MLX multimodal/tool/cache parity; PRV-018 exact provider auth matrix; PRV-019 labeled usage/cost estimation and pricing ownership.
- **ACP custom methods:** ACP-004 extension/tool methods; ACP-005 provider/onboarding methods; ACP-006 recipe methods; ACP-007 schedule methods; ACP-008 auxiliary dictation/apps/prompts/diagnostics/source methods. The domain behavior is planned, but exact versioned schemas, access matrix, ordering, and transactional errors are incomplete.
- **Security:** SEC-002 egress/classifier privacy, chunking, timeout, and fail-open/closed policy; SEC-005 extension malware heuristics, signatures, false positives, and CLI validation.
- **Recipes:** RCP-002 typed parameters/secrets; RCP-004 source/auth/cache/offline behavior; RCP-005 session/profile precedence and cleanup; RCP-006 structured output integration detail; RCP-008 full desktop create/edit/delete/import UX; RCP-011 deeplink trust/size/signature/version behavior.
- **MCP:** MCP-003 exact macOS/Windows/Linux computer-controller capability and permission matrix.
- **Sessions:** SES-007 explicit legacy import/backup/idempotency/rollback; SES-008 extension/profile restore precedence and partial failure.
- **Dictation:** DCT-001 device/permission/interruption/sample/platform capture behavior; DCT-003 exact cloud-provider contracts, limits, privacy, and retention.
- **CLI:** CLI-001 headless session option precedence/lifecycle; CLI-003 session management selection and deletion safety; CLI-004 provider/config/Doctor TTY and mutation semantics; CLI-008 final command inclusion/exclusion matrix.
- **Desktop:** DUI-010 prompt-management scope and live-session semantics (the existing Sim skills settings surface is reusable); DUI-012 display modes and remaining native-Goose-app versus MCP-App product scope.
- **Developer Context and Commands:** the 33-capability source-backed subcatalog in `developer-experience/coverage-audit.md` found seven confirmed native extensions (local commands, MCP prompt arguments, and project-rule diagnostics), six decision-gated behaviors, five cross-spec behaviors, two intentional exclusions, and two Goose internals with no direct port. The new `goal-grind-commands` pack owns the previously gated persistent-goal/bounded-grind capability.
- **Observability/evaluation/developer workflows:** OBS-006 tool inspection/repetition retention/privacy; EVL-003 datasets/metrics/hardware variance/result schema; DX-001 exact runnable examples; DX-002 script-by-script reuse/exclusion and CI matrix.

## Reuse opportunities and duplicate prevention

| Behavior | Canonical Sim owner to reuse |
| --- | --- |
| Agent turns, compaction, tools, subagents | `crates/agent` thread/session/tool paths |
| ACP agents and extension processes | `crates/agent_servers`, `crates/context_server`, `crates/acp_thread` |
| Provider registry and normalized model contracts | `crates/language_models`, `crates/language_model_core` |
| OAuth callbacks and credentials | `crates/oauth_callback_server`, `crates/credentials_provider` |
| Settings paths and migration | `crates/settings`, `crates/migrator`, `crates/paths` |
| Permission decisions and UI | `crates/agent/src/tool_permissions.rs`, ACP permission paths, `crates/agent_ui` |
| Session persistence/import/export | `crates/agent` thread database/store and existing thread importer |
| Recipe schedules | one recipe/session service; `crates/scheduler` supplies executor primitives only |
| MCP Apps and AutoVisualiser | one conditional context-server/agent-UI renderer with one security policy |
| Diagnostics/Doctor | existing diagnostics collection/UI plus provider registry health checks |
| Downloads/local models | `crates/http_client` plus the selected existing model/cache owner |
| Telemetry/Langfuse/OTLP/analytics | existing tracing/telemetry pipeline with conditional exporters |
| Documentation | existing mdBook, preprocessors, link checks, and deploy workflows |
| Release/build/community workflows | Sim-native CI/release/governance; no direct Goose workflow copy |

## Suspected specification overclaims found and corrected

1. Runtime agent snapshots were actually Insta prompt golden files; runtime capture/restore was removed.
2. The supposed Goose REST/OpenAPI server and routes do not exist; the pack now covers ACP stdio and authenticated HTTP/WebSocket and excludes REST parity.
3. Local inference was incorrectly described as Candle; the plan now reuses Sim llama.cpp and decision-gates MLX.
4. Peekaboo was incorrectly treated as cross-platform; the current source is macOS-gated.
5. A nonexistent generic embedding provider source was cited; the work was replaced by source-backed canonical metadata and usage normalization.
6. OIDC proxy behavior was incorrectly described as an end-user/Anthropic login service; it is a GitHub Actions JWT-verifying upstream-key proxy.
7. Multiple designs assumed new Doctor, download, PostHog, security, permission, i18n, app, runner, and other crates from directory names; affected packs now choose existing integration points first.
8. Tasks used past-tense “implemented/created/tests passing” language without code evidence; those claims were converted to unchecked implementation instructions.
9. Documentation proposed a second Docusaurus site although Sim already has mdBook, preprocessors, search/deployment, link checks, and release channels; the pack now extends existing docs.
10. Ask AI requirements had no task or operational boundary; the pack now decision-gates the external Discord service and records auth/privacy/abuse/freshness/ownership needs.
11. Developer experience assumed `/help` and literal `/recipe` were Goose agent built-ins, proposed direct-agent unknown-command rejection, described `sources.rs` as a root registry, erased action history on clear, and overlooked Sim's existing shared session initialization and skills settings UI. The repaired pack corrects those claims and adds the confirmed multi-argument MCP prompt gap.

## Decisions requiring review

1. Expose Sim as a standalone ACP server: stdio, authenticated HTTP/WebSocket, both, or neither.
2. Keep REST/OpenAPI excluded, or commission a separate non-parity product specification.
3. Approve a terminal-native UI/headless agent CLI and choose the exact command matrix.
4. Select subscription-backed ACP agents, declarative provider presets, and consumer providers Sim will support.
5. Select local-inference scope: existing llama.cpp only, MLX too, supported model families, hardware, and platforms.
6. Approve or exclude persistent memory, Nostr sharing, Telegram gateway, arbitrary container execution, and embedded MCP Apps after privacy/security review.
7. Approve or reject model-based read-only permission judgment and automatic Doctor provider/model changes.
8. Decide public SDK commitments: TypeScript and/or Rust/Python/Kotlin, including support/versioning ownership.
9. Decide whether localization is a repository-wide initiative and whether Goose locale content informs it.
10. Decide whether Sim will operate the GitHub Actions OIDC proxy and Ask AI Discord service, with named operations/security owners.
11. Decide whether Nostr/session links, recipe deeplinks, and shared sessions require signing, encryption, retention, or service infrastructure beyond local import/export.
12. Decide whether blog/community content and additional machine-consumable documentation artifacts belong in this repository.
13. Decide whether nested access-triggered instructions, instruction imports, structured source CRUD APIs, agent/check catalogs, or embedded MCP Apps are Sim products; persistent `/goal` and bounded `/grind` are now approved and owned by `goal-grind-commands`.

## Files and validation

- Authoritative audit artifacts: `coverage-catalog.md`, this summary, `master-migration-plan.md`, and the source-backed domain subcatalog `developer-experience/coverage-audit.md`.
- Updated all 17 migration feature packs (`requirements.md`, `design.md`, and `tasks.md`) under this directory, preserving existing requirement/task IDs where possible and adding stable acceptance-criterion IDs and traceability.
- All task checkboxes remain unchecked.
- Repository spec validator: all 17 feature packs pass. Non-fatal warnings identify repeated write ownership that implementation sequencing must serialize; no unknown, untraced, or uncovered acceptance criteria remain.
