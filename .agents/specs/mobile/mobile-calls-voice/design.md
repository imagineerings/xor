# Design: Calls & Voice

## 1. Overview

Voice features are split into two domains: (1) voice as an input/output modality for the AI agent (speech-to-text, text-to-speech), and (2) real-time audio/video calls with collaborators via WebRTC/LiveKit. The iOS app already has extensive voice managers; this design formalizes the API and extends it with call capabilities.

### Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Speech-to-text | Platform-native (iOS: SFSpeechRecognizer, Android: SpeechRecognizer) | No external dependency, offline capable |
| Text-to-speech | Platform-native (iOS: AVSpeechSynthesizer, Android: TextToSpeech) | Low latency, system voice quality |
| WebRTC calls | LiveKit SDK (matching desktop) | Reuse existing `call` crate infrastructure |
| Voice modes | Normal, Transcribe, Continuous | Progressive complexity; Normal is default |

## 2. Architecture

```mermaid
graph TB
    subgraph "Voice Input"
        VIM[VoiceInputManager]
        SR[Speech Recognizer]
        MV[Microphone View<br/>waveform + transcription]
    end

    subgraph "Voice Output"
        VOM[VoiceOutputManager]
        TTS[Text-to-Speech]
    end

    subgraph "Calls"
        CM[CallManager]
        LK[LiveKit Client]
        CU[Call UI<br/>video tiles + controls]
    end

    subgraph "Shared"
        VM[ChatViewModel]
        API[AgentAPIService]
        WS[CollabWebSocketManager]
    end

    VIM --> SR
    VIM --> VM: transcription → send message
    VOM --> TTS
    VOM <--> VM: speak response
    CM --> LK
    CM --> WS: signaling
    CU --> CM
```

## 3. Tasks

- [ ] 1. VoiceInputManager with Normal/Transcribe/Continuous modes
- [ ] 2. VoiceOutputManager with TTS
- [ ] 3. Microphone UI with waveform visualization
- [ ] 4. CallManager with LiveKit integration
- [ ] 5. Call UI (video tiles, mute, speaker, end call)
- [ ] 6. Incoming call notification and accept/decline flow
