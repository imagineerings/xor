# Requirements: Miscellaneous Services

## Introduction

Migrate the remaining goose services, scripts, and examples: the Ask AI bot service, development/CI scripts, Nostr session sharing, session import formats, examples, and the provider error proxy.

## Glossary

- **Ask AI Bot**: A service bot for answering questions about goose
- **Nostr**: Open protocol for decentralized social networking — used for sharing sessions
- **Provider Error Proxy**: A proxy server for intercepting and debugging provider API errors
- **Session Import Format**: Format for importing sessions from other tools or formats

## Requirements

### Requirement 1: Ask AI Bot Service

**User Story:** As a sim user, I want a Q&A bot that can answer questions about using the agent, so that I can get help without reading all documentation.

#### Acceptance Criteria

1. **1.1** WHERE a separately operated Ask AI service is approved, IT SHALL answer documentation questions in the approved chat platform with citations or source links and a clear uncertainty/failure response
2. **1.2** THE service SHALL ingest only approved canonical Sim documentation with defined freshness, authentication, privacy, prompt-injection, abuse-control, rate-limit, and retention policies
3. **1.3** THE deployment SHALL have an explicit operational owner for bot credentials, model/provider credentials, observability, incidents, content updates, and container publishing

### Requirement 2: Session Import Formats

**User Story:** As a sim user, I want to import sessions from other formats, so that I can migrate conversations from other tools.

#### Acceptance Criteria

1. **2.1** THE existing thread import pipeline SHALL support the approved Claude Code, Codex, and Pi JSONL transcript formats in addition to Sim-native import
2. **2.2** WHEN a session is imported THEN messages, roles, tool calls/results, timestamps, metadata, and supported attachments SHALL be converted to the native thread format without executing imported content
3. **2.3** IF the format, record version, event kind, role transition, tool pairing, attachment, timestamp, or identifier is unsupported or invalid, THEN the importer SHALL return a source-located error or an explicit documented skip result
4. **2.4** IMPORT SHALL be explicit, validate size/depth/path limits, handle duplicate IDs and partial files deterministically, and leave source data unchanged on failure

### Requirement 3: Nostr Session Sharing

**User Story:** As a sim user, I want to share agent sessions via Nostr, so that I can publish and discover sessions on a decentralized network.

#### Acceptance Criteria

1. **3.1** WHERE Nostr sharing is approved, AN optional adapter SHALL encrypt and publish approved shared-session data to user-selected relays
2. **3.2** THE adapter SHALL authenticate, decrypt, validate, and import Nostr session payloads through the same native thread import boundary
3. **3.3** Nostr support SHALL be feature-gated and opt-in and SHALL disclose key custody, relay metadata leakage, content permanence/deletion limits, abuse reporting, failure behavior, and compatibility

### Requirement 4: Examples

**User Story:** As a sim developer, I want example integrations, so that I can see how to use and extend the agent.

#### Acceptance Criteria

1. **4.1** THE migration SHALL inventory Goose's MCP wiki example and port an equivalent only if the corresponding Sim extension surface is public and approved
2. **4.2** THE migration SHALL inventory plugin examples and add only examples for the approved Sim plugin/extension format
3. **4.3** THE migration SHALL inventory `frontend_tools.py` and map it to an approved Sim tool/UI protocol rather than copying a Goose-specific endpoint
4. **4.4** EACH retained example SHALL declare prerequisites and owner and SHALL run in CI or an explicit reproducible validation job

### Requirement 5: Development and CI Scripts

**User Story:** As a sim developer, I want scripts for common development tasks, so that development workflows are automated.

#### Acceptance Criteria

1. **5.1** THE migration SHALL map Goose's Windows build script to Sim's existing Windows CI/release path and add nothing when behavior is already covered
2. **5.2** THE Goose OpenAPI validation workflow SHALL be excluded because current Goose exposes ACP rather than a REST/OpenAPI product surface; ACP/schema generation is owned by the ACP and SDK specs
3. **5.3** DIAGNOSTICS viewing SHALL extend Sim's existing diagnostics/log tooling with redaction and size limits instead of copying a parallel viewer unless a confirmed workflow gap remains
4. **5.4** DATABASE inspection SHALL extend or document Sim's existing thread/database tooling with read-only defaults, backup, version checks, and explicit destructive confirmation
5. **5.5** MCP test workflows SHALL reuse Sim context-server/agent-server test harnesses
6. **5.6** SUBAGENT, sub-recipe, and compaction workflows SHALL become focused tests or scripts only where existing test infrastructure cannot reproduce the source behavior
7. **5.7** RELEASE automation SHALL remain owned by Sim's release pipeline; only approved migration packaging inputs may be added
8. **5.8** EACH retained developer script SHALL have an owner, safe defaults, prerequisites, deterministic exit status, and CI or documented validation

### Requirement 6: Provider Error Proxy

**User Story:** As a sim developer, I want a proxy for intercepting and debugging provider API errors, so that I can diagnose provider integration issues.

#### Acceptance Criteria

1. **6.1** WHERE a standalone provider error proxy is still needed after reviewing Sim HTTP diagnostics, IT SHALL be an explicit opt-in developer tool and SHALL never be selected in production configuration by default
2. **6.2** THE proxy SHALL redact authorization, cookies, provider keys, user content, binary bodies, and configured sensitive headers/fields while preserving useful status, timing, retry, and streaming diagnostics
3. **6.3** THE proxy SHALL preserve TLS expectations, streaming, cancellation, status, headers, and bodies within configured limits and SHALL fail closed on invalid upstream configuration

## References

- Source: `projects/goose/services/ask-ai-bot/`
- Source: `projects/goose/crates/goose/src/session/import_formats/`
- Source: `projects/goose/crates/goose/src/session/nostr_share.rs`
- Source: `projects/goose/examples/`
- Source: `projects/goose/scripts/`
- Source: `projects/goose/scripts/provider-error-proxy/`
