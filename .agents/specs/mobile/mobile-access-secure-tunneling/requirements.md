# Requirements Document: Mobile Access via Secure Tunneling

## Introduction

Sim users who work on remote machines (accessed via SSH) currently connect their desktop client through Sim's remote server. However, there is no way for the **Sim mobile app** to establish a secure connection to these remote development environments.

This feature **migrates** the tunnel management functionality (currently in the `goose-server` crate) into Sim, and adds a settings panel where users can start/stop a secure tunnel and display a QR code that the mobile app scans to configure and connect. This enables developers to access their remote workspaces from a mobile device seamlessly, with no external server dependency.

## Glossary

| Term | Definition |
|------|------------|
| **Secure Tunnel** | An encrypted forwarding channel (e.g., SSH reverse/forward tunnel) that exposes a remote workspace to the mobile app through a secure endpoint. |
| **QR Code** | A matrix barcode encoding the connection parameters (host, port, credentials, etc.) that the mobile app scans to configure itself. |
| **Mobile App** | The Sim companion mobile application that scans the QR code to establish a secure connection to the remote environment. |
| **Tunnel Manager** | The Sim-native module (migrated from `goose-server`) responsible for starting, stopping, and monitoring secure tunnels. |
| **Remote Server** | Sim's existing SSH-based remote server infrastructure (`remote_server` crate) that provides remote development capabilities. |

## Requirements

### Requirement 1: Tunnel Management Module (Migrated from goose-server)

**User Story:** As a developer, I want the tunnel start/stop/status logic to live inside Sim, so that mobile access works without depending on an external goose-server.

#### Acceptance Criteria

1.1 The tunnel start, stop, and status query logic currently in `goose-server`'s `/tunnel/start`, `/tunnel/stop`, and `/tunnel/status` SHALL be migrated into Sim as a native module (crate or submodule).

1.2 WHEN the module starts a tunnel THEN it SHALL establish a secure forwarding channel (e.g., SSH reverse tunnel or relay connection) that exposes the local workspace to the mobile app.

1.3 WHEN the module stops a tunnel THEN it SHALL tear down the forwarding channel and clean up any processes or ports.

1.4 THE module SHALL expose a programmatic API (not HTTP) for starting, stopping, and querying tunnel status, callable directly from the settings UI.

1.5 THE tunnel SHALL be configurable with an optional SSH host hint to leverage existing remote server identities.

### Requirement 2: Tunnel Settings Panel

**User Story:** As a Sim user, I want a dedicated settings page where I can start and stop a secure tunnel natively, so that I can expose my workspace to the mobile app on demand.

#### Acceptance Criteria

2.1 WHEN the user opens the Sim settings window THEN THE system SHALL display a "Mobile Access" settings page under an appropriate section (e.g., "Remote Development" or "Server").

2.2 WHEN the user navigates to the Mobile Access settings page THEN THE system SHALL show the current tunnel status (Running / Stopped / Error) and the tunnel endpoint URL (if active).

2.3 WHEN the tunnel is stopped THEN THE system SHALL display a "Start Tunnel" button.

2.4 WHEN the tunnel is running THEN THE system SHALL display a "Stop Tunnel" button.

2.5 WHEN the user clicks "Start Tunnel" THEN THE system SHALL call the native tunnel manager to start the tunnel and display an in-progress indicator while waiting.

2.6 WHEN the user clicks "Stop Tunnel" THEN THE system SHALL call the native tunnel manager to stop the tunnel and display an in-progress indicator while waiting.

2.7 IF the tunnel start operation fails THEN THE system SHALL display a user-visible error message with the failure reason.

2.8 IF the tunnel stop operation fails THEN THE system SHALL display a user-visible error message with the failure reason.

### Requirement 3: QR Code Display

**User Story:** As a mobile app user, I want to scan a QR code to configure my connection, so that I don't need to manually enter host, port, or credentials.

#### Acceptance Criteria

3.1 WHEN the tunnel is running THEN THE system SHALL display a QR code encoding the connection parameters (tunnel endpoint URL, optional authentication token).

3.2 WHEN the user clicks the displayed QR code THEN THE system SHALL provide an option to copy the connection string to the clipboard.

3.3 IF the connection parameters change (e.g., tunnel restart) THEN THE system SHALL regenerate the QR code to reflect the new parameters.

3.4 WHILE the tunnel is starting or stopping THEN THE system SHALL hide or gray out the QR code with a status message.

3.5 WHERE the Mobile Access settings page is displayed THE system SHALL show human-readable connection instructions below the QR code (e.g., "Open the Sim mobile app, tap 'Add Connection', and scan this QR code.").

### Requirement 4: Integration with Existing SSH Remote Server

**User Story:** As a user, I want the mobile connection feature to work with my existing SSH-based remote server, so that I don't need to set up additional infrastructure.

#### Acceptance Criteria

4.1 WHERE the user has an active SSH remote connection in Sim THEN THE system SHALL use the remote server's SSH identity and host information to pre-configure the tunnel parameters.

4.2 IF no remote connection is active THEN THE system SHALL allow the user to configure a tunnel independently by specifying the SSH host and port manually.

4.3 THE system SHALL remember the last-used tunnel configuration across Sim restarts (persist in settings).

### Requirement 5: Tunnel Lifecycle and Cleanup

**User Story:** As a user, I want tunnels to be managed reliably without leaking processes or ports.

#### Acceptance Criteria

5.1 WHEN the tunnel is started THEN THE system SHALL manage the subprocess (if any) and ensure cleanup on stop.

5.2 IF Sim exits while a tunnel is running THEN THE system SHALL attempt to clean up the tunnel on next startup (or the tunnel SHALL self-terminate after a timeout).

5.3 WHEN the user closes the settings window while the tunnel is running THEN THE tunnel SHALL remain running (tunnel lifecycle is independent of the settings panel being open).

### Requirement 6: Visual and UX Consistency

**User Story:** As a Sim user, I want the Mobile Access page to look and behave consistently with the existing settings UI.

#### Acceptance Criteria

6.1 WHEN rendering the Mobile Access page THEN THE system SHALL use the same design language (Tailwind-like styling via GPUI divs, consistent spacing, typography) as other settings pages in `settings_ui`.

6.2 WHEN the tunnel QR code is displayed THEN THE system SHALL render it at a readable size (at least 200x200 px) within the settings content area.

6.3 WHEN there is an error or status change THEN THE system SHALL use standard Sim notification/messaging patterns (inline banner).
