# Implementation Plan: Observability and Analytics

## Overview

Extend zed's observability infrastructure with Langfuse tracing, OpenTelemetry OTLP export, an observation layer, rate limiter, PostHog analytics, token counter, and tool monitoring.

## Repo Reconciliation

- `crates/language_model_core/src/rate_limiter.rs` already provides a semaphore-based request limiter used by several providers.
- `crates/agent/src/thread.rs` already tracks provider-reported token usage and emits token usage updates; this is not the same as a model-aware preflight token counter.

## Tasks

- [ ] 1. Implement token counter
  - Create TokenCounter trait with model-aware counting
  - Implement TikTokenCounter using `tiktoken-rs`
  - Fallback character-based counter for unknown models

  - _Requirements: 6.1, 6.2, 6.3, 6.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/observability-analytics/requirements.md, .agents/specs/goose-migration/observability-analytics/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_model_core/src/token_counter.rs_
  - _Writes: crates/language_model_core/src/token_counter.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 2. Implement Langfuse tracing backend
  - Extend `crates/telemetry/` with Langfuse backend
  - Export existing correlated spans for approved LLM calls, tool calls, and agent turns
  - Define consent, redacted attribute allowlist, endpoint/TLS/credentials, sampling, queue bounds, retries, backpressure, drop accounting, and shutdown flush

  - _Requirements: 1.1, 1.2, 1.3, 1.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/observability-analytics/requirements.md, .agents/specs/goose-migration/observability-analytics/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/telemetry/src/langfuse.rs_
  - _Writes: crates/telemetry/src/langfuse.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 3. Implement OpenTelemetry OTLP backend
  - Extend `crates/telemetry/` with OTel backend
  - Export traces via OTLP protocol
  - Configurable endpoint and auth

  - _Requirements: 2.1, 2.2, 2.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/observability-analytics/requirements.md, .agents/specs/goose-migration/observability-analytics/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/telemetry/src/otel.rs_
  - _Writes: crates/telemetry/src/otel.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 4. Implement observation layer
  - Record agent turns, tool calls, LLM requests with timing
  - Configurable max observations with circular buffer
  - Export observations to registered telemetry backends

  - _Requirements: 3.1, 3.2, 3.3, 3.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/observability-analytics/requirements.md, .agents/specs/goose-migration/observability-analytics/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/telemetry/src/observation.rs_
  - _Writes: crates/telemetry/src/observation.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 5. Extend existing provider rate limiter
  - Audit current semaphore-based `language_model_core::RateLimiter`
  - Add token-bucket or sliding-window behavior only if Goose requires rate-over-time enforcement
  - Add per-provider configuration for limits
  - Preserve existing queue/delay behavior used by providers

  - _Requirements: 4.1, 4.2, 4.3, 4.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/observability-analytics/requirements.md, .agents/specs/goose-migration/observability-analytics/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/language_model_core/src/rate_limiter.rs, crates/language_models/src/provider/_
  - _Writes: crates/language_model_core/src/rate_limiter.rs, crates/language_models/src/provider/_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 6. Resolve analytics event and exporter policy
  - Reuse Zed's telemetry abstraction and add PostHog only as an approved exporter
  - Define each event's purpose, consent basis, stable schema, cardinality, owner, retention, and deletion behavior
  - Exclude prompts, tool data, paths, secrets, attachments, and direct identifiers by default
  - Bound queues/retries and isolate offline, backpressure, shutdown, and exporter failures from the agent

  - _Requirements: 5.1, 5.2, 5.3, 5.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/observability-analytics/requirements.md, .agents/specs/goose-migration/observability-analytics/design.md, .agents/specs/goose-migration/coverage-catalog.md, projects/goose/crates/goose/src/posthog.rs, crates/telemetry/_
  - _Writes: crates/telemetry/_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 7. Implement tool monitor and inspector
  - Record tool invocations with timing and success/failure
  - Aggregate statistics per tool
  - Enumerate registered tools with schemas

  - _Requirements: 7.1, 7.2, 7.3, 8.1, 8.2, 8.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/observability-analytics/requirements.md, .agents/specs/goose-migration/observability-analytics/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/tool_monitor.rs, crates/agent/src/tool_inspector.rs_
  - _Writes: crates/agent/src/tool_monitor.rs, crates/agent/src/tool_inspector.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 8. Integrate observability into agent and providers
  - Hook token counter into provider request path
  - Hook observation layer into agent turn processing
  - Hook rate limiter into provider request path
  - Hook tool monitor into tool execution pipeline

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 5.4, 6.1, 6.2, 6.3, 6.4, 7.1, 7.2, 7.3, 8.1, 8.2, 8.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/observability-analytics/requirements.md, .agents/specs/goose-migration/observability-analytics/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/observability_integration.rs_
  - _Writes: crates/agent/src/observability_integration.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 9. Write tests
  - Token counter accuracy with known texts
  - Rate limiter burst and window behavior
  - Observation layer capture and export
  - PostHog event formatting
  - Tool monitor stats accumulation

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 3.1, 3.2, 3.3, 3.4, 4.1, 4.2, 4.3, 4.4, 5.1, 5.2, 5.3, 5.4, 6.1, 6.2, 6.3, 6.4, 7.1, 7.2, 7.3, 8.1, 8.2, 8.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/observability-analytics/requirements.md, .agents/specs/goose-migration/observability-analytics/design.md, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: none_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

## Notes

- All observability is optional — compiled behind Cargo feature flags
- Rate limiter works at the provider request level, not the HTTP level
- Tool monitoring data is available via API for dashboard integration
