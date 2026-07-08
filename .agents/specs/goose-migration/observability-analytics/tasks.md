# Implementation Plan: Observability and Analytics

## Overview

Extend sim's observability infrastructure with Langfuse tracing, OpenTelemetry OTLP export, an observation layer, rate limiter, PostHog analytics, token counter, and tool monitoring.

## Repo Reconciliation

- `crates/language_model_core/src/rate_limiter.rs` already provides a semaphore-based request limiter used by several providers.
- `crates/agent/src/thread.rs` already tracks provider-reported token usage and emits token usage updates; this is not the same as a model-aware preflight token counter.

## Tasks

- [x] 1. Implement token counter
  - Create TokenCounter trait with model-aware counting
  - Implement TikTokenCounter using `tiktoken-rs`
  - Fallback character-based counter for unknown models
  - _Requirements: 6_
  - _writes: crates/language_model_core/src/token_counter.rs_

- [x] 2. Implement Langfuse tracing backend
  - Extend `crates/telemetry/` with Langfuse backend
  - Create spans for LLM calls, tool calls, agent turns
  - Configurable via settings (endpoint, API keys, enable/disable)
  - _Requirements: 1_
  - _writes: crates/telemetry/src/langfuse.rs_

- [x] 3. Implement OpenTelemetry OTLP backend
  - Extend `crates/telemetry/` with OTel backend
  - Export traces via OTLP protocol
  - Configurable endpoint and auth
  - _Requirements: 2_
  - _writes: crates/telemetry/src/otel.rs_

- [x] 4. Implement observation layer
  - Record agent turns, tool calls, LLM requests with timing
  - Configurable max observations with circular buffer
  - Export observations to registered telemetry backends
  - _Requirements: 3_
  - _writes: crates/telemetry/src/observation.rs_

- [x] 5. Extend existing provider rate limiter
  - Audit current semaphore-based `language_model_core::RateLimiter`
  - Add token-bucket or sliding-window behavior only if Goose requires rate-over-time enforcement
  - Add per-provider configuration for limits
  - Preserve existing queue/delay behavior used by providers
  - _Requirements: 4_
  - _writes: crates/language_model_core/src/rate_limiter.rs, crates/language_models/src/provider/_

- [x] 6. Implement PostHog analytics
  - Create `crates/posthog/` with PostHog client
  - Event capture for key user actions
  - Configurable (disable, API key, host)
  - PII-free event properties
  - _Requirements: 5_
  - _writes: crates/posthog/src/posthog.rs, crates/posthog/src/client.rs_

- [x] 7. Implement tool monitor and inspector
  - Record tool invocations with timing and success/failure
  - Aggregate statistics per tool
  - Enumerate registered tools with schemas
  - _Requirements: 7, 8_
  - _writes: crates/agent/src/tool_monitor.rs, crates/agent/src/tool_inspector.rs_

- [ ] 8. Integrate observability into agent and providers
  - Hook token counter into provider request path
  - Hook observation layer into agent turn processing
  - Hook rate limiter into provider request path
  - Hook tool monitor into tool execution pipeline
  - _Requirements: 1-8_
  - _writes: crates/agent/src/observability_integration.rs_

- [ ] 9. Write tests
  - Token counter accuracy with known texts
  - Rate limiter burst and window behavior
  - Observation layer capture and export
  - PostHog event formatting
  - Tool monitor stats accumulation
  - _Requirements: 1-8_

## Notes

- All observability is optional — compiled behind Cargo feature flags
- Rate limiter works at the provider request level, not the HTTP level
- Tool monitoring data is available via API for dashboard integration
