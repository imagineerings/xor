# Goose → Baymax Migration Plan

## Purpose

Identify all features and functionality present in the `projects/goose/` directory and plan their migration into baymax, avoiding duplication where baymax already has equivalent functionality.

## Methodology

For each goose feature, we assess:
- **Already exists in baymax** — No migration needed; document the correspondence.
- **Partially exists** — Gaps to fill.
- **Does not exist** — Full migration required.

---

## Feature Inventory

### 1. Core Agent Engine (`projects/goose/crates/goose/src/agents/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Agent loop & tool execution | `crates/agent/` | ✅ Already exists |
| MCP client | `crates/context_server/` | ✅ Already exists |
| Extension manager | `crates/extension/` | ✅ Already exists |
| Prompt manager | `crates/prompt_store/` | ✅ Already exists |
| Retry logic | `crates/agent/` (partial) | ⚠️ Partially exists |
| Large response handler | — | ❌ New |
| Final output tool | `crates/agent/src/tools/` | ⚠️ Needs assessment |
| Subagent execution & task config | — | ❌ New |
| Tool confirmation | `crates/agent/src/tool_permissions.rs` | ⚠️ Partially exists |
| Agent snapshots | — | ❌ New |
| Extension malware check | — | ❌ New |
| Validate extensions | — | ❌ New |
| MOIM (multi-agent?) | — | ❌ New |

### 2. LLM Providers (`projects/goose/crates/goose/src/providers/`)

| Goose Provider | Baymax Equivalent | Status |
|---|---|---|
| Anthropic | `crates/anthropic/` | ✅ Already exists |
| OpenAI | `crates/open_ai/` | ✅ Already exists |
| Google/Gemini | `crates/google_ai/` | ✅ Already exists |
| Ollama | `crates/ollama/` | ✅ Already exists |
| OpenRouter | `crates/open_router/` | ✅ Already exists |
| AWS Bedrock | `crates/bedrock/` | ✅ Already exists |
| DeepSeek | `crates/deepseek/` | ✅ Already exists |
| Mistral | `crates/mistral/` | ✅ Already exists |
| xAI | `crates/x_ai/` | ✅ Already exists |
| LM Studio | `crates/lmstudio/` | ✅ Already exists |
| Copilot Chat | `crates/copilot_chat/` | ✅ Already exists |
| OpenCode | `crates/opencode/` | ✅ Already exists |
| OpenAI Compatible | `crates/language_models/src/provider/open_ai_compatible.rs` | ✅ Already exists |
| **Azure** | — | ❌ New |
| **GCP Vertex AI** | — | ❌ New |
| **Claude ACP** | — | ❌ New |
| **Claude Code** | — | ❌ New |
| **ChatGPT/Codex** | — | ❌ New |
| **Cursor Agent** | — | ❌ New |
| **Databricks (v1/v2)** | — | ❌ New |
| **Snowflake** | — | ❌ New |
| **HuggingFace** | — | ❌ New |
| **LiteLLM** | — | ❌ New |
| **NanoGPT** | — | ❌ New |
| **Tetrate** | — | ❌ New |
| **Avian** | — | ❌ New |
| **KimiCode** | — | ❌ New |
| **Sagemaker TGI** | — | ❌ New |
| **Local Inference** | — | ❌ New |
| **Gemini CLI/OAuth** | — | ❌ New |
| **Declarative providers** | — | ❌ New |
| **Embedding providers** | — | ❌ New |
| **Provider registry** | — | ❌ New |

### 3. ACP Protocol (`projects/goose/crates/goose/src/acp/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| ACP server | `crates/acp_thread/` | ✅ Already exists |
| ACP tools | `crates/acp_tools/` | ✅ Already exists |
| ACP transport | `crates/acp_thread/src/connection.rs` | ✅ Already exists |
| ACP templates | — | ⚠️ Partial |
| ACP MCP app proxy | — | ❌ New |
| ACP provider/adapters | — | ⚠️ Partial |
| ACP response builder | — | ⚠️ Partial |

### 4. MCP Tools (`crates/goose-mcp/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| MCP server runner | — | ❌ New |
| **AutoVisualiser** (code viz) | — | ❌ New |
| **Computer Controller** (PDF/DOCX/XLSX/platform) | — | ❌ New |
| **Memory** (long-term memory) | — | ❌ New |
| **Peekaboo** (screen monitoring) | — | ❌ New |
| **Tutorial** | — | ❌ New |

### 5. Configuration (`projects/goose/crates/goose/src/config/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Base config | `crates/settings/` | ✅ Already exists |
| Provider config | `crates/agent_settings/` | ✅ Already exists |
| Extension config | `crates/settings/` | ✅ Already exists |
| Goose mode | — | ❌ New |
| Experiments/feature flags | `crates/feature_flags/` | ✅ Already exists |
| Migrations | — | ❌ New |
| Permission config | — | ❌ New |
| Declarative providers | — | ❌ New (see providers) |

### 6. Security (`projects/goose/crates/goose/src/security/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Adversary inspector | — | ❌ New |
| Egress inspector | — | ❌ New |
| Classification client | — | ❌ New |
| Security scanner | — | ❌ New |
| Pattern detection | — | ❌ New |
| Security inspector (combined) | — | ❌ New |

### 7. Permissions (`projects/goose/crates/goose/src/permission/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Permission confirmation | `crates/agent/src/tool_permissions.rs` | ⚠️ Partial |
| Permission inspector | — | ❌ New |
| Permission judge | — | ❌ New |
| Permission store | — | ❌ New |

### 8. Gateway (`projects/goose/crates/goose/src/gateway/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Gateway handler | — | ❌ New |
| Gateway manager | — | ❌ New |
| Pairing | — | ❌ New |
| **Telegram integration** | — | ❌ New |
| Telegram format | — | ❌ New |

### 9. Dictation (`projects/goose/crates/goose/src/dictation/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Whisper STT | — | ❌ New |
| Cloud dictation providers | — | ❌ New |

### 10. Session (`projects/goose/crates/goose/src/session/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Session manager | `crates/session/` | ✅ Already exists |
| Chat history search | `crates/search/` | ⚠️ Partial |
| Session diagnostics | — | ⚠️ Partial |
| Extension data | — | ❌ New |
| Last message snippet | — | ❌ New |
| Legacy migration | — | ❌ New |
| Nostr sharing | — | ❌ New |
| Import formats | — | ❌ New |

### 11. Skills (`projects/goose/crates/goose/src/skills/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Skill management | `crates/agent_skills/` | ✅ Already exists |
| Built-in skills | — | ⚠️ Partial |
| Skill arguments | — | ⚠️ Partial |
| Skill client | `crates/agent_skills/` | ✅ Already exists |
| Skill discovery | `crates/agent_skills/` | ✅ Already exists |

### 12. Slash Commands (`projects/goose/crates/goose/src/slash_commands/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Slash command system | — | ❌ New |
| Recipe slash command | — | ❌ New |
| Skill slash command | — | ❌ New |
| Types & utilities | — | ❌ New |

### 13. Hints (`projects/goose/crates/goose/src/hints/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Import files hints | — | ❌ New |
| Load hints | — | ❌ New |

### 14. Goose Apps (`projects/goose/crates/goose/src/goose_apps/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| App system (chat, clock, etc.) | — | ❌ New |
| Cache | — | ❌ New |
| Resource | — | ❌ New |

### 15. Execution (`projects/goose/crates/goose/src/execution/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Execution manager | `crates/agent/` | ⚠️ Partial |

### 16. Scheduler (`projects/goose/crates/goose/src/scheduler*.rs`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Scheduler | `crates/scheduler/` | ✅ Already exists |

### 17. Prompts (`projects/goose/crates/goose/src/prompts/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| System prompts | `crates/prompt_store/` | ✅ Already exists |
| Specialized prompts | `crates/prompt_store/` | ✅ Already exists |
| Plan prompt | — | ⚠️ Partial |
| Compaction prompt | — | ❌ New |
| Permission judge prompt | — | ❌ New |

### 18. Tracing/Observability (`projects/goose/crates/goose/src/tracing/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Langfuse layer | — | ❌ New |
| Observation layer | — | ❌ New |
| Rate limiter | — | ❌ New |
| OpenTelemetry (OTLP) | `crates/otel/` | ⚠️ Partial |

### 19. OAuth (`projects/goose/crates/goose/src/oauth/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| OAuth callback server | `crates/oauth_callback_server/` | ✅ Already exists |
| OAuth persistence | — | ❌ New |
| OAuth device flow | — | ❌ New |

### 20. Context Management (`projects/goose/crates/goose/src/context_mgmt/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Context window management | — | ❌ New |

### 21. Plugins (`projects/goose/crates/goose/src/plugins/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Plugin discovery | — | ❌ New |
| Plugin formats | — | ❌ New |

### 22. Hooks (`projects/goose/crates/goose/src/hooks/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Hook system | — | ❌ New |

### 23. Platform Extensions (`projects/goose/crates/goose/src/agents/platform_extensions/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Apps extension | — | ❌ New |
| Code execution | — | ❌ New |
| Orchestrator | — | ❌ New |
| Chatrecall | — | ❌ New |
| Summarize | — | ❌ New |
| Summon | — | ❌ New |
| Todo | — | ❌ New |
| Tom | — | ❌ New |
| Analyze | — | ❌ New |
| Developer | — | ❌ New |
| Extension manager | — | ❌ New |

### 24. Other Core Files (`projects/goose/crates/goose/src/*.rs`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Action required manager | — | ❌ New |
| Built-in extensions | `crates/extension/` | ⚠️ Partial |
| Doctor | — | ❌ New |
| Download manager | — | ❌ New |
| Instance ID | — | ❌ New |
| Logging | `crates/zlog/` | ✅ Already exists |
| MCP utilities | `crates/context_server/` | ✅ Already exists |
| Model abstraction | `crates/language_model_core/` | ✅ Already exists |
| PostHog analytics | — | ❌ New |
| Prompt template | — | ⚠️ Partial |
| Recipe deeplink | — | ❌ New |
| Source roots/sources | — | ❌ New |
| Subprocess | — | ❌ New |
| Token counter | — | ❌ New |
| Tool inspection | — | ❌ New |
| Tool monitoring | — | ❌ New |
| Utilities | `crates/util/` | ✅ Already exists |

### 25. CLI (`crates/goose-cli/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| CLI commands | `crates/cli/` | ✅ Already exists |
| Configure command | — | ⚠️ Partial |
| Doctor command | — | ❌ New |
| Gateway command | — | ❌ New |
| Plugin command | — | ❌ New |
| Project tracker | — | ⚠️ Partial |
| Recipe commands | — | ❌ New |
| Schedule command | — | ⚠️ Partial |
| Session command | — | ⚠️ Partial |
| Skills command | — | ⚠️ Partial |
| Term/TUI commands | — | ❌ New |
| Update command | `crates/auto_update/` | ✅ Already exists |
| Scenario tests | — | ❌ New |

### 26. Server (`crates/goose-server/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| REST API server | — | ❌ New |
| Auth/TLS | `crates/collab/` | ⚠️ Partial |
| OpenAPI | — | ❌ New |
| Session event bus | — | ❌ New |
| Tunnel | — | ❌ New |
| Routes (agent, session, recipe, etc.) | — | ❌ New |

### 27. UI Desktop (`ui/desktop/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Electron desktop app | — | ❌ New |
| React component library | — | ❌ New |
| ACP client | — | ⚠️ Partial |
| Session management UI | — | ❌ New |
| Scheduling UI | — | ❌ New |
| Updates UI | — | ❌ New |
| Recipe UI | — | ❌ New |
| Settings UI | — | ❌ New |
| i18n | — | ❌ New |
| Theme system | `crates/theme/` | ✅ Already exists |

### 28. UI Text / TUI (`ui/text/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Terminal UI application | — | ❌ New |
| Configure flow | — | ❌ New |
| Extensions management | — | ❌ New |
| Markdown rendering | — | ❌ New |
| Onboarding | — | ❌ New |

### 29. TypeScript SDK (`ui/sdk/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Goose TS client | — | ❌ New |
| HTTP streaming | — | ❌ New |
| MCP apps integration | — | ❌ New |
| Client capabilities | — | ❌ New |

### 30. OIDC Proxy (`oidc-proxy/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Cloudflare OIDC worker | — | ❌ New |
| Anthropic OIDC proxy | — | ❌ New |

### 31. Recipe Scanner (`recipe-scanner/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Recipe scanning infrastructure | — | ❌ New |
| Docker-based scanning | — | ❌ New |

### 32. Workflow Recipes (`workflow_recipes/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Pre-built workflow recipes | — | ❌ New |

### 33. Evals (`evals/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Harbor eval framework | — | ❌ New |
| Open Model Gym | — | ❌ New |

### 34. Documentation (`documentation/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Docusaurus docs site | — | ❌ New |
| Blog, docs, tutorials | — | ❌ New |

### 35. Examples (`examples/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| MCP wiki example | — | ⚠️ Partial |
| Plugin examples | — | ⚠️ Partial |
| Frontend tools | — | ❌ New |

### 36. Services (`services/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Ask AI bot service | — | ❌ New |

### 37. Scripts (`scripts/`)

| Goose Feature | Baymax Equivalent | Status |
|---|---|---|
| Benchmark scripts | — | ⚠️ Partial |
| DB helper | — | ❌ New |
| OpenAPI check | — | ❌ New |
| Diagnostics viewer | — | ❌ New |
| Pre-release script | — | ⚠️ Partial |
| MCP/sub-agent/sub-recipe testing | — | ❌ New |
| Windows build | — | ❌ New |
| Compaction testing | — | ❌ New |
| Provider error proxy | — | ❌ New |

---

## Summary

| Category | Count |
|---|---|
| ✅ Already exists in baymax | ~40 features |
| ⚠️ Partially exists in baymax | ~25 features |
| ❌ New (needs migration) | ~80 features |

## Migration Specs

The new features are grouped into logical specs for migration. Each spec follows the EARS requirements → Design → Tasks workflow.
