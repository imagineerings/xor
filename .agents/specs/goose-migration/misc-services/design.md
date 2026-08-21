# Design Document: Miscellaneous Services

## 1. Overview

Migrate the remaining goose components: Ask AI bot service, Nostr session sharing, session import formats, examples, development/CI scripts, and the provider error proxy.

### Key Architectural Decisions

- **Ask AI remains conditional**: Do not create or deploy a service until product, chat-platform, privacy, abuse, and operational ownership are approved.
- **Session import extends the existing thread importer**: Add Claude Code, Codex, and Pi adapters without creating a second session store.
- **Nostr remains conditional**: If approved, make it an optional adapter over shared-session serialization; do not create a crate until the feature boundary requires one.
- **Scripts and examples are reconciled, not copied**: Reuse Zed's `script/`, CI, examples, and test harnesses and add only confirmed workflow gaps.
- **Provider error proxy is developer-only and conditional**: Prefer existing HTTP diagnostics; any proxy has opt-in and redaction-safe defaults.

## 2. Architecture

```mermaid
graph TD
    subgraph "Session Features (crates/session/)"
        ImportFormats[Import Formats]
        LegacySupport[Legacy Session Import]
        NostrShare[Nostr Sharing]
    end

    subgraph "Services"
        AskBot[Ask AI Bot Service]
    end

    subgraph "Dev Tools"
        ErrorProxy[Provider Error Proxy]
        Scripts[CI / Dev Scripts]
        Examples[Example Integrations]
    end

    subgraph "External"
        Nostr[Nostr Relays]
        FileImport[Session Files - JSON/MD/other]
    end

    ImportFormats --> FileImport
    NostrShare --> Nostr
    AskBot --> KnowledgeBase[Documentation KB]
    ErrorProxy -->|intercepts| Providers[LLM Providers]
```

## 3. Components and Interfaces

### Component: Session Import

```rust
pub struct SessionImporter {
    formats: Vec<Box<dyn ImportFormat>>,
}

#[async_trait]
pub trait ImportFormat: Send + Sync {
    fn name(&self) -> &str;
    fn extensions(&self) -> Vec<&str>;
    async fn detect(&self, content: &[u8]) -> bool;
    async fn import(&self, content: &[u8]) -> Result<ImportedSession>;
}

pub struct ImportedSession {
    pub title: String,
    pub messages: Vec<ImportedMessage>,
    pub metadata: Value,
}
```

### Component: Nostr Sharing

```rust
pub struct NostrShare {
    relays: Vec<String>,
    private_key: Option<String>,
}

impl NostrShare {
    pub async fn publish_session(&self, session: &Session) -> Result<String>; // returns event ID
    pub async fn import_session(&self, event_id: &str) -> Result<Session>;
    pub async fn discover_sessions(&self, author: &str) -> Result<Vec<SessionSummary>>;
}
```

### Component: Ask AI Bot

```typescript
// services/ask-ai-bot/
class AskAIBot {
  constructor(config: BotConfig)

  async answerQuestion(question: string): Promise<string> {
    // 1. Search knowledge base
    // 2. Generate answer using LLM
    // 3. Return with citations
  }

  private async searchKnowledgeBase(query: string): Promise<Document[]> {
    // Vector search over documentation
  }
}
```

### Component: Provider Error Proxy

```rust
pub struct ProviderErrorProxy {
    target_url: String,
    listener_port: u16,
    log_file: PathBuf,
}

impl ProviderErrorProxy {
    pub async fn start(&self) -> Result<()>;
    pub async fn stop(&self);

    // Intercepts requests, logs them, forwards to target
    async fn handle_request(&self, req: Request) -> Response {
        log_request(&req);
        let response = forward_to_provider(req).await;
        log_response(&response);
        response
    }
}
```

## 4. Data Models

```rust
pub struct ImportedMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    pub timestamp: Option<DateTime<Utc>>,
}

pub struct BotConfig {
    pub llm_provider: String,
    pub llm_model: String,
    pub knowledge_base_paths: Vec<PathBuf>,
    pub embedding_provider: Option<String>,
}
```

## 5. Correctness Properties

### Property 1: Import Fidelity

_For any_ session imported [from a supported format], [after import], THE system SHALL preserve message content, role, and ordering.

**Validates: Requirement 2.2**

### Property 2: Nostr Privacy

_For any_ session shared via Nostr, [after sharing], THE session SHALL NOT be shared automatically — only when explicitly requested.

**Validates: Requirement 3.3**

## 6. Error Handling

| Error Scenario | Handling |
|---|---|
| Unknown import format | Return list of supported formats |
| Nostr relay unavailable | Try next relay, fail after all exhausted |
| Ask AI bot KB not found | Fall back to generic LLM answer |
| Error proxy fails to bind port | Log error, suggest alternative port |

## 7. Testing Strategy

- **Import tests**: Test each import format with sample files
- **Nostr tests**: With mock relay server
- **Error proxy tests**: With mock provider endpoint

## References

- Source: `projects/goose/services/ask-ai-bot/`
- Source: `projects/goose/crates/goose/src/session/import_formats/`
- Source: `projects/goose/crates/goose/src/session/nostr_share.rs`
- Source: `projects/goose/examples/`
- Source: `projects/goose/scripts/`

## Requirements traceability

| Requirement | Design element | Verification |
| --- | --- | --- |
| 1.1 | Ask AI Bot Service audit design | Observable scenario and failure-path test for 1.1 |
| 1.2 | Ask AI Bot Service audit design | Observable scenario and failure-path test for 1.2 |
| 1.3 | Ask AI Bot Service audit design | Observable scenario and failure-path test for 1.3 |
| 2.1 | Session Import Formats audit design | Observable scenario and failure-path test for 2.1 |
| 2.2 | Session Import Formats audit design | Observable scenario and failure-path test for 2.2 |
| 2.3 | Session Import Formats audit design | Observable scenario and failure-path test for 2.3 |
| 2.4 | Session Import Formats audit design | Observable scenario and failure-path test for 2.4 |
| 3.1 | Nostr Session Sharing audit design | Observable scenario and failure-path test for 3.1 |
| 3.2 | Nostr Session Sharing audit design | Observable scenario and failure-path test for 3.2 |
| 3.3 | Nostr Session Sharing audit design | Observable scenario and failure-path test for 3.3 |
| 4.1 | Examples audit design | Observable scenario and failure-path test for 4.1 |
| 4.2 | Examples audit design | Observable scenario and failure-path test for 4.2 |
| 4.3 | Examples audit design | Observable scenario and failure-path test for 4.3 |
| 4.4 | Examples audit design | Observable scenario and failure-path test for 4.4 |
| 5.1 | Development and CI Scripts audit design | Observable scenario and failure-path test for 5.1 |
| 5.2 | Development and CI Scripts audit design | Observable scenario and failure-path test for 5.2 |
| 5.3 | Development and CI Scripts audit design | Observable scenario and failure-path test for 5.3 |
| 5.4 | Development and CI Scripts audit design | Observable scenario and failure-path test for 5.4 |
| 5.5 | Development and CI Scripts audit design | Observable scenario and failure-path test for 5.5 |
| 5.6 | Development and CI Scripts audit design | Observable scenario and failure-path test for 5.6 |
| 5.7 | Development and CI Scripts audit design | Observable scenario and failure-path test for 5.7 |
| 5.8 | Development and CI Scripts audit design | Observable scenario and failure-path test for 5.8 |
| 6.1 | Provider Error Proxy audit design | Observable scenario and failure-path test for 6.1 |
| 6.2 | Provider Error Proxy audit design | Observable scenario and failure-path test for 6.2 |
| 6.3 | Provider Error Proxy audit design | Observable scenario and failure-path test for 6.3 |
