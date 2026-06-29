# Implementation Plan: Core Infrastructure & Connectivity

## Overview

This plan covers the cross-platform infrastructure layer that both iOS and Android clients depend on. Tasks are ordered to build from the ground up: (1) foundation interfaces and data models, (2) connection lifecycle and credential management, (3) tunnel and provenance detection, (4) SSE streaming and real-time communication, (5) session management, (6) integration testing. Each task produces files for both platforms unless otherwise noted.

**Key constraints:**
- iOS tasks write to `mobile/ios/Baymax/`
- Android tasks write to `mobile/android/app/src/main/java/com/simtropolis/baymax/`
- All platform-agnostic algorithms (provenance detection, state machine) are documented identically but implemented per-platform

## Tasks

- [ ] 1. Define shared data models and state enums
  - Define `ConnectionState` enum with all 10 states and `Provenance`, `TunnelType` enums
  - Define `AgentConfiguration` struct with all fields and validation rules
  - Define `ChatSession`, `Message`, `SSEEvent` models
  - Define `ConnectionError` enum hierarchy
  - _Requirements: 1.1, 2.1, 4.1, 5.1_
  - _writes: iOS: `Models/ConnectionState.swift`, `Models/AgentConfiguration.swift`, `Models/ChatSession.swift`, `Models/Message.swift`, `Models/SSEEvent.swift`_
  - _writes: Android: `data/model/ConnectionState.kt`, `data/model/AgentConfiguration.kt`, `data/model/ChatSession.kt`, `data/model/Message.kt`, `data/model/SSEEvent.kt`_

- [ ] 2. Implement TunnelDetector (pure function, identical on both platforms)
  - Implement `detectTunnelType(url:)`, `detectProvenance(url:)`, `isPrivateNetworkURL(url:)`, `tunnelErrorMessage(url:)` functions
  - Test with URL corpus covering: Tailscale (100.x.x.x, *.ts.net), Cloudflare, SSH (127.0.0.1, localhost), Trial (demo-baymaxed.fly.dev), Direct LAN
  - _Requirements: 2.1, 2.2, 2.3, 2.6_
  - _writes: iOS: `Utils/TunnelDetector.swift`, `Utils/TunnelDetectorTests.swift`_
  - _writes: Android: `util/TunnelDetector.kt`, `util/TunnelDetectorTest.kt`_

- [ ] 3. Implement Connection State Machine
  - Implement state enum with all valid transitions as a guarded state machine
  - Implement `transition(event:)` method that rejects illegal transitions
  - Implement observable state wrapper for UI binding (iOS: `@Published`, Android: `StateFlow`)
  - Write property-based tests covering all transitions
  - _Requirements: 1.1–1.13_
  - _writes: iOS: `Services/ConnectionStateMachine.swift`, `Services/ConnectionStateMachineTests.swift`_
  - _writes: Android: `data/repository/ConnectionStateMachine.kt`, `.../ConnectionStateMachineTest.kt`_

- [ ] 4. Implement CredentialManager with secure storage
  - Agent CRUD: save, load, update, delete, switch
  - Secure storage integration (iOS: Keychain via `Security` framework, Android: `EncryptedSharedPreferences`)
  - QR code URL parsing (`baymaxchat://configure?data=...` format)
  - Biometric lock (iOS: `LocalAuthentication`, Android: `BiometricPrompt`)
  - Unit tests for each operation with credential isolation validation
  - _Requirements: 5.1–5.11_
  - _writes: iOS: `Services/CredentialManager.swift`, `Services/CredentialManagerTests.swift`_
  - _writes: Android: `data/repository/CredentialManager.kt`, `data/repository/CredentialManagerTest.kt`_

- [ ] 5. Implement AgentAPIService with HTTP client and retry
  - Implement `testConnection()`, `getStatus()` endpoints
  - Implement retry wrapper with exponential backoff
  - Implement error classification into `ConnectionError` hierarchy
  - Integrate with `TunnelDetector` for provenance-aware error messages
  - _Requirements: 1.2–1.14_
  - _writes: iOS: `Services/AgentAPIService.swift` (extend existing), `Services/AgentAPIServiceTests.swift`_
  - _writes: Android: `data/api/BaymaxApiService.kt` (extend existing), `.../BaymaxApiServiceTest.kt`_

- [ ] 6. Implement SSE streaming for agent responses
  - Implement SSE stream connection via native HTTP streaming
  - Implement SSE parser (token, toolCall, toolResult, endStream, error events)
  - Implement stream lifecycle management (start, cancel, reconnect)
  - Implement incremental token delivery to UI via async stream
  - _Requirements: 3.1–3.7_
  - _writes: iOS: `Services/SSEStreamManager.swift`, `Services/SSEStreamManagerTests.swift`_
  - _writes: Android: `data/repository/SSEStreamManager.kt`, `.../SSEStreamManagerTest.kt`_

- [ ] 7. Implement SessionManager with pagination and favorites
  - Fetch sessions with date-based pagination (initial 5 days, increment 5 days)
  - Fetch single session messages
  - Create, delete, rename sessions
  - Local-only favorites (starred/bookmarked) with persistence
  - Session list caching and auto-refresh on foreground
  - _Requirements: 4.1–4.12_
  - _writes: iOS: `Services/SessionManager.swift`, `Services/SessionManagerTests.swift`_
  - _writes: Android: `data/repository/SessionManager.kt`, `.../SessionManagerTest.kt`_

- [ ] 8. Implement CollabWebSocketManager for real-time features
  - WebSocket connection lifecycle (connect, disconnect, reconnect)
  - Heartbeat/ping with 30-second detection
  - Message serialization for collab protocol events
  - Presence, channel message, incoming call event dispatch
  - Graceful degradation in Trial Mode
  - _Requirements: 3.8–3.11_
  - _writes: iOS: `Services/CollabWebSocketManager.swift`, `Services/CollabWebSocketManagerTests.swift`_
  - _writes: Android: `data/repository/CollabWebSocketManager.kt`, `.../CollabWebSocketManagerTest.kt`_

- [ ] 9. Implement ConnectionBanner UI component
  - Reactive connection state → banner mapping
  - All 9 banner states (disconnected, connecting, connected, reconnecting, tailscaleError, tunnelError×2, error×2, trialMode)
  - Action buttons per banner type (Retry, Open Tailscale, Go to Settings, Configure)
  - _Requirements: 1.7, 1.8, 1.12_
  - _writes: iOS: `Components/ConnectionBanner.swift`_
  - _writes: Android: `ui/components/ConnectionBanner.kt`_

- [ ] 10. Wire up connection lifecycle in app entry points
  - On app start: check credentials → auto-connect → navigate to session list or trial
  - On agent switch: disconnect → connect to new agent
  - On network change: detect via NetInfo/ConnectivityManager → trigger reconnect
  - On background → no action (connection stays active); on foreground → verify connection
  - Integration tests for full connection lifecycle
  - _Requirements: 1.2, 1.9, 1.5, 1.8_
  - _writes: iOS: `BaymaxApp.swift` (modify), `AppLifecycleManager.swift`_
  - _writes: Android: `BaymaxApplication.kt` (modify), `data/repository/AppLifecycleManager.kt`_

## Notes

- iOS files with the same name as existing files (e.g., `BaymaxAPIService.swift`, `ConfigurationHandler.swift`) extend/modify the existing implementation rather than replacing it entirely
- All tunnel detection logic must be identical on both platforms — the algorithm in the design doc is the reference
- Model files import existing types where possible (e.g., Android's `ChatSession.kt` already exists in `data/model/`)
- Testing should use MockWebServer (Android) and URLProtocol mocking (iOS) for HTTP/SSE tests
