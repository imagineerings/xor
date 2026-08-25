# Implementation Plan: Dictation (Speech-to-Text)

> Cross-cutting contract: every production write in this plan inherits the [`agentic` feature boundary](../feature-boundary.md). Completion evidence must classify actual writes and include the required enabled/disabled validation.

## Overview

Implement the dictation system with microphone capture (extending `crates/audio/`), local Whisper inference using `candle`, and pluggable cloud dictation providers. The dictation service is in a new `crates/dictation/` crate.

## Tasks

- [ ] 1. Extend audio crate with microphone capture
  - Microphone device enumeration
  - Audio capture stream with configurable sample rate and format
  - Platform-specific audio capture backends

  - _Requirements: 1.1_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/dictation/requirements.md, .agents/specs/goose-migration/dictation/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/audio/src/capture.rs, crates/audio/src/audio.rs_
  - _Writes: crates/audio/src/capture.rs, crates/audio/src/audio.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 2. Create dictation crate with provider trait
  - Define DictationProvider trait
  - Define AudioFormat enum, DictationConfig

  - _Requirements: 3.1, 3.2, 3.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/dictation/requirements.md, .agents/specs/goose-migration/dictation/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/dictation/Cargo.toml, crates/dictation/src/dictation.rs, Cargo.toml, Cargo.lock_
  - _Writes: crates/dictation/Cargo.toml, crates/dictation/src/dictation.rs, Cargo.toml, Cargo.lock_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 3. Implement Whisper local provider
  - Integrate with candle-core and candle-nn for Whisper inference
  - Model download management (tiny, base, small, medium, large)
  - Audio preprocessing (resampling, normalization)

  - _Requirements: 1.1, 1.2, 1.3, 1.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/dictation/requirements.md, .agents/specs/goose-migration/dictation/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/dictation/src/whisper.rs, crates/dictation/src/dictation.rs_
  - _Writes: crates/dictation/src/whisper.rs, crates/dictation/src/dictation.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 4. Implement cloud dictation providers
  - Generic HTTP-based cloud dictation provider
  - Support for common cloud STT APIs

  - _Requirements: 2.1, 2.2, 2.3, 2.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/dictation/requirements.md, .agents/specs/goose-migration/dictation/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/dictation/src/cloud_providers.rs, crates/dictation/src/dictation.rs, crates/dictation/Cargo.toml_
  - _Writes: crates/dictation/src/cloud_providers.rs, crates/dictation/src/dictation.rs, crates/dictation/Cargo.toml_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 5. Implement dictation service
  - Orchestrates microphone capture → provider → text pipeline
  - Provider selection and fallback
  - Auto-stop on silence detection

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/dictation/requirements.md, .agents/specs/goose-migration/dictation/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/dictation/src/service.rs_
  - _Writes: crates/dictation/src/service.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 6. Integrate dictation into agent and UI
  - Agent tool for dictation (text input via voice)
  - CLI command for dictation
  - Desktop UI toggle/microphone button

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/dictation/requirements.md, .agents/specs/goose-migration/dictation/design.md, .agents/specs/goose-migration/coverage-catalog.md, crates/agent/src/tools/dictation_tool.rs, crates/cli/src/commands/dictation.rs_
  - _Writes: crates/agent/src/tools/dictation_tool.rs, crates/cli/src/commands/dictation.rs_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

- [ ] 7. Write tests
  - Audio format conversion tests
  - Whisper inference tests with test fixtures
  - Mock cloud provider tests

  - _Requirements: 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2, 3.3_
  - _Depends on: none_
  - _Reads: .agents/specs/goose-migration/dictation/requirements.md, .agents/specs/goose-migration/dictation/design.md, .agents/specs/goose-migration/coverage-catalog.md_
  - _Writes: none_
  - _Validation: Run focused behavior and failure-path tests, then ./script/clippy for affected Rust crates_

## Notes

- Whisper models are downloaded on first use, cached locally
- Cloud providers require API key configuration
- Dictation is optional — compiled behind a Cargo feature flag
