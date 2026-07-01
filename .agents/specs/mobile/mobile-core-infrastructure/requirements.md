# Requirements: Core Infrastructure & Connectivity

## Introduction

The Baymax mobile client (Swift iOS / Jetpack Compose Android) needs a robust infrastructure layer that handles server connections, tunnel management, real-time communication, session lifecycle, and authentication. This foundation enables all higher-level features (chat, collaboration, calls, etc.) to function reliably across both platforms.

Currently, both iOS and Android have basic implementations of some infrastructure (REST API service, basic auth via secret key, simple session fetch, QR-based configuration). This spec formalizes, consolidates, and extends those into a **shared cross-platform infrastructure design** that also connects to the Baymax desktop editor's existing remote and tunnel systems (`mobile_tunnel`, `remote`, `rpc` crates).

## Glossary

| Term | Definition |
|------|------------|
| **Baymax Agent** | The AI coding assistant running locally or remotely. Exposes an HTTP/SSE API for chat and tool calls. |
| **Baymax Collab Server** | The multi-user collaboration server providing channels, calls, project sharing, and presence. Connected via gRPC/RPC. |
| **Tunnel** | A secure network forwarding channel (SSH, Tailscale, or Cloudflare) that exposes a remote Baymax agent behind NAT/firewall to the mobile app. |
| **SSE (Server-Sent Events)** | Unidirectional streaming protocol used by the Baymax agent to stream AI responses in real-time. |
| **WebSocket** | Bidirectional streaming protocol used for real-time collab features (presence, location sharing, etc.). |
| **Session** | A single agent conversation, identified by a UUID. Contains a sequence of messages (user messages + assistant responses + tool calls). Session history is stored on the agent server. |
| **Auth Token** | A secret key used to authenticate the mobile app to the Baymax agent. Also called "secret key" in the existing codebase. |
| **Enhanced Auth** | Context-aware authentication that tracks connection provenance (tunnel, tailscale, direct LAN, collab server) and may use different credential types for each. |
| **Connection Provenance** | The network path used to reach the server: direct LAN, Tailscale tunnel, SSH tunnel, Cloudflare tunnel, or collab server relay. |
| **Trial Mode** | A demo mode connecting to a hosted instance at `demo-baymaxed.fly.dev` with limited functionality. |

## Requirements

### Requirement 1: Server Connection

**User Story:** As a mobile user, I want the app to establish and maintain a connection to my Baymax agent or collab server, so that I can interact with my agent and collaborators.

#### Acceptance Criteria

1.1 THE app SHALL support connecting to a Baymax agent via a configured base URL and secret key.

1.2 WHEN the app starts with a saved configuration THEN it SHALL automatically attempt to connect without user intervention.

1.3 WHEN the user enters a new server URL and secret key THEN THE app SHALL test the connection by calling the agent's `/status` endpoint.

1.4 IF the connection test succeeds THEN THE app SHALL save the configuration and transition to the connected state.

1.5 IF the connection test fails THEN THE app SHALL display a user-visible error message with the failure reason.

1.6 THE app SHALL maintain a `isConnected` boolean state and `connectionError` string observable by the UI layer.

1.7 WHEN the connection is lost during normal operation THEN THE app SHALL display a connection banner (similar to `ConnectionBanner` in mobile-dev).

1.8 IF the connection is lost THEN THE app SHALL automatically retry with exponential backoff (1s, 2s, 4s, 8s, max 30s).

1.9 WHEN the user switches between saved agent configurations THEN THE app SHALL disconnect from the current agent and connect to the new one.

1.10 THE app SHALL track and display **connection provenance** — whether the connection is via Direct LAN, Tailscale, SSH Tunnel, Cloudflare Tunnel, or Trial Mode.

1.11 WHERE the app detects a Tailscale URL (100.x.x.x or *.ts.net) AND the connection fails THEN THE app SHALL display a Tailscale-specific error with an option to open the Tailscale app.

1.12 WHERE the app detects a Cloudflare tunnel URL AND the connection fails THEN THE app SHALL display a tunnel-specific error message.

1.13 THE app SHALL support a **Trial Mode** that defaults to `https://demo-baymaxed.fly.dev` with secret key `test` when no configuration is saved.

1.14 WHEN the app successfully connects to an agent THEN THE app SHALL display the assigned **server version** in the settings UI.

### Requirement 2: Tunnel Management

**User Story:** As a developer using Baymax on a remote machine, I want the mobile app to recognize and manage tunnel-based connections so I can access my agent through NAT/firewall.

#### Acceptance Criteria

2.1 THE app SHALL detect the **tunnel type** from the server URL:
   - `TAILSCALE`: URL contains 100.x.x.x or .ts.net domain
   - `CLOUDFLARE`: URL contains `cloudflare-tunnel-proxy`, `.trycloudflare.com`, or `cf-tunnel`
   - `SSH`: Connection established via the desktop's `TunnelManager` (Rust crate)
   - `NONE`: Direct LAN or public URL

2.2 WHEN the app is launched with a Tailscale tunnel URL AND the connection fails THEN THE app SHALL attempt to open the Tailscale app (iOS: `tailscale://` deep link, Android: `tailscale://` intent with Play Store fallback).

2.3 WHEN connecting through a Tailscale tunnel AND the connection succeeds THEN THE app SHALL display "Tailscale" as the connection method in the UI.

2.4 WHERE the desktop Baymax editor has an active `TunnelManager` (SSH tunnel) THEN the mobile app SHALL be able to connect via the SSH tunnel endpoint.

2.5 WHEN an SSH tunnel endpoint is provided via QR code configuration THEN THE app SHALL connect using the tunnel's local port and auth token.

2.6 THE mobile app SHALL NOT manage tunnel lifecycle (start/stop) itself — tunnel start/stop is the desktop's responsibility via the `mobile_tunnel` crate's `TunnelManager`.

2.7 WHERE the desktop Baymax exposes a QR code for a running tunnel THEN THE mobile app SHALL scan and configure itself automatically.

2.8 THE app SHALL handle tunnel connection errors gracefully, distinguishing between:
   - Tunnel not running (connection refused)
   - Tunnel unreachable (network timeout)
   - Tunnel authentication failure (invalid auth token)

2.9 WHEN the desktop tunnel stops OR the auth token expires THEN THE app SHALL detect the disconnection and transition to a disconnected state with a clear message.

### Requirement 3: Real-Time Communication (SSE / WebSocket)

**User Story:** As a mobile user, I want to receive real-time streaming responses from my agent and real-time updates from collaborators, so the app feels responsive and live.

#### Acceptance Criteria

3.1 THE app SHALL use **Server-Sent Events (SSE)** for streaming agent responses, reusing the existing agent API endpoint (`/reply`).

3.2 WHEN a user message is sent THEN THE app SHALL open an SSE connection to stream the agent's response tokens in real-time.

3.3 WHILE the SSE stream is active THE app SHALL incrementally render the assistant's response (token by token), supporting markdown, code blocks, and tool calls.

3.4 WHEN the SSE stream emits a `toolRequest` event THEN THE app SHALL show a tool call card with the tool name, arguments, and status (running/completed/failed).

3.5 WHEN the SSE stream completes (end-of-stream event) THEN THE app SHALL finalize the message and update the session history.

3.6 IF the SSE connection is interrupted mid-stream THEN THE app SHALL attempt to reconnect and resume from the last received token.

3.7 WHEN the agent connection is via a tunnel (Tailscale/SSH/Cloudflare) THEN THE SSE connection SHALL be established through the same tunnel endpoint.

3.8 The app SHALL use **WebSocket** (or SSE) for real-time collaboration features:
   - **Presence**: When a collaborator comes online/goes offline
   - **Channel Messages**: New messages in joined collaboration channels
   - **Notifications**: Incoming call notifications, project share invitations

3.9 WHERE the agent server does not support WebSocket collab features (e.g., in Trial Mode) THEN THE app SHALL degrade gracefully and hide collab features.

3.10 THE app SHALL implement a **connection heartbeat** (Ping/Pong) to detect stale connections within 30 seconds.

3.11 WHEN the heartbeat fails THEN THE app SHALL:
    - Mark the connection as lost
    - Show a connection banner in the UI
    - Begin reconnect with exponential backoff
    - Pause any active SSE streams
    - Re-establish SSE or WebSocket on reconnection

### Requirement 4: Session Management

**User Story:** As a mobile user, I want to view, create, and resume chat sessions with my Baymax agent, so I can continue conversations across time and devices.

#### Acceptance Criteria

4.1 THE app SHALL fetch the list of sessions from the agent API, ordered by most recent first.

4.2 WHEN the user opens the app THEN THE app SHALL display the session list as the home screen, showing:
   - Session name (first user message or auto-generated title)
   - Timestamp (relative: "2m ago", "1h ago", "Yesterday", date)
   - A preview of the last message (user or assistant)

4.3 THE app SHALL support **pagination** of sessions, loading sessions from the last N days initially and loading more on demand.

4.4 WHEN the user taps a session in the list THEN THE app SHALL load that session's full message history from the API and display it.

4.5 WHEN the user is viewing a session AND receives new messages (via polling or SSE) THEN THE app SHALL append them to the session view.

4.6 THE app SHALL support creating a **new session** by using a dedicated "New Session" button or input in the sidebar/home screen.

4.7 WHEN the user sends the first message in a new session THEN THE app SHALL:
    - POST the message to the agent API
    - Start an SSE stream for the response
    - On first assistant response, the session SHALL appear in the session list

4.8 THE app SHALL track **favorite sessions** (starred/bookmarked), persisted locally, and show them at the top of the session list.

4.9 WHERE the app has been offline AND the user returns THEN THE app SHALL re-fetch the session list to pick up any sessions created from other devices.

4.10 THE app SHALL support **per-session renaming** (setting a custom display name) via a long-press or context menu action.

4.11 THE app SHALL support **deleting sessions** via the agent API with confirmation UI.

4.12 WHEN the user is in an active session AND opens another session, the app SHALL preserve the scroll state of the first session (cached in memory for the current app session).

### Requirement 5: Authentication & Credential Management

**User Story:** As a mobile user, I want my agent connection credentials to be stored securely and managed easily, so I can switch between agents safely.

#### Acceptance Criteria

5.1 THE app SHALL store the agent's base URL and secret key in the platform's secure storage:
   - iOS: Keychain
   - Android: EncryptedSharedPreferences

5.2 THE app SHALL support **multiple saved agent configurations**, each with:
   - `id` (UUID)
   - `name` (optional custom name)
   - `url` (server base URL)
   - `secret` (secret key)
   - `lastUsed` (timestamp)
   - `provenance` (auto-detected tunnel type: tailscale/cloudflare/ssh/direct/trial)

5.3 WHEN a new agent configuration is added (via manual entry or QR scan) THEN THE app SHALL:
    - Save it to the agent list
    - Auto-detect and store the connection provenance
    - Generate a default name based on URL pattern (e.g., "Trial", "Desktop")
    - Test the connection
    - Switch to the new configuration on success

5.4 WHEN the user opens the agent selector THEN THE app SHALL display the list of saved agents sorted by `lastUsed` descending, with the current agent highlighted.

5.5 WHEN the user taps a saved agent THEN THE app SHALL switch to that configuration and test the connection.

5.6 WHEN the user long-presses or swipes on a saved agent THEN THE app SHALL offer option to:
    - Rename
    - Edit (URL or secret)
    - Delete

5.7 THE app SHALL support **QR code configuration** that deep-links into the app:
    - URL scheme: `baymaxchat://configure?data=<url-encoded-json>`
    - JSON format: `{"url": "https://...", "secret": "..."}`
    - On receipt, parse, validate, test connection, and save as new agent

5.8 IF the QR code results in a connection error THEN THE app SHALL show the error and NOT save the configuration.

5.9 THE app SHALL support **biometric authentication** (Face ID / Fingerprint) to unlock the app:
    - WHEN enabled, the app SHALL require biometric verification before showing any UI
    - IF biometrics are not available on the device THEN the option SHALL be hidden
    - THE biometric lock SHALL use a grace period of 5 minutes after the app goes to background before re-locking

5.10 WHERE the app connects to a **Baymax Collab Server** (not just an agent) THEN authentication SHALL extend to include collab server credentials (OAuth token, session cookie) in addition to the agent secret key.

5.11 WHILE in Trial Mode THE app SHALL limit functionality:
    - One session per device
    - Limited tools
    - No collaboration features
    - Show a banner indicating trial mode

## Existing Assets Inventory

The following implementations already exist and must be reused (not rebuilt):

| Platform | Feature | File(s) |
|----------|---------|---------|
| iOS | Agent API Service | `BaymaxAPIService.swift` — HTTP client, retry, connection testing |
| iOS | Agent Storage | `ConfigurationHandler.swift` — Multiple agents, switch, save, delete |
| iOS | QR Config | `ConfigurationHandler.swift` — Parse `baymaxchat://configure` URL |
| iOS | Tunnel Detection | `ConfigurationHandler.swift` — Tailscale URL detection, Tailscale app deep link |
| iOS | Session Fetch | `ContentView.swift` — Session list with pagination |
| iOS | Settings UI | `SettingsView.swift` — Server URL/secret, connection test, agent list |
| iOS | Trial Mode | `TrialMode.swift`, `TrialModeBanner.swift` — Trial detection and banner |
| Android | API Service | `BaymaxApiService.kt` — OkHttp client, retry, SSE streaming |
| Android | Settings/Storage | `SettingsRepository.kt`, `SettingsScreen.kt`, `SettingsViewModel.kt` |
| Android | Agent Config | `AgentConfiguration.kt`, `AgentRepository.kt` |
| Android | QR Config | `QRConfigHandler.kt` — QR scan and URL handling |
| Android | Tunnel Detection | `TunnelDetector.kt`, `TunnelType.kt` — Pattern-matched tunnel detection |
| Android | Trial Mode | `TrialModeManager.kt`, `TrialModeInstructionsScreen.kt` |
| Android | Session Polling | `SessionPoller.kt` — Polling for session updates |
| Rust (desktop) | Tunnel Manager | `mobile_tunnel/crates/tunnel_manager.rs` — SSH tunnel lifecycle |
| Rust (desktop) | QR Generation | `mobile_tunnel/crates/qr_code.rs` — QR code PNG generation |
| Rust (desktop) | Global Tunnel State | `mobile_tunnel/crates/lib.rs` — GlobalTunnelManager for settings UI |
| Go (desktop) | Tailscale Tunnel | `baymax-tunnel/main.go` — Tailscale-based tunnel service |
