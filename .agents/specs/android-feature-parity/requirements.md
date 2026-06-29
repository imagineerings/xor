# Android Feature Parity - Requirements

## Overview
Bring the Android Baymax client to feature parity with the iOS implementation. This spec documents the **complete roadmap** from initial infrastructure (Phases 1-10, ✅ completed) through remaining gaps and polish (Phases 11-15, ❌ remaining).

---

## ✅ Phases 1-10: Foundation & Core Parity (COMPLETED)

The following work was completed in the initial parity effort (commit `2d197f6` on `android-feature-parity` branch). All 34 tasks are implemented.

### 1. Voice Input & Output (Priority 1) — ✅ DONE
**User Story**: As a mobile user, I want to use voice to interact with Baymax so I can use it hands-free.

| Criteria | Status |
|---|---|
| WHEN user taps microphone button THEN THE app SHALL start speech recognition | ✅ |
| WHEN speech recognition returns partial results THEN THE app SHALL display live transcription | ✅ |
| WHEN user stops speaking THEN THE app SHALL finalize transcription and optionally auto-submit | ✅ |
| WHEN assistant response completes THEN THE app SHALL speak via TTS if voice mode enabled | ✅ |
| IF user cancels voice input THEN THE app SHALL discard transcription | ✅ |
| **Voice Modes**: Normal, Transcribe, Continuous | ✅ |

### 2. Agent Configuration & Storage (Priority 1) — ✅ DONE
**User Story**: As a user with multiple Baymax instances, I want to save and switch between agent configurations.

| Criteria | Status |
|---|---|
| WHEN user configures a new server URL/secret THEN THE app SHALL offer to save as named agent | ✅ |
| WHEN user opens settings THEN THE app SHALL display list of saved agents | ✅ |
| WHEN user taps a saved agent THEN THE app SHALL switch configuration and test connection | ✅ |
| WHEN user scans QR code THEN THE app SHALL parse baymaxchat://configure URL | ✅ |
| IF connection test fails for Tailscale URL THEN THE app SHALL show Tailscale-specific error | ✅ |

### 3. QR Code / Deep Link Configuration (Priority 1) — ✅ DONE
**User Story**: As a user, I want to configure the app by scanning a QR code from my desktop.

| Criteria | Status |
|---|---|
| WHEN app receives baymaxchat://configure?data=<encoded> intent THEN THE app SHALL decode and apply | ✅ |
| WHEN configuration applies successfully THEN THE app SHALL test connection and show result | ✅ |
| IF configuration URL uses Tailscale THEN THE app SHALL detect and offer to open Tailscale app | ✅ |

### 4. Session Polling for Live Updates (Priority 2) — ✅ DONE
**User Story**: As a user resuming a session, I want to see new messages that arrived while I was away.

| Criteria | Status |
|---|---|
| WHEN user loads session with recent activity THEN THE app SHALL start polling | ✅ |
| WHEN polling detects new messages THEN THE app SHALL append them | ✅ |
| WHEN polling receives 404 THEN THE app SHALL stop polling | ✅ |
| WHILE polling active AND no changes for 20s THEN THE app SHALL stop polling | ✅ |
| IF user sends new message THEN THE app SHALL stop polling and switch to streaming | ✅ |

### 5. Tool Call Visualization (Priority 2) — ✅ DONE
**User Story**: As a user, I want to see tool calls and their results inline in the conversation.

| Criteria | Status |
|---|---|
| WHEN SSE emits toolRequest THEN THE app SHALL show loading tool card | ✅ |
| WHEN SSE emits toolResponse THEN THE app SHALL show result with duration | ✅ |
| WHEN multiple consecutive tool calls occur THEN THE app SHALL group them visually | ✅ |
| IF tool call fails THEN THE app SHALL show error state with retry option | ✅ |

### 6. Message Grouping & Memory Management (Priority 2) — ✅ DONE
**User Story**: As a user with long conversations, I want smooth performance without memory issues.

| Criteria | Status |
|---|---|
| WHEN conversation exceeds 50 messages THEN THE app SHALL prune oldest (keep first system message) | ✅ |
| WHEN tool calls exceed 20 completed THEN THE app SHALL prune oldest completed calls | ✅ |
| WHEN consecutive assistant messages have only tool calls THEN THE app SHALL group them | ✅ |

### 7. App Notices System (Priority 3) — ✅ DONE
**User Story**: As a user, I want to be notified of connection issues or required updates.

| Criteria | Status |
|---|---|
| WHEN API returns 503 THEN THE app SHALL show "Tunnel disabled" notice | ✅ |
| WHEN API returns decoding error THEN THE app SHALL show "App needs update" notice | ✅ |
| WHEN private network URL fails to connect THEN THE app SHALL show "Tunnel unreachable" notice | ✅ |
| WHEN user taps notice THEN THE app SHALL dismiss or navigate to relevant screen | ✅ |

### 8. Tunnel Detection & Integration (Priority 3) — ✅ DONE
**User Story**: As a user behind NAT, I want the app to detect and help configure tunnels.

| Criteria | Status |
|---|---|
| WHEN server URL contains .ts.net or 100.x.x.x THEN THE app SHALL detect Tailscale | ✅ |
| WHEN server URL matches Cloudflare tunnel pattern THEN THE app SHALL detect Cloudflare | ✅ |
| IF Tailscale detected and not connected THEN THE app SHALL offer to open Tailscale app | ✅ |

### 9. Trial Mode Session Management (Priority 3) — ✅ DONE
**User Story**: As a trial user, I want my session to persist across app launches.

| Criteria | Status |
|---|---|
| WHEN in trial mode AND no session exists THEN THE app SHALL create one on first message | ✅ |
| WHEN trial session created THEN THE app SHALL persist session ID locally | ✅ |
| WHEN app relaunches in trial mode THEN THE app SHALL resume existing trial session | ✅ |

### 10. Markdown Rendering (Priority 3) — ✅ DONE (Basic)
**User Story**: As a user, I want formatted responses with code blocks, tables, lists.

| Criteria | Status |
|---|---|
| WHEN assistant response contains markdown THEN THE app SHALL render with formatting | ✅ |
| WHEN response contains code blocks THEN THE app SHALL syntax highlight | ⚠️ Basic only |
| WHEN response contains tables THEN THE app SHALL render as formatted tables | ✅ |

---

## ❌ Phases 11-15: Remaining Gaps & Polish (IN PROGRESS)

The following features were identified as gaps after the initial parity effort. These represent the remaining ~15% to full parity.

### 11. Welcome Screen Enhancements (NEW — Feature Area D)

#### Requirement D.1: Animated Splash Screen
**User Story**: As a user launching the app, I want to see an animated splash screen with the Baymax logo.

| Criteria | Status |
|---|---|
| WHEN the app is launched THEN THE system SHALL show a splash screen with Baymax logo centered | ❌ |
| WHEN the splash screen appears THEN THE system SHALL fade in the logo over 0.2-0.4 seconds | ❌ |
| IF first launch OR app not opened in 24+ hours THEN THE system SHALL show splash for ~1.8 seconds | ❌ |
| IF app opened recently (<24 hours) THEN THE system SHALL show splash for ~0.7 seconds | ❌ |
| WHEN splash animation completes THEN THE system SHALL transition to main content | ❌ |

#### Requirement D.2: Welcome Card Typewriter Effect
**User Story**: As a user on the home screen, I want the welcome greeting to animate character-by-character.

| Criteria | Status |
|---|---|
| WHEN the welcome card is displayed THEN THE system SHALL animate greeting text with typewriter effect (20ms per character) | ❌ |

#### Requirement D.3: Token Progress Bar & Session Density
**User Story**: As a user on the home screen, I want to see token usage and session density info.

| Criteria | Status |
|---|---|
| WHEN home screen loads THEN THE system SHALL fetch `/sessions/insights` for total token usage | ❌ |
| IF insights API available THEN THE system SHALL display token progress bar with formatting (e.g., "450M") | ❌ |
| IF insights API unavailable or trial mode THEN THE system SHALL display mock data (5 sessions, 450M tokens) | ❌ |
| WHEN viewing sessions for a specific day THEN THE system SHALL detect density: quiet (≤2), light (3-5), busy (>5) | ❌ |
| WHEN density detected THEN THE system SHALL adjust greeting: "Quiet yesterday", "Light yesterday", "Busy yesterday" | ❌ |

### 12. Rich Markdown Rendering (NEW — Feature Area E)

#### Requirement E.1: Multi-Language Syntax Highlighting
**User Story**: As a user reading code blocks, I want syntax-highlighted code in multiple languages.

| Criteria | Status |
|---|---|
| WHEN a code block is rendered THEN THE system SHALL detect language from language tag | ❌ |
| WHEN language detected THEN THE system SHALL apply highlighting for: Swift, Python, JavaScript, TypeScript, JSON, Shell, Ruby, Go, Rust, SQL, HTML, CSS | ❌ |
| WHEN syntax highlighting applied THEN THE system SHALL colorize: strings (orange), comments (green), keywords (blue/purple), numbers (light blue), functions (yellow), types (teal) | ❌ |
| WHEN code block has no language tag THEN THE system SHALL render as plain monospace | ❌ |
| WHEN code block rendered THEN THE system SHALL show language badge in top-right corner | ❌ |

#### Requirement E.2: Markdown Test View (Debug)
**User Story**: As a developer, I want a debug screen to test markdown rendering.

| Criteria | Status |
|---|---|
| WHEN app built in debug mode THEN Settings screen SHALL show "Markdown Test" link | ❌ |
| WHEN developer taps link THEN THE system SHALL display comprehensive test markdown | ❌ |
| WHEN test view displayed THEN THE system SHALL render in both assistant-message and user-message styles | ❌ |

### 13. API Completeness (NEW — Feature Area G)

#### Requirement G.1: Session Insights API
**User Story**: As the welcome screen, I want to fetch `/sessions/insights` to display token usage and session counts.

| Criteria | Status |
|---|---|
| `SessionInsights` data model | ✅ Exists in `ChatSession.kt` |
| `fetchInsights(): ApiResult<SessionInsights>` | ❌ Missing |
| IF API fails or trial mode THEN return mock data (5 sessions, 450M tokens) | ❌ |

#### Requirement G.2: Model Provider Update API
**User Story**: As the chat system, I want to support updating the model provider for a session.

| Criteria | Status |
|---|---|
| `updateProvider(sessionId, provider, model): ApiResult<Unit>` — `POST /agent/update_provider` | ❌ Missing |
| IF API returns error THEN log and continue with current provider | ❌ |

#### Requirement G.3: Extension Loading API
**User Story**: As the chat system, I want to load enabled extensions from the server.

| Criteria | Status |
|---|---|
| `loadEnabledExtensions(): ApiResult<List<String>>` — `GET /config/extensions` | ❌ Missing |
| WHEN extensions loaded THEN pass them when resuming agent session | ❌ |

#### Requirement G.4: Retry Logic for Transient Errors
**User Story**: As the API client, I want automatic retries for transient network errors.

| Criteria | Status |
|---|---|
| WHEN network request fails with transient error (timeout, connection lost) THEN THE system SHALL retry up to 2 times with 1-second delay | ❌ |
| IF all retry attempts fail THEN THE system SHALL return error to caller | ❌ |
| IF request succeeds on retry THEN THE system SHALL return successful result | ❌ |

### 14. Visual Session Navigation (NEW — Feature Area H)

#### Requirement H.1: NodeMatrix / NodeFocus
**User Story**: As a user on the home screen, I want to see a visual grid of circular nodes representing my sessions so I can intuitively navigate by date.

| Criteria | Status |
|---|---|
| WHEN home screen has sessions THEN THE system SHALL render a NodeMatrix — grid of circular nodes arranged by date | ❌ |
| Node size proportional to message count (min 8dp, max 16dp) | ❌ |
| Horizontal swipe gesture to navigate between days | ❌ |
| WHEN session node tapped THEN THE system SHALL show NodeFocus popover with session details | ❌ |
| IF session updated within 5 minutes THEN THE system SHALL render as "live" node with pulsing animation | ❌ |
| IF session favorited THEN node SHALL show star indicator | ❌ |
| Empty state when no sessions for a day | ❌ |
| Loading spinner while sessions load | ❌ |

#### Requirement H.2: SSE NotificationEvent Type
**User Story**: As the SSE protocol consumer, I want to handle `Notification` event types from the server.

| Criteria | Status |
|---|---|
| WHEN server sends `{"type": "Notification"}` SSE event THEN THE system SHALL parse into `SSEEvent.NotificationEvent` | ❌ |
| WHEN NotificationEvent parsed THEN THE system SHALL dispatch via NoticeManager with method + params | ❌ |

### 15. UX Polish (NEW — Feature Area I)

#### Requirement I.1: Intelligent Auto-Scroll
**User Story**: As a user viewing a streaming chat response, I want the view to auto-scroll to show new content, but stay where I am if I'm manually scrolling.

| Criteria | Status |
|---|---|
| WHEN new streaming content arrives THEN THE system SHALL auto-scroll to bottom | ❌ |
| WHEN user manually scrolls up during streaming THEN THE system SHALL stop auto-scrolling | ❌ |
| WHEN user scrolls back to bottom THEN THE system SHALL resume auto-scrolling | ❌ |
| WHEN streaming completes and auto-scroll was active THEN THE system SHALL perform final scroll to bottom | ❌ |

#### Requirement I.2: Configuration Success Feedback
**User Story**: As a user scanning a QR code to configure the app, I want visual feedback that configuration succeeded.

| Criteria | Status |
|---|---|
| WHEN QR configuration succeeds THEN THE system SHALL display success indicator for 3 seconds | ❌ |
| WHEN 3-second timer expires THEN THE system SHALL clear the success indicator | ❌ |

#### Requirement I.3: Dark Mode Toggle
**User Story**: As a user, I want a manual dark mode toggle in settings to override the system theme.

| Criteria | Status |
|---|---|
| WHEN user opens Settings THEN THE system SHALL show "Dark Mode" toggle | ❌ |
| WHEN user toggles dark mode THEN THE system SHALL immediately switch UI theme | ❌ |
| WHEN user toggles dark mode THEN THE system SHALL persist the preference | ❌ |
| WHEN app launches with persisted dark mode preference THEN THE system SHALL apply it | ❌ |

#### Requirement I.4: Voice State Rich UI
**User Story**: As a user using voice mode, I want to see distinct icons and colors for each voice state.

| Criteria | Status |
|---|---|
| WHEN voice state is `idle` THEN THE system SHALL show muted microphone icon in gray | ❌ |
| WHEN voice state is `listening` THEN THE system SHALL show microphone icon in blue | ❌ |
| WHEN voice state is `processing` THEN THE system SHALL show ellipsis/circle icon in orange | ❌ |
| WHEN voice state is `speaking` THEN THE system SHALL show speaker wave icon in green | ❌ |
| WHEN voice state is `error` THEN THE system SHALL show warning triangle icon in red | ❌ |

#### Requirement I.5: Liquid Glass Effects
**User Story**: As a user on a supported device, I want to see glass-morphism visual effects for a modern look.

| Criteria | Status |
|---|---|
| WHEN welcome card displayed on Android API 31+ THEN THE system SHALL apply frosted glass/blur effect | ❌ |
| WHEN device does not support blur effects THEN THE system SHALL fall back to translucent solid surface | ❌ |

---

## Glossary

| Term | Definition |
|---|---|
| **Tool Call** | An LLM-generated request to invoke a specific function/tool, with arguments and results. |
| **SSE** | Server-Sent Events — the streaming protocol used for real-time chat responses. |
| **Trial Mode** | A mode where the app uses a demo server (`demo-baymaxed.fly.dev`) with mock data and limited functionality. |
| **Tailscale** | A WireGuard-based VPN used for secure tunneling between mobile and desktop Baymax instances. |
| **Cloudflare Tunnel** | An alternative tunneling mechanism using Cloudflare's `trycloudflare.com` service. |
| **NodeMatrix** | A session visualization grid on the iOS Welcome screen showing circular nodes for each session. |
| **Markwon** | The Android library used for Markdown rendering (equivalent to iOS's SwiftUI Markdown + custom renderer). |
| **DataStore** | Android Jetpack's key-value persistence (equivalent to iOS UserDefaults). |
| **Syntax Highlighting** | Colorized rendering of code tokens (keywords, strings, comments) for multiple programming languages. |