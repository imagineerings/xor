# Sim Mobile App — Master Plan

## Purpose

Define the complete mobile experience for Sim by consolidating all mobile feature specifications into a single umbrella. This plan covers **iOS (Swift/SwiftUI)** and **Android (Kotlin/Jetpack Compose)** clients, plus the desktop-side infrastructure needed to support them. Each subspec follows the EARS requirements → Design → Tasks workflow.

## Scope

The Sim mobile app provides AI agent interaction, collaboration, voice/calls, file management, notifications, and more — on both iOS and Android. The desktop companion features (tunnel management, QR code generation, push proxy) are included where they directly enable mobile functionality.

---

## Spec Inventory

### 1. Core Infrastructure & Connectivity

**Summary**: Unified connection state machine, SSE streaming, WebSocket, authentication, session lifecycle, and cross-platform networking foundation.

| Feature | iOS | Android | Desktop |
|---------|-----|---------|---------|
| Server connection (HTTP/SSE) | Existing REST service | Existing REST service | — |
| Connection state machine | New | New | — |
| Connection provenance detection | New | Existing (partial) | — |
| WebSocket for collab | New | New | — |
| Auth token / enhanced auth | Existing (basic) | Existing (basic) | — |
| Session lifecycle | Existing | Existing | — |
| Tunnel connectivity | — | — | TunnelManager crate |
| QR code connection | Existing | Existing | — |

**Location**: `mobile-core-infrastructure/`

---

### 2. Agent Chat Interface

**Summary**: Rich chat experience with streaming responses, markdown/code rendering, tool call cards, message actions, threading, and search.

| Feature | iOS | Android | Notes |
|---------|-----|---------|-------|
| Message display & streaming | Existing `ChatView.swift` | Existing `ChatScreen.kt` | Extend with tool call cards |
| Markdown & syntax highlighting | Existing (partial) | Existing `MarkdownText.kt` | Full spec coverage |
| Tool call display | New card component | New card component | Collapsible, status states |
| Message actions (copy, retry) | New | New | — |
| Slash commands | New | New | Autocomplete from tool list |
| Threading | New | New | Branch from message |
| Search | New | New | Client-side filter |
| Session management | Existing | Existing | — |

**Location**: `mobile-agent-chat/`

---

### 3. Calls & Voice

**Summary**: Voice input/output for the AI agent (STT/TTS) and real-time audio/video calls with collaborators via LiveKit.

| Feature | iOS | Android | Desktop |
|---------|-----|---------|---------|
| Voice input (STT) | Existing `VoiceInputManager` | Existing | — |
| Voice output (TTS) | Existing `VoiceOutputManager` | Existing | — |
| Voice modes (Normal/Transcribe/Continuous) | Existing `ContinuousVoiceManager` | Partial | — |
| Audio/video calls | New (LiveKit) | New (LiveKit) | Existing `call` crate |
| In-call UI | New | New | — |
| Screen sharing (view) | New | New | — |

**Location**: `mobile-calls-voice/`

---

### 4. Collaboration

**Summary**: Connect to the Sim Collab Server for channels, chat, contacts, shared documents, project sharing, and agent thread sharing.

| Feature | iOS | Android | Notes |
|---------|-----|---------|-------|
| Channel browser | New | New | Favorites, categories, DMs |
| Channel chat | New | New | Real-time via WebSocket |
| Contacts & presence | New | New | — |
| Shared document viewing | New | New | Read-only on mobile |
| Project sharing | New | New | — |
| Agent thread sharing | New | New | — |

**Location**: `mobile-collaboration/`

---

### 5. Files & Media

**Summary**: File attachments, document viewing, code visualization, and media handling for agent context.

| Feature | iOS | Android | Notes |
|---------|-----|---------|-------|
| File attachments | New | Existing (partial) | Document picker, upload |
| Image/photo picker | New | New | Camera & library |
| PDF viewer | New | New | Platform-native |
| Code viewer with syntax highlighting | Existing | Existing `SyntaxHighlighter.kt` | — |
| Upload progress & cancellation | New | New | — |

**Location**: `mobile-files-media/`

---

### 6. Integrations & Extensibility

**Summary**: Tool browser, slash command autocomplete, context server management, and external service integrations.

| Feature | iOS | Android | Notes |
|---------|-----|---------|-------|
| Tool browser | New | New | Fetch from agent API |
| Slash command autocomplete | New | New | Derived from tools |
| Tool invocation UI | New | New | Parameter forms |
| Context server management | New | New | MCP server config |
| External integrations | New | New | Webhooks/OAuth |

**Location**: `mobile-integrations-extensibility/`

---

### 7. Notifications

**Summary**: Push notifications (APNs/FCM) and in-app notifications (toasts, badges) for agent events, messages, and calls.

| Feature | iOS | Android | Desktop |
|---------|-----|---------|---------|
| Push notifications (APNs) | New | — | Push proxy service |
| Push notifications (FCM) | — | New | Push proxy service |
| In-app toasts/banners | New | New | — |
| Notification preferences | New | New | — |
| Badge count | New | New | — |

**Location**: `mobile-notifications/`

---

### 8. Settings & Configuration

**Summary**: Server configuration, multi-agent management, display customization, biometric lock, notification settings, and about/licensing.

| Feature | iOS | Android | Notes |
|---------|-----|---------|-------|
| Server config (URL + secret) | Existing `SettingsView.swift` | Existing `SettingsScreen.kt` | Extend |
| Multi-agent management | Existing | Existing (from parity) | — |
| Display settings (theme, font) | New | New | — |
| Biometric lock | New | New | — |
| Notification settings | New | New | — |
| About / licenses | New | New | — |

**Location**: `mobile-settings-config/`

---

### 9. User Experience

**Summary**: User profile, custom status, theming, CRT mode, and onboarding flow.

| Feature | iOS | Android | Notes |
|---------|-----|---------|-------|
| User profile | New | New | Avatar, display name |
| Custom status with emoji | New | New | Auto-clear duration |
| Theme system | New | New | Light/dark, accent colors |
| CRT mode | New | New | Scanlines, glow |
| Onboarding flow | New (partial) | New (partial) | First-run tutorial |

**Location**: `mobile-user-experience/`

---

### 10. Mobile Access via Secure Tunneling (Desktop)

**Summary**: Tunnel management (port from goose-server) and QR code UI in the desktop settings for mobile app connectivity.

| Feature | Desktop | Notes |
|---------|---------|-------|
| TunnelManager (start/stop/status) | New crate (`mobile_tunnel`) | Ported from goose-server |
| SSH tunnel process | New | Reuses existing SSH infrastructure |
| QR code generation | New | — |
| Mobile Access settings page | New (`settings_ui`) | — |
| Tunnel config persistence | New | Existing settings.json |

**Location**: `mobile-access-secure-tunneling/`

---

### 11. Android Feature Parity

**Summary**: Bring Android to feature parity with iOS across all areas. Documents completed phases (1–10) and remaining gaps (11–15).

| Area | Status | Notes |
|------|--------|-------|
| Voice input & output | ✅ Complete | Phases 1 |
| Agent configuration & storage | ✅ Complete | Phase 2 |
| QR code / deep link config | ✅ Complete | Phase 3 |
| Syntax highlighting | ✅ Complete | Phase 4 |
| Connection management | ✅ Complete | Phase 5 |
| Agent chat UI | ✅ Complete | Phase 6 |
| Tool calls display | ✅ Complete | Phase 7 |
| SSE streaming improvements | ✅ Complete | Phase 8 |
| Markdown improvements | ✅ Complete | Phase 9 |
| Performance & stability | ✅ Complete | Phase 10 |
| Remaining gaps (Phases 11–15) | ❌ Remaining | — |

**Location**: `android-feature-parity/`

---

### 12. Mobile Build and Publish

**Summary**: Build, sign, validate, and publish Android and iOS mobile apps for tester distribution and real-device validation.

| Feature | Android | iOS | Notes |
|---------|---------|-----|-------|
| Local build/test scripts | New | New | Shared entry points under `mobile/scripts/` |
| Signed release artifacts | New AAB/APK | New IPA | Signing material supplied via env/CI secrets |
| Root CI workflows | New | New | Active workflows under `.github/workflows/` |
| Tester publishing | Play internal track | TestFlight | Optional publish mode |
| Feature readiness validation | New | New | Derived from mobile specs |

**Location**: `mobile-build-publish/`

---

## Summary

| Spec | Area | Platform | Status |
|------|------|----------|--------|
| 1. Core Infrastructure & Connectivity | Networking, auth, sessions | iOS + Android | Design complete |
| 2. Agent Chat Interface | Chat UI, streaming, markdown | iOS + Android | Design complete |
| 3. Calls & Voice | STT, TTS, WebRTC calls | iOS + Android + Desktop | Design complete |
| 4. Collaboration | Channels, contacts, sharing | iOS + Android | Design complete |
| 5. Files & Media | Attachments, viewers, code | iOS + Android | Design complete |
| 6. Integrations & Extensibility | Tools, slash commands, MCP | iOS + Android | Design complete |
| 7. Notifications | Push, in-app, badges | iOS + Android + Desktop | Design complete |
| 8. Settings & Configuration | Server, display, security | iOS + Android | Design complete |
| 9. User Experience | Profile, themes, onboarding | iOS + Android | Design complete |
| 10. Secure Tunneling | Desktop tunnel → mobile | Desktop (mobile enabler) | Design complete |
| 11. Android Feature Parity | Android ↔ iOS parity | Android | ✅ Phases 1–10 complete, ❌ 11–15 |
| 12. Mobile Build and Publish | Build, sign, validate, publish | iOS + Android + CI | Design complete |

## Related Specs

- **Goose → Sim Migration** (`../goose-migration/`) — Desktop-side features that also benefit mobile (e.g., gateway, dictation, ACP tools)
- **Collab Enhancement** (`../collab-enhancement/`) — Shared collaboration features used by mobile collaboration spec
- **Spectrum 2 Theme & UI Language** (`../spectrum-2-theme/`, `../spectrum-2-ui-language/`) — Design system used by all mobile UI

## Workflow

Each subspec follows this lifecycle:
1. **Requirements** (`requirements.md`) — EARS-based user stories, acceptance criteria, glossary
2. **Design** (`design.md`) — Architecture decisions, component diagrams, data flow, interfaces
3. **Tasks** (`tasks.md`) — Concrete implementation tasks with file paths and dependencies
