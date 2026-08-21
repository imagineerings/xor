# Requirements: Observability and Analytics

## Introduction

Migrate goose's observability and analytics infrastructure: Langfuse tracing, OpenTelemetry OTLP export, observation layer, rate limiter, PostHog product analytics, token counting, and tool monitoring/inspection.

## Glossary

- **Langfuse**: Open-source observability platform for LLM applications
- **OpenTelemetry (OTel)**: Open standard for observability (traces, metrics, logs)
- **OTLP**: OpenTelemetry Protocol for exporting telemetry data
- **Observation Layer**: Captures detailed observations about agent operations (turns, tool calls, latency)
- **Rate Limiter**: Prevents excessive requests to APIs
- **PostHog**: Open-source product analytics platform
- **Token Counter**: Tracks token usage across LLM requests
- **Tool Monitor**: Tracks tool usage statistics and patterns
- **Tool Inspector**: Allows inspection of tool implementations and schemas

## Requirements

### Requirement 1: Langfuse Tracing

**User Story:** As a zed developer, I want to trace LLM calls and agent operations in Langfuse, so that I can debug and optimize agent behavior.

#### Acceptance Criteria

1. **1.1** WHERE a Langfuse exporter is approved, THE system SHALL extend Zed's existing tracing/telemetry pipeline rather than create a second agent event stream
2. **1.2** WHEN enabled with explicit consent, approved LLM calls, agent turns, and tool operations SHALL produce correlated traces with a documented redacted attribute schema
3. **1.3** EXPORT SHALL be asynchronous and bounded with sampling, batching, retry/backoff, backpressure/drop accounting, shutdown flush, and failure isolation from agent execution
4. **1.4** ENDPOINT, TLS, credentials, sampling, enablement, and retention SHALL be configurable; secrets, prompt/tool content, paths, user identifiers, and attachments SHALL be excluded unless separately disclosed and approved

### Requirement 2: OpenTelemetry OTLP Export

**User Story:** As a zed operator, I want to export telemetry data via OpenTelemetry OTLP, so that I can integrate with my existing observability infrastructure.

#### Acceptance Criteria

1. **2.1** THE system SHALL support exporting traces via OTLP protocol
2. **2.2** THE OTLP export SHALL be configurable with endpoint and authentication
3. **2.3** WHEN OpenTelemetry is enabled THEN agent operations SHALL produce spans and events

### Requirement 3: Observation Layer

**User Story:** As a zed developer, I want detailed observations about agent operations, so that I can analyze performance, latency, and behavior.

#### Acceptance Criteria

1. **3.1** THE observation layer SHALL capture observations for agent turns
2. **3.2** THE observation layer SHALL capture observations for tool calls (duration, success/failure)
3. **3.3** THE observation layer SHALL capture token usage per operation
4. **3.4** WHEN the agent processes a message THEN the observation layer SHALL record key metrics

### Requirement 4: Rate Limiter

**User Story:** As a zed user, I want the agent to respect rate limits when calling external APIs, so that I don't get throttled or exceed quota.

#### Acceptance Criteria

1. **4.1** THE rate limiter SHALL limit requests based on configurable thresholds
2. **4.2** THE rate limiter SHALL support per-provider rate limits
3. **4.3** WHEN a rate limit is hit THEN the system SHALL queue or delay the request
4. **4.4** THE rate limiter SHALL support burst allowances

### Requirement 5: PostHog Analytics

**User Story:** As a zed developer, I want product analytics via PostHog, so that I can understand how users interact with the agent.

#### Acceptance Criteria

1. **5.1** WHERE product analytics events are approved, THE system SHALL use Zed's existing telemetry abstraction; PostHog is an exporter choice, not a parallel client or mandatory vendor
2. **5.2** EACH approved event SHALL have a stable schema, purpose, consent basis, cardinality bound, and owner and SHALL exclude prompts, tool arguments/results, file paths, secrets, and direct identifiers by default
3. **5.3** ANALYTICS SHALL respect Zed's distribution/channel consent and disablement policy and SHALL configure endpoint/credentials outside user-readable settings
4. **5.4** QUEUING, retry, offline behavior, backpressure, shutdown, deletion/retention, and exporter failure SHALL not block or fail agent operations and SHALL be testable

### Requirement 6: Token Counter

**User Story:** As a zed user, I want to track token usage across LLM requests, so that I can monitor costs and usage.

#### Acceptance Criteria

1. **6.1** THE token counter SHALL track tokens used per request
2. **6.2** THE token counter SHALL aggregate token usage across a session
3. **6.3** THE token counter SHALL support different tokenization schemes per provider
4. **6.4** THE token usage SHALL be available via API and UI

### Requirement 7: Tool Monitoring

**User Story:** As a zed user, I want visibility into tool usage, so that I can see which tools are used most frequently and their performance.

#### Acceptance Criteria

1. **7.1** THE tool monitor SHALL record tool invocations with timestamps and duration
2. **7.2** THE tool monitor SHALL track success and failure rates per tool
3. **7.3** THE tool monitor SHALL provide aggregation of tool usage statistics

### Requirement 8: Tool Inspection

**User Story:** As a zed user, I want to inspect available tools and their schemas, so that I can understand what the agent can do.

#### Acceptance Criteria

1. **8.1** THE tool inspector SHALL enumerate all registered tools
2. **8.2** FOR each tool THE system SHALL show its name, description, and parameter schema
3. **8.3** THE tool inspector SHALL support filtering and searching tools

## References

- Source: `projects/goose/crates/goose/src/tracing/` — mod.rs, langfuse_layer.rs, observation_layer.rs, rate_limiter.rs
- Source: `projects/goose/crates/goose/src/otel/` — mod.rs, otlp.rs
- Source: `projects/goose/crates/goose/src/posthog.rs`
- Source: `projects/goose/crates/goose/src/token_counter.rs`
- Source: `projects/goose/crates/goose/src/tool_monitor.rs`
- Source: `projects/goose/crates/goose/src/tool_inspection.rs`
- Existing zed: `crates/telemetry/`
