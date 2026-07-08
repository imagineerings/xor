# Implementation Plan: Dictation (Speech-to-Text)

## Overview

Implement the dictation system with microphone capture (extending `crates/audio/`), local Whisper inference using `candle`, and pluggable cloud dictation providers. The dictation service is in a new `crates/dictation/` crate.

## Tasks

- [x] 1. Extend audio crate with microphone capture
  - Microphone device enumeration
  - Audio capture stream with configurable sample rate and format
  - Platform-specific audio capture backends
  - _Requirements: 1.1_
  - _writes: crates/audio/src/capture.rs, crates/audio/src/audio.rs_

- [x] 2. Create dictation crate with provider trait
  - Define DictationProvider trait
  - Define AudioFormat enum, DictationConfig
  - _Requirements: 3_
  - _writes: crates/dictation/Cargo.toml, crates/dictation/src/dictation.rs, Cargo.toml, Cargo.lock_

- [x] 3. Implement Whisper local provider
  - Integrate with candle-core and candle-nn for Whisper inference
  - Model download management (tiny, base, small, medium, large)
  - Audio preprocessing (resampling, normalization)
  - _Requirements: 1_
  - _writes: crates/dictation/src/whisper.rs, crates/dictation/src/dictation.rs_

- [x] 4. Implement cloud dictation providers
  - Generic HTTP-based cloud dictation provider
  - Support for common cloud STT APIs
  - _Requirements: 2_
  - _writes: crates/dictation/src/cloud_providers.rs, crates/dictation/src/dictation.rs, crates/dictation/Cargo.toml_

- [x] 5. Implement dictation service
  - Orchestrates microphone capture → provider → text pipeline
  - Provider selection and fallback
  - Auto-stop on silence detection
  - _Requirements: 1, 2, 3_
  - _writes: crates/dictation/src/service.rs_

- [x] 6. Integrate dictation into agent and UI
  - Agent tool for dictation (text input via voice)
  - CLI command for dictation
  - Desktop UI toggle/microphone button
  - _Requirements: 1, 2_
  - _writes: crates/agent/src/tools/dictation_tool.rs, crates/cli/src/commands/dictation.rs_

- [x] 7. Write tests
  - Audio format conversion tests
  - Whisper inference tests with test fixtures
  - Mock cloud provider tests
  - _Requirements: 1-3_

## Notes

- Whisper models are downloaded on first use, cached locally
- Cloud providers require API key configuration
- Dictation is optional — compiled behind a Cargo feature flag
