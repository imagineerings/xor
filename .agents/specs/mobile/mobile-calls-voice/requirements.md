# Requirements: Calls & Voice

## Introduction

The Sim mobile client needs voice and calling capabilities. This includes voice input for the agent (dictation, voice mode) and audio/video calls with collaborators. The iOS app already has `VoiceInputManager`, `VoiceOutputManager`, `ContinuousVoiceManager`, and `EnhancedVoiceManager` — these need to be unified and extended. The `mobile-dev` app has WebRTC-based calls via `react-native-webrtc` and `react-native-incall-manager`. The Sim desktop has calls via the `call` crate with LiveKit integration.

## Glossary

| Term | Definition |
|------|------------|
| **Voice Input** | Speech-to-text for sending messages to the agent. Supports Normal mode (tap to speak), Transcribe mode (streaming transcription), and Continuous mode (always-listening). |
| **Voice Output** | Text-to-speech for hearing agent responses aloud. |
| **Call** | A real-time audio/video connection with one or more participants via LiveKit/WebRTC. |
| **LiveKit** | The open-source WebRTC SFU (Selective Forwarding Unit) used by Sim for multi-party calls. |
| **Room** | A virtual space where call participants connect. Manages participants, screen sharing, and mute state. |
| **DTMF** | Dual-tone multi-frequency signaling for in-call actions. |

## Requirements

### Requirement 1: Voice Input for Agent

**User Story:** As a mobile user, I want to speak to my Sim agent instead of typing, so I can use it hands-free.

1.1 THE app SHALL provide a microphone button in the chat input area.

1.2 WHEN the user taps the microphone button THEN THE app SHALL start speech recognition and display a voice input UI with waveform visualization.

1.3 WHILE recording, the app SHALL display live transcription (partial results updating in real-time).

1.4 WHEN the user taps the stop button OR pauses speaking for 1.5 seconds THEN THE app SHALL finalize the transcription and auto-submit it as a message.

1.5 THE app SHALL support three voice modes:
   - **Normal**: Tap to start, tap to stop
   - **Transcribe**: Continuous dictation with live transcription shown in input field
   - **Continuous**: Always-listening mode; agent proactively responds to queries

1.6 WHEN voice mode is enabled AND the assistant response completes THEN THE app SHALL speak the response via TTS.

1.7 IF the user cancels voice input THEN THE app SHALL discard the transcription and return to text input.

### Requirement 2: Agent-to-Agent Calls (collaboration)

**User Story:** As a mobile user, I want to make audio/video calls with my collaborators, so we can discuss work in real-time.

2.1 WHEN the user opens a contact or channel THEN THE app SHALL provide a "Call" button to start a call.

2.2 WHEN the user taps "Call" THEN THE app SHALL create a room on the collab server and invite the selected participants.

2.3 WHEN the call connects THEN THE app SHALL display a call UI with: participant video tiles, mute button, speaker toggle, end call button.

2.4 THE app SHALL support switching between audio-only and video during a call.

2.5 THE app SHALL display participant presence (speaking indicator, mute status, connection quality).

2.6 WHEN the user receives an incoming call notification (from `CollabWebSocketManager`) THEN THE app SHALL show a incoming call UI with accept/decline options.

### Requirement 3: In-Call Controls

**User Story:** As a mobile user, I want to control my audio/video during a call.

3.1 THE app SHALL support mute/unmute (microphone on/off).

3.2 THE app SHALL support speakerphone toggle (earpiece vs speaker vs Bluetooth).

3.3 THE app SHALL support camera on/off toggle.

3.4 THE app SHALL support switching between front and rear cameras.

### Requirement 4: Screen Sharing

**User Story:** As a mobile user, I want to view a collaborator's shared screen during a call.

4.1 WHEN a participant starts screen sharing THEN THE app SHALL display the shared screen as a large video tile.

4.2 THE app SHALL support viewing the shared screen in full-screen mode.

## Existing Assets

- iOS: `VoiceInputManager.swift`, `VoiceOutputManager.swift`, `ContinuousVoiceManager.swift`, `EnhancedVoiceManager.swift`
- Android: `VoiceManager.kt`, `ContinuousVoiceManager.kt`, `data/model/VoiceState.kt`
- mobile-dev: `app/products/calls/` (full calls product with WebRTC, signaling, connection management)
- Sim desktop: `crates/call/` (LiveKit-based calls, room management, participant handling)
