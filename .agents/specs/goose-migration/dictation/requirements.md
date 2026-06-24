# Requirements: Dictation (Speech-to-Text)

## Introduction

Migrate goose's dictation system, which provides speech-to-text capabilities using both local Whisper models and cloud-based dictation providers. This allows users to interact with the agent using voice input.

## Glossary

- **Dictation**: Speech-to-text conversion
- **Whisper**: OpenAI's open-source speech recognition model (can run locally)
- **Cloud Dictation Provider**: A cloud-based speech-to-text API service

## Requirements

### Requirement 1: Local Whisper Dictation

**User Story:** As a baymax user, I want to use local Whisper models for speech-to-text, so that I can dictate to the agent without sending audio to external services.

#### Acceptance Criteria

1. WHEN dictation is activated THE system SHALL capture audio from the microphone
2. THE system SHALL process audio through a local Whisper model
3. WHEN speech is transcribed THE system SHALL return the recognized text
4. THE local dictation SHALL work offline without internet connectivity

### Requirement 2: Cloud Dictation Providers

**User Story:** As a baymax user, I want to use cloud-based dictation providers, so that I can get higher accuracy speech recognition.

#### Acceptance Criteria

1. THE system SHALL support pluggable cloud dictation providers
2. WHEN a cloud provider is configured THE system SHALL send audio to that provider's API
3. WHEN the transcription is returned THE system SHALL provide the result to the agent
4. IF the cloud provider is unavailable THEN the system SHALL return a clear error

### Requirement 3: Dictation Provider Abstraction

**User Story:** As a baymax developer, I want a common interface for dictation providers, so that new providers can be added without changing the core dictation system.

#### Acceptance Criteria

1. THE dictation system SHALL define a common trait for all dictation providers
2. THE system SHALL support configuring which provider to use
3. THE system SHALL handle provider errors gracefully

## References

- Source: `goose/crates/goose/src/dictation/` — mod.rs, providers.rs, whisper.rs, whisper_data/
