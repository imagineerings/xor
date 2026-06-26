# Android Feature Parity - Requirements

## Overview
Bring the Android Baymax client to feature parity with the iOS implementation. The iOS app is production-ready with sophisticated streaming, session management, voice features, and tunnel integration. Android currently has ~30% feature coverage.

## Feature Areas

### 1. Voice Input & Output (Priority 1)
**User Story**: As a mobile user, I want to use voice to interact with Baymax so I can use it hands-free.

#### EARS Acceptance Criteria
- WHEN user taps microphone button THEN THE app SHALL start speech recognition
- WHEN speech recognition returns partial results THEN THE app SHALL display live transcription in input field
- WHEN user stops speaking THEN THE app SHALL finalize transcription and optionally auto-submit
- WHEN assistant response completes THEN THE app SHALL speak the response via TTS if voice mode is enabled
- IF user cancels voice input THEN THE app SHALL discard transcription and return to text input

#### Voice Modes (matching iOS)
- **Normal**: Tap-to-record, transcribe on stop, manual submit
- **Transcribe**: Tap-to-record, live partials, auto-submit on silence
- **Continuous**: Always listening, live partials, TTS responses, hands-free conversation

### 2. Agent Configuration & Storage (Priority 1)
**User Story**: As a user with multiple Baymax instances, I want to save and switch between agent configurations easily.

#### EARS Acceptance Criteria
- WHEN user configures a new server URL/secret THEN THE app SHALL offer to save it as named agent
- WHEN user opens settings THEN THE app SHALL display list of saved agents with name, URL preview, last used
- WHEN user taps a saved agent THEN THE app SHALL switch configuration and test connection
- WHEN user scans QR code THEN THE app SHALL parse baymaxchat://configure URL and apply settings
- IF connection test fails for Tailscale URL THEN THE app SHALL show Tailscale-specific error with deep link

### 3. QR Code / Deep Link Configuration (Priority 1)
**User Story**: As a user, I want to configure the app by scanning a QR code from my desktop.

#### EARS Acceptance Criteria
- WHEN app receives baymaxchat://configure?data=<encoded> intent THEN THE app SHALL decode and apply configuration
- WHEN configuration applies successfully THEN THE app SHALL test connection and show success/error
- IF configuration URL uses Tailscale THEN THE app SHALL detect and offer to open Tailscale app

### 4. Session Polling for Live Updates (Priority 2)
**User Story**: As a user resuming a session, I want to see new messages that arrived while I was away.

#### EARS Acceptance Criteria
- WHEN user loads a session with recent activity THEN THE app SHALL start polling for updates
- WHEN polling detects new messages THEN THE app SHALL append them to conversation
- WHEN polling receives 404 THEN THE app SHALL stop polling (session deleted)
- WHILE polling active AND no changes for 20 seconds THEN THE app SHALL stop polling
- IF user sends new message THEN THE app SHALL stop polling and switch to streaming

### 5. Tool Call Visualization (Priority 2)
**User Story**: As a user, I want to see tool calls and their results inline in the conversation.

#### EARS Acceptance Criteria
- WHEN SSE stream emits toolRequest THEN THE app SHALL show loading tool card
- WHEN SSE stream emits toolResponse THEN THE app SHALL show result with duration
- WHEN multiple consecutive tool calls occur THEN THE app SHALL group them visually
- IF tool call fails THEN THE app SHALL show error state with retry option

### 6. Message Grouping & Memory Management (Priority 2)
**User Story**: As a user with long conversations, I want smooth performance without memory issues.

#### EARS Acceptance Criteria
- WHEN conversation exceeds 50 messages THEN THE app SHALL prune oldest (keep first system message)
- WHEN tool calls exceed 20 completed THEN THE app SHALL prune oldest completed calls
- WHEN consecutive assistant messages have only tool calls THEN THE app SHALL group them

### 7. App Notices System (Priority 3)
**User Story**: As a user, I want to be notified of connection issues or required updates.

#### EARS Acceptance Criteria
- WHEN API returns 503 THEN THE app SHALL show "Tunnel disabled" notice
- WHEN API returns decoding error THEN THE app SHALL show "App needs update" notice
- WHEN private network URL fails to connect THEN THE app SHALL show "Tunnel unreachable" notice
- WHEN user taps notice THEN THE app SHALL dismiss or navigate to relevant screen

### 8. Tunnel Detection & Integration (Priority 3)
**User Story**: As a user behind NAT, I want the app to detect and help configure tunnels.

#### EARS Acceptance Criteria
- WHEN server URL contains .ts.net or 100.x.x.x THEN THE app SHALL detect Tailscale
- WHEN server URL matches Cloudflare tunnel pattern THEN THE app SHALL detect Cloudflare
- IF Tailscale detected and not connected THEN THE app SHALL offer to open Tailscale app

### 9. Trial Mode Session Management (Priority 3)
**User Story**: As a trial user, I want my session to persist across app launches.

#### EARS Acceptance Criteria
- WHEN in trial mode AND no session exists THEN THE app SHALL create one on first message
- WHEN trial session created THEN THE app SHALL persist session ID locally
- WHEN app relaunches in trial mode THEN THE app SHALL resume existing trial session

### 10. Markdown Rendering (Priority 3)
**User Story**: As a user, I want formatted responses with code blocks, tables, lists.

#### EARS Acceptance Criteria
- WHEN assistant response contains markdown THEN THE app SHALL render with formatting
- WHEN response contains code blocks THEN THE app SHALL syntax highlight
- WHEN response contains tables THEN THE app SHALL render as formatted tables