# Implementation Plan: Calls & Voice

- [ ] 1. Unify and extend VoiceInputManager
  - Normal, Transcribe, Continuous voice modes
  - Live transcription display, auto-submit on pause, cancel
  - _Requirements: 1.1–1.7_
  - _writes: iOS: `Services/VoiceInputManager.swift` (extend); Android: `data/repository/VoiceManager.kt` (extend)_

- [ ] 2. Extend VoiceOutputManager with TTS
  - Speak assistant responses when voice mode enabled
  - Queue management for multiple responses
  - _Requirements: 1.6_
  - _writes: iOS: `Services/VoiceOutputManager.swift` (extend); Android: `data/repository/VoiceManager.kt` (extend)_

- [ ] 3. Implement microphone UI with waveform
  - Visualization during recording, live transcription display
  - _Requirements: 1.2, 1.3_
  - _writes: iOS: `Components/MicrophoneView.swift`; Android: `ui/components/MicrophoneView.kt`_

- [ ] 4. Implement CallManager with LiveKit
  - Create/join/leave rooms, participant management, mute/speaker/camera controls
  - _Requirements: 2.1–2.6_
  - _writes: iOS: `Services/CallManager.swift`; Android: `data/repository/CallManager.kt`_

- [ ] 5. Implement call UI
  - Video tiles, mute/speaker/camera/end buttons, participant list, speaking indicator
  - _Requirements: 2.3–2.6, 3.1–3.4_
  - _writes: iOS: `Views/CallView.swift`; Android: `ui/screens/CallScreen.kt`_

- [ ] 6. Implement incoming call flow
  - Background push → notification → in-app overlay → accept/decline
  - _Requirements: 2.6_
  - _writes: iOS: `Views/IncomingCallOverlay.swift`; Android: `ui/components/IncomingCallOverlay.kt`_
