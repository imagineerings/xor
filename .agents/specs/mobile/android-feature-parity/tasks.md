# Android Feature Parity - Implementation Tasks

## Overview

This plan covers the **complete roadmap** from foundation to full feature parity with iOS.

- **Phases 1-10**: 34 tasks ✅ **COMPLETED** (initial parity effort, commit `2d197f6`)
- **Phases 11-15**: 13 tasks ✅ **COMPLETED** (gaps identified after initial effort)

**Total: 47 tasks** — 100% complete

**Prerequisites**: Kotlin 1.9+, Compose, DataStore dependencies already in place. No new Gradle dependencies required.

---

## ✅ Phases 1-10: Foundation & Core Parity (COMPLETED)

All 34 tasks in the initial parity effort are implemented.

| Phase | Area | Tasks | Status |
|---|---|---|---|
| 1 | Infrastructure & Foundation | 6 | ✅ |
| 2 | Voice Features | 5 | ✅ |
| 3 | Session Polling | 3 | ✅ |
| 4 | Tool Call Visualization | 4 | ✅ |
| 5 | Memory & Performance | 3 | ✅ |
| 6 | App Notices | 4 | ✅ |
| 7 | Tunnel Detection | 3 | ✅ |
| 8 | Trial Mode | 2 | ✅ |
| 9 | Markdown Rendering (Basic) | 2 | ✅ |
| 10 | Integration & Testing | 2 | ✅ |
| **Total** | | **34** | ✅ **Done** |

---

## ✅ Phases 11-15: Remaining Gaps & Polish (COMPLETED)

### Phase 11: Welcome Screen Enhancements (3 tasks)

- [x] **D.1: Create animated SplashScreen**
  - Create `ui/screens/SplashScreen.kt`:
    - `SplashScreen(isActive: MutableState<Boolean>)` composable
    - Fade-in animation with `AnimatedVisibility` or manual alpha animation
    - Track `lastAppOpenTime` and `hasLaunchedBefore` in SharedPreferences
    - Timing logic:
      - First launch or >24h gap: 0.4s fade in, 1.0s display, 0.4s fade out
      - Otherwise: 0.2s fade in, 0.3s display, 0.2s fade out
    - Baymax logo centered (use `R.drawable.ic_baymax_logo`)
  - Integrate into `MainActivity.kt` as overlay before `BaymaxNavigation()`
  - When `isActive` becomes `false`, show main navigation
  - Mirror iOS `SplashScreenView.swift`
  - _Writes: ui/screens/SplashScreen.kt, MainActivity.kt_

- [x] **D.2: Add typewriter effect to WelcomeCard**
  - Modify `WelcomeCard` in `ui/components/WelcomeCard.kt`:
    - Add `greeting: String` parameter
    - Use `LaunchedEffect` with `delay(20)` per character to animate text display
    - Track `displayedText` as `mutableStateOf`
    - Show `displayedText` instead of raw greeting
    - Cancel animation on recomposition (trigger reset when greeting changes)
  - _Writes: WelcomeCard.kt_

- [x] **D.3: Add token progress bar and session density to WelcomeCard**
  - Add `fetchInsights()` method to `BaymaxApiService` (see G.1)
  - Add `fetchInsights()` call to `HomeViewModel`:
    - Fetch on init, store in UI state
    - Return mock data (5 sessions, 450M tokens) in trial mode or on error
  - Modify `WelcomeCard`:
    - Accept `tokenCount: Long?`, `isLoadingTokens: Boolean` parameters
    - Display horizontal progress bar with formatted token count
    - `formatTokenCount(count: Long)`: "450M", "12K", "1B"
  - Add session density detection:
    - Accept `sessions: List<ChatSession>`, `daysOffset: Int`
    - Compute density: quiet (≤2), light (3-5), busy (>5)
    - Adjust greeting: "Quiet yesterday", "Light yesterday", "Busy yesterday"
  - _Writes: BaymaxApiService.kt, HomeViewModel.kt, WelcomeCard.kt, HomeScreen.kt_

### Phase 12: Rich Markdown Rendering (2 tasks)

- [x] **E.1: Implement multi-language syntax highlighting**
  - Create `ui/components/SyntaxHighlighter.kt`:
    - `object SyntaxHighlighter` with `highlight(code: String, language: String): AnnotatedString`
    - Regex-based token matching (mirroring iOS `SharedMessageComponents.swift` `SyntaxHighlighter`)
    - Supported languages: Swift, Python, JavaScript, TypeScript, JSON, Shell, Ruby, Go, Rust, SQL, HTML, CSS
    - Token colors: strings (orange), comments (green), keywords (blue), numbers (teal), functions (yellow), types (teal)
    - Unknown language → plain text
  - Integrate with `MarkdownText.kt`:
    - When Markwon renders a code block, apply `SyntaxHighlighter` to the code text via `SpannableString` + `ForegroundColorSpan`
    - Add language badge in top-right of code blocks
  - _Writes: ui/components/SyntaxHighlighter.kt, MarkdownText.kt_

- [x] **E.2: Create MarkdownTestScreen (debug)**
  - Create `ui/screens/MarkdownTestScreen.kt`:
    - Comprehensive test markdown string covering all supported features
    - Render in both assistant-message and user-message styles
    - "Done" button to dismiss
  - Add route `"markdown-test"` to nav graph, gated by `BuildConfig.DEBUG`
  - Add "Markdown Test" link in `SettingsScreen` (visible only in debug builds)
  - Mirror iOS `MarkdownTestView.swift`
  - _Writes: ui/screens/MarkdownTestScreen.kt, MainActivity.kt, SettingsScreen.kt_

### Phase 13: API Completeness (1 task)

- [x] **G.1: Add remaining API methods and retry logic**
  - `SessionInsights` data class ✅ ALREADY EXISTS in `ChatSession.kt`
  - Add methods to `BaymaxApiService.kt`:
    - `fetchInsights(): ApiResult<SessionInsights>` — `GET /sessions/insights`
    - `updateProvider(sessionId, provider, model): ApiResult<Unit>` — `POST /agent/update_provider`
    - `loadEnabledExtensions(): ApiResult<List<String>>` — `GET /config/extensions`
  - Add retry logic:
    - `fetchWithRetry(maxAttempts: Int = 2, operation: suspend () -> ApiResult<T>): ApiResult<T>`
    - Transient errors to retry: timeout, connection lost, DNS failure, HTTP 502/503/504
    - 1-second delay between attempts
  - _Writes: BaymaxApiService.kt_

### Phase 14: Visual Session Navigation (2 tasks)

- [x] **H.1: Implement NodeMatrix and NodeFocus**
  - Create `NodeMatrix` composable in `ui/components/NodeMatrix.kt`:
    - Compose `Canvas`-based node rendering
    - Nodes sized by message count (min 8dp, max 16dp)
    - Horizontal swipe gesture for day navigation via `pointerInput`
    - `daysOffset` state tracking current day
    - Positioning with caching to avoid recomputation
    - Pulsing animation for live sessions (<5 min since update)
    - Star indicator for favorited sessions
    - Empty state when no sessions for a day
    - Loading spinner while sessions load
  - Create `NodeFocus` composable in `ui/components/NodeFocus.kt`:
    - Popover card showing session details on node tap
    - Session description, message count, working directory
    - "Continue Session" button → navigates to ChatScreen
    - Close/dismiss button
    - Border with subtle blue stroke
  - Integrate into `HomeScreen.kt` — replace or complement existing session list with NodeMatrix
  - Mirror iOS `NodeMatrix.swift` and `NodeFocus.swift`
  - _Writes: ui/components/NodeMatrix.kt, ui/components/NodeFocus.kt, HomeScreen.kt_

- [x] **H.2: Add SSE NotificationEvent type**
  - Extend `SSEEvent` sealed class in `SSEEvent.kt`:
    ```kotlin
    @Serializable
    @SerialName("Notification")
    data class NotificationEvent(
        @SerialName("request_id") val requestId: String,
        val message: NotificationMessage
    ) : SSEEvent()

    @Serializable
    data class NotificationMessage(
        val method: String,
        val params: Map<String, JsonElement> = emptyMap()
    )
    ```
  - Extend `parseSSEEvent()` in `BaymaxApiService.kt`:
    ```kotlin
    "Notification" -> json.decodeFromString<SSEEvent.NotificationEvent>(data)
    ```
  - Dispatch notification via `NoticeManager` when received in SSE stream
  - Mirror iOS `Message.swift` lines 475-493
  - _Writes: SSEEvent.kt, BaymaxApiService.kt_

### Phase 15: UX Polish (5 tasks)

- [x] **I.1: Implement intelligent auto-scroll**
  - Add scroll management state to `ChatScreen.kt`:
    - `shouldAutoScroll: Boolean`
    - `userIsScrolling: Boolean`
    - `lastContentHeight: Int`
    - `lastScrollUpdate: Long`
  - Implement `LazyListState` scroll listener to detect user interruption
  - When `shouldAutoScroll && !userIsScrolling` and new content arrives, call `lazyListState.animateScrollToItem(lastIndex)`
  - Throttle scroll updates to max 1 per 100ms during fast streaming
  - On stream finish, perform final scroll to bottom
  - Mirror iOS `ChatView.swift` lines 42-48
  - _Writes: ChatScreen.kt_

- [x] **I.2: Add configuration success feedback**
  - Add `configurationSuccess: MutableState<Boolean>` to `QRConfigHandler.kt`
  - Set `configurationSuccess = true` when `handleDeepLink()` succeeds
  - Add `LaunchedEffect` with `delay(3000)` to clear the flag
  - Show green success indicator in relevant UI (banner/checkmark) that auto-dismisses after 3s
  - Mirror iOS `ConfigurationHandler.swift` lines 351-353
  - _Writes: QRConfigHandler.kt, MainActivity.kt_

- [x] **I.3: Add dark mode toggle to settings**
  - Create `ThemeManager` object (or extend existing `BaymaxColors` infrastructure):
    - `isDarkMode: MutableState<Boolean>` with persistence to `SharedPreferences`
    - `toggle()` function
  - Add dark mode toggle to `SettingsScreen`:
    - `Switch` with "Dark Mode" label
    - On toggle, call `ThemeManager.toggle()`
  - Modify `BaymaxTheme` composable in `Theme.kt` to observe `ThemeManager.isDarkMode`
  - Update `WelcomeCard` and other components that call `isSystemInDarkTheme()` to use `ThemeManager.isDarkMode` instead
  - Mirror iOS `ThemeManager.swift`
  - _Writes: Theme.kt (or new ThemeManager.kt), SettingsScreen.kt, WelcomeCard.kt_

- [x] **I.4: Add voice state rich UI**
  - Extend `VoiceState` data class in `VoiceState.kt`:
    - Add `stateLabel: String` field (values: "Idle", "Listening", "Processing", "Speaking", "Error")
    - Add computed `iconRes: Int` — returns correct `R.drawable.*` for each state
    - Add computed `tintColor: Color` — returns correct color for each state
  - Update `ContinuousVoiceManager` to set `stateLabel` correctly during state transitions
  - Update voice UI components to use `iconRes` and `tintColor` from `VoiceState`
  - Mirror iOS `ContinuousVoiceManager.swift` lines 23-23-37
  - _Writes: VoiceState.kt, ContinuousVoiceManager.kt, relevant voice UI components_

- [x] **I.5: Add liquid glass effect fallback**
  - Create glass effect modifier utility in `ui/components/LiquidGlassModifier.kt`:
    - API 31+: Use `RenderEffect.createBlurEffect()` + `graphicsLayer` for frosted glass
    - API < 31: Fallback to semi-transparent solid surface with shadow
  - Apply to `WelcomeCard` background
  - Mirror iOS `LiquidGlassModifier.swift` (iOS 26+ with `ultraThinMaterial` fallback)
  - _Writes: ui/components/LiquidGlassModifier.kt, WelcomeCard.kt_

---

## Execution Order

```
Phase A (Foundation APIs):
  G.1 → H.2 → I.2  (API methods, SSE NotificationEvent, config feedback)

Phase B (UI Components):
  D.1 → D.2 → D.3  (splash + welcome card)
  E.1 → E.2        (markdown)
  H.1               (NodeMatrix) — most complex single task

Phase C (Polish):
  I.1 → I.3 → I.4 → I.5  (auto-scroll, dark mode, voice UI, glass effects)
```

---

## Notes

- **No new Gradle dependencies** required for any task.
- All tasks work within existing Kotlin/Compose/Material3 stack.
- iOS stubs (like `ToolConfirmationView` permission response) replicated as stubs.
- Syntax highlighting (E.1) uses `SpannableString` on code block `TextView` after Markwon renders.
- Splash screen (D.1) uses `SharedPreferences` for simplicity.
- All new screens follow existing Compose Navigation pattern in `MainActivity.kt`.
- NodeMatrix (H.1) is most complex task — mirrors iOS's ~400-line SwiftUI `Canvas` implementation.
- Liquid Glass (I.5) is lowest priority — iOS feature is iOS 26+ only, Android equivalent needs API 31+.
