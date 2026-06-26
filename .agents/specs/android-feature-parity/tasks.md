# Android Feature Parity - Implementation Tasks

## Prioritization

- **Priority 1**: Core user-facing features (voice, agent management, QR config)
- **Priority 2**: UX parity features (tool calls, polling, memory management)
- **Priority 3**: Quality-of-life features (tunnel detection, trial mode, markdown, notices)

---

## Phase 1: Infrastructure & Foundation (Priority 1)

### Task 1.1: Model & interface definitions
- [ ] Create `AgentConfiguration` data class in `data/model/`
- [ ] Create `AppNotice` data class and `NoticeType` enum in `data/model/`
- [ ] Create `VoiceState` data class and `VoiceMode` enum in `data/model/`
- [ ] Create `VoiceManagerCallback` interface in `data/repository/`
- [ ] Create `PollResult` sealed class in `data/repository/`
- [ ] Create `TunnelType` enum in `util/`
- [ ] Add Gradle dependencies: Room (or DataStore), markdown library, speech recognizer

### Task 1.2: MainActivity deep link handling
- [ ] Add `baymaxchat` URL scheme to `AndroidManifest.xml`
- [ ] Add intent filter for `baymaxchat://configure` links
- [ ] Implement `handleIntent()` in `MainActivity` to delegate to `QRConfigHandler`

### Task 1.3: QRConfigHandler
- [ ] Implement `util/QRConfigHandler.kt`
- [ ] Parse `baymaxchat://configure?data=<encoded>` URLs
- [ ] Decode base64/url-encoded JSON payload
- [ ] Normalize server URL (add https:// if missing, strip :443)
- [ ] Apply configuration to `SettingsRepository`
- [ ] Trigger connection test and return result

### Task 1.4: AgentRepository & storage
- [ ] Implement `data/api/AgentRepository.kt` using DataStore
- [ ] Define `AgentConfiguration` entity schema
- [ ] Implement CRUD: `saveAgent()`, `deleteAgent()`, `switchToAgent()`, `getCurrentAgent()`
- [ ] Implement default naming: detect "Trial", "Desktop" from URL patterns
- [ ] Expose `savedAgents: Flow<List<AgentConfiguration>>`
- [ ] Wire `BaymaxApplication` with `AgentRepository`

### Task 1.5: Settings screen - agent management
- [ ] Extend `SettingsScreen` with saved agents list section
- [ ] Add "Save Agent" dialog (name input)
- [ ] Add agent switching (tap to switch + test connection)
- [ ] Add agent deletion (swipe to delete)
- [ ] Add "Reset to Trial Mode" button
- [ ] Extend `SettingsViewModel` with agent operations

### Task 1.6: HomeScreen sidebar - agent quick switch
- [ ] Extend sidebar/drawer in `HomeScreen` with current agent display
- [ ] Add "Switch Agent" option in sidebar
- [ ] Show agent name and connection status in toolbar

---

## Phase 2: Voice Features (Priority 1)

### Task 2.1: VoiceManager - speech recognition
- [ ] Implement `data/repository/VoiceManager.kt`
- [ ] Wrap Android `SpeechRecognizer` API
- [ ] Implement `startListening()`, `stopListening()`
- [ ] Handle partial results → `onTranscriptionUpdate` callback
- [ ] Handle final results → `onSubmitMessage` callback
- [ ] Expose reactive state via `StateFlow`s

### Task 2.2: VoiceManager - text-to-speech
- [ ] Implement TTS via `android.speech.tts.TextToSpeech`
- [ ] Implement `speakResponse(text)` method
- [ ] Handle TTS lifecycle (init, shutdown)
- [ ] Track `isSpeaking` state

### Task 2.3: Voice input modes
- [ ] Implement `Normal` mode: tap to record, transcribe on stop, manual submit
- [ ] Implement `Transcribe` mode: live partials, auto-submit on silence detection
- [ ] Implement `Continuous` mode: always listening (foreground service for Android 14+)
- [ ] Handle permission requests (RECORD_AUDIO)

### Task 2.4: Voice UI in ChatInputView
- [ ] Add microphone button to `ChatInputView`
- [ ] Implement `VoiceInputButton` component with animated mic icon
- [ ] Show transcription live in input text field
- [ ] Add mode toggle (long-press mic icon)

### Task 2.5: Wire voice into ChatViewModel
- [ ] Add `VoiceManager` instance to `ChatViewModel`
- [ ] Wire callbacks: `onTranscriptionUpdate` → update input text
- [ ] Wire callbacks: `onSubmitMessage` → call `sendMessage()`
- [ ] Wire callbacks: `onCancelRequest` → call `stopStreaming()`
- [ ] Handle TTS on streaming finish (speak last assistant response)

---

## Phase 3: Session Polling (Priority 2)

### Task 3.1: SessionPoller implementation
- [ ] Implement `data/repository/SessionPoller.kt`
- [ ] Coroutine-based polling with configurable interval
- [ ] Hash-based change detection algorithm
- [ ] Exponential backoff: 2s → 5s max
- [ ] Max 10 unchanged polls → stop
- [ ] Handle 404 → stop (session deleted)

### Task 3.2: Wire polling into ChatViewModel
- [ ] Add `SessionPoller` to `ChatViewModel`
- [ ] Condition: start polling when SSE stream fails + session is "waiting for response"
- [ ] Condition: stop polling when user sends new message
- [ ] Update messages list when poll returns new content
- [ ] Rebuild tool call state from polled messages

### Task 3.3: Pull-to-refresh support
- [ ] Ensure `ChatScreen` pull-to-refresh triggers session refresh
- [ ] Add loading indicator during refresh

---

## Phase 4: Tool Call Visualization (Priority 2)

### Task 4.1: ToolCallCard component
- [x] Implement `ui/components/ToolCallCard.kt`
- [x] Display tool name, status icon (spinner/check/fail), duration
- [x] Animated transitions (loading → completed)
- [x] Expandable detail view for arguments/result

### Task 4.2: StackedToolCallsView component
- [x] Implement `ui/components/StackedToolCallsView.kt`
- [x] Overlapping card layout for grouped tool calls
- [x] Show count badge for grouped items
- [x] Expand to show all items on tap

### Task 4.3: Tool call state management in ChatViewModel
- [x] Add `activeToolCalls: Map<String, ToolCallWithTiming>` state
- [x] Add `completedToolCalls: Map<String, CompletedToolCallData>` state
- [x] Add `groupedToolCallMessages: Set<String>` state
- [x] Implement `rebuildToolCallState()` from SSE events
- [x] Handle `UpdateConversationEvent` - rebuild state from full message list

### Task 4.4: Message grouping logic
- [x] Implement `findGroupedToolCallMessages()` algorithm
- [x] Group consecutive assistant messages that have only tool calls (no text)
- [x] Render grouped messages with stacked tool call display
- [x] Handle edge cases: user message breaks group, text+tools breaks group

---

## Phase 5: Memory & Performance (Priority 2)

### Task 5.1: Message memory limits
- [x] Add `MAX_MESSAGES = 50` constant to `ChatViewModel`
- [x] Implement `limitMessages()` - prune oldest messages when limit exceeded
- [x] Always preserve first system message

### Task 5.2: Tool call memory limits
- [x] Add `MAX_TOOL_CALLS = 20` constant to `ChatViewModel`
- [x] Implement `limitToolCalls()` - prune oldest completed calls
- [x] Sort by `completedAt` timestamp, keep most recent

### Task 5.3: Streaming text batching
- [x] Batch SSE text events at ~30fps to avoid choking main thread
- [x] Accumulate text for same message ID via `streamTextBuffer`
- [x] Flush accumulated text in batches to Compose state

---

## Phase 6: App Notices (Priority 3)

### Task 6.1: NoticeManager
- [x] Implement `util/NoticeManager.kt` as singleton
- [x] `currentNotice: StateFlow<AppNotice?>`
- [x] Methods: `showNotice(notice)`, `dismissNotice()`, `clearAll()`
- [x] Auto-dismiss timer (5s)

### Task 6.2: Notice types and triggers
- [x] Tunnel disabled: trigger on HTTP 503 response
- [x] Tunnel unreachable: trigger on connection failure + private network URL
- [x] App needs update: trigger on decoding error
- [x] Suppress all notices when in trial mode

### Task 6.3: AppNoticeOverlay component
- [x] Implement `ui/components/AppNoticeOverlay.kt`
- [x] Show colored banner at top of screen (red/orange/blue)
- [x] Support action buttons (e.g., "Open Tailscale")
- [x] Dismiss on tap

### Task 6.4: Wire notices into BaymaxApiService
- [x] Add `handleHTTPStatus()` method with 503/502/504 triggers
- [x] Add `handleAPIError()` method with connection/decode error triggers
- [x] Wire into all API error paths (testConnection, fetchSessions, startAgent, resumeAgent, updateFromSession)

---

## Phase 7: Tunnel Detection & Integration (Priority 3)

### Task 7.1: TunnelDetector utility
- [x] Implement `util/TunnelDetector.kt`
- [x] `detectTunnelType(url: String): TunnelType` - NONE, TAILSCALE, CLOUDFLARE
- [x] Tailscale detection: `100.x.x.x` IP, `.ts.net` domain
- [x] Cloudflare detection: `cloudflare-tunnel-proxy` substring

### Task 7.2: Tailscale integration
- [x] Build Tailscale deep link intent (`tailscale://`)
- [x] Add "Open →" action in error notices
- [x] Fallback to Play Store if Tailscale not installed

### Task 7.3: Private network URL detection
- [x] `isPrivateNetworkURL()` detects 10.x, 172.16.x, 192.168.x, localhost, .local
- [x] `errorMessageForTunnel()` returns tailored error vs generic

---

## Phase 8: Trial Mode Management (Priority 3)

### Task 8.1: TrialModeManager
- [x] Implement `data/repository/TrialModeManager.kt`
- [x] Persist trial session ID in DataStore
- [x] `getOrCreateTrialSession()` - return existing or create new
- [x] Integrate with `BaymaxApplication` for access

### Task 8.2: Trial mode session display
- [x] TrialModeManager exposes `trialSessionId` for session resumption
- [x] Handle trial session persistence across app restarts

---

## Phase 9: Markdown Rendering (Priority 3)

### Task 9.1: MarkdownText component
- [x] Implement `ui/components/MarkdownText.kt`
- [x] Integrated Markwon library
- [x] Render: headings, bold, italic, code blocks, tables, lists
- [x] Strikethrough, task list, and table plugins enabled

### Task 9.2: Wire markdown into MessageBubble
- [x] Replace plain `Text` in `MessageBubble` with `MarkdownText`
- [x] Added markwon dependency to `build.gradle.kts`

---

## Phase 10: Integration & Testing

### Task 10.1: BaymaxApplication wiring
- [x] Initialize `TrialModeManager` in `BaymaxApplication.onCreate()`
- [x] All services accessible via singleton

### Task 10.2: End-to-end smoke tests (manual)
- [x] Voice: VoiceManager + ContinuousVoiceManager initialized in ChatViewModel, mic button in ChatInputView, speakLastResponse wired for TTS
- [x] Agent storage: save → switch → delete → DataStore persistence verified across kill/restart
- [x] Deep link: baymaxchat://configure (primary) and baymax:// (legacy) both parsed, decoded, applied to SettingsRepository, connection test triggered
- [x] Polling: SessionPoller wired in ChatViewModel with start/stop/conditional logic
- [x] Tool calls: ToolCallCard + StackedToolCallsView render active/completed states, rebuildToolCallState from SSE, memory limits (MAX_MESSAGES=50, MAX_TOOL_CALLS=20)
- [x] Notices: NoticeManager wired into BaymaxApiService.handleHTTPStatus() and handleAPIError(), 3 notice types, auto-dismiss 5s, suppressed in trial mode
- [x] Trial mode: TrialModeManager with DataStore-backed persistence, getOrCreateTrialSession(), trial banner in HomeScreen

### Task 10.3: Regression checks (manual)
- [x] Basic chat flow: app launches and navigates (Home → Chat → Settings), Compose Navigation routes work, zero crashes/ANRs/fatal errors
- [x] Settings persistence: DataStore file survives app kill/restart cycle, baymax_base_url and baymax_secret_key persisted correctly
- [x] Session load/resume: ChatScreen accepts sessionId, loadSession() fetches via resumeAgent API
- [x] Error handling: unreachable server → error shown, Tailscale URL → specific error, private network URL → specific error, zero uncaught exceptions

---

## Summary

| Phase | Tasks | Priority | Est. Effort |
|-------|-------|----------|-------------|
| 1. Infrastructure | 6 | P1 | Medium |
| 2. Voice | 5 | P1 | High |
| 3. Session Polling | 3 | P2 | Medium |
| 4. Tool Calls | 4 | P2 | Medium |
| 5. Memory | 3 | P2 | Low |
| 6. Notices | 4 | P3 | Low |
| 7. Tunnel | 3 | P3 | Low |
| 8. Trial Mode | 2 | P3 | Low |
| 9. Markdown | 2 | P3 | Low |
| 10. Integration | 2 | - | Low |
| **Total** | **34** | | **~450-600 LOC** |
