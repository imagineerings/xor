# Design: User Experience

## 1. Overview

UX features enhance the app's polish and personalization. The architecture follows a modular pattern where each feature (profile, status, theme, onboarding) is independent but shares common UI patterns.

### Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Theme persistence | UserDefaults / DataStore | Simple key-value, synced with system setting |
| Profile source | Agent API (status endpoint) + collab server | Profile synced from connected server |
| Onboarding | Conditional — shown only on first launch (no saved config) | Standard mobile UX pattern |

## 2. Tasks

- [ ] 1. User profile view and edit screen
- [ ] 2. Custom status picker with emoji and auto-clear
- [ ] 3. Theme manager (light/dark/system, accent color, font size, clock format)
- [ ] 4. Onboarding flow (welcome, trial, configure, scan QR)
- [ ] 5. Tutorial highlights on first use
