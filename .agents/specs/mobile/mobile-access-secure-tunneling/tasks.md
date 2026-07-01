# Implementation Plan: Mobile Access via Secure Tunneling

## Overview

This plan migrates the tunnel management logic from `goose-server` into Baymax and builds a settings panel with QR code display. The work is ordered to establish the core tunnel manager first (porting the goose-server SSH tunnel logic), then build the settings UI on top, then add QR code generation, and finally wire up persistence and tests. Each step produces a compilable, testable increment.

**Key decisions:**
- The migrated tunnel logic lives in a new `mobile_tunnel` crate within Baymax
- The settings page is added to the existing `settings_ui` crate following its established patterns
- QR code generation uses the `qrcode` crate (to be added as a workspace dependency)
- SSH subprocess management reuses `util::command` patterns already present in `remote` and `repl`

## Tasks

- [x] 1. Scaffold the `mobile_tunnel` crate
  - Create `crates/mobile_tunnel/` with `Cargo.toml` and `src/lib.rs`
  - Add `qrcode` and `image` workspace dependencies in the root `Cargo.toml`
  - Define the public types (`TunnelStatus`, `TunnelInfo`, `TunnelManager`) in `lib.rs`
  - Stub `TunnelManager::new()`, `start()`, `stop()`, `status()` with placeholder implementations
  - Add the crate to the workspace `members` list
  - _Requirements: 1.1_
  - _writes: Cargo.toml, crates/mobile_tunnel/Cargo.toml, crates/mobile_tunnel/src/lib.rs_

- [x] 2. Port SSH tunnel start logic from goose-server (reusing existing Baymax SSH infrastructure)
  - [x] 2.1 Implement `TunnelManager::start()`
    - Port the SSH tunnel creation, port allocation, and retry logic from goose-server's `/tunnel/start` handler
    - Use `std::net::TcpListener::bind("127.0.0.1:0")` for local port allocation (pattern already used in `ssh_kernel.rs`)
    - **If an `SshRemoteConnection` is active**: call `build_forward_ports_command()` on it to create a new SSH session through the existing **ControlMaster socket** — no re-authentication needed
    - **If no active SSH remote connection**: use `util::command::Command` to spawn a standalone SSH subprocess with `-L` flags, using user-provided `SshConnectionOptions` with `port_forwards` set
    - Monitor subprocess stdout/stderr for tunnel readiness (matching the existing SSH kernel pattern)
    - Return `TunnelInfo` on success
    - _Requirements: 1.2, 1.4_
    - _writes: crates/mobile_tunnel/src/tunnel_manager.rs_

  - [x] 2.2 Implement `TunnelManager::stop()`
    - Port the cleanup logic from goose-server's `/tunnel/stop` handler
    - Kill the SSH subprocess (the ControlMaster itself stays alive — only the forward session is torn down)
    - Clean up any allocated ports
    - Transition state to `Stopped`
    - _Requirements: 1.3, 1.4_
    - _writes: crates/mobile_tunnel/src/tunnel_manager.rs_

- [x] 3. Add QR code generation
  - [x] 3.1 Implement QR code module
    - Create `crates/mobile_tunnel/src/qr_code.rs`
    - Use the `qrcode` crate to encode a connection string into a QR code bitmap
    - Render the bitmap as a PNG `Vec<u8>` using the `image` crate
    - Define the connection string format: `baymax-tunnel://{host}:{port}?token={token}`
    - Write unit tests verifying the QR code output is valid PNG and the connection string round-trips
    - _Requirements: 3.1_
    - _writes: crates/mobile_tunnel/src/qr_code.rs, crates/mobile_tunnel/Cargo.toml_

- [x] 4. Register the `mobile_tunnel` crate with Baymax's init
  - [x] 4.1 Initialize `TunnelManager` during Baymax startup
    - Add a global or app-level `TunnelManager` instance in `crates/baymax/src/main.rs` (or in `settings_ui::init`)
    - Wire it to detect an existing SSH remote connection from the `remote` crate — hold a `WeakEntity<RemoteClient>` so the tunnel can access the active `SshRemoteConnection`'s ControlMaster socket
    - _Requirements: 4.1, 4.3_
    - _writes: crates/baymax/src/main.rs_ (or appropriate init location)

- [x] 5. Build the Mobile Access settings page
  - [x] 5.1 Create the page module
    - Create `crates/settings_ui/src/pages/mobile_access_setup.rs`
    - Implement `render_mobile_access_setup_page()` with the full layout: header, status indicator, Start/Stop button, QR code slot, connection instructions
    - Use `TunnelManager` via a global or injected reference to drive state
    - Wire Start button to `tunnel_manager.start()`, Stop button to `tunnel_manager.stop()`
    - Handle all states (`Stopped`, `Starting`, `Running`, `Stopping`, `Error`) with appropriate UI rendering
    - Show error messages as inline banners (matching existing error patterns in settings_ui)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 3.4, 6.1, 6.3_
    - _writes: crates/settings_ui/src/pages/mobile_access_setup.rs_

  - [x] 5.2 Register the page in settings navigation
    - Add the `MobileAccess` SubPageLink to the `network_page()` (or a new section) in `crates/settings_ui/src/page_data.rs`
    - Add the module declaration to `crates/settings_ui/src/pages.rs`
    - _Requirements: 2.1_
    - _writes: crates/settings_ui/src/pages.rs, crates/settings_ui/src/page_data.rs_

  - [x] 5.3 Add QR code rendering to the page
    - Use the QR code generator from Task 3 to produce a PNG
    - Render the PNG as an `img` element in GPUI (use existing image support)
    - Show human-readable instructions below the QR code
    - Add clipboard copy on click
    - _Requirements: 3.1, 3.2, 3.3, 3.5, 5.2, 6.2_
    - _writes: crates/settings_ui/src/pages/mobile_access_setup.rs_

- [x] 6. Add settings persistence
  - [x] 6.1 Define `MobileAccessSettings` in the settings content types
    - Add `mobile_access` field to `SettingsContent` in `crates/settings_content/`
    - Define fields: `ssh_host: Option<String>`, `ssh_port: Option<u16>`
    - Add JSON schema annotations for settings UI field rendering
    - _Requirements: 4.2, 4.3_
    - _writes: crates/settings_content/src/settings_content.rs_

  - [x] 6.2 Wire settings to the Mobile Access page
    - Add a `SettingField` for `ssh_host` and `ssh_port` on the Mobile Access page
    - Load saved settings on page open, pre-fill the SSH host field
    - Save on tunnel start or on field change
    - _Requirements: 4.2, 4.3_
    - _writes: crates/settings_ui/src/pages/mobile_access_setup.rs_

- [x] 7. Add tests
  - [x] 7.1 Unit tests for `TunnelManager`
    - Test state transitions: Stopped→Starting→Running→Stopping→Stopped
    - Test error transitions: Starting→Error (simulate SSH failure)
    - Test double start/stop (concurrent calls are no-ops)
    - _Requirements: 1.4_
    - _writes: crates/mobile_tunnel/src/tunnel_manager.rs_

  - [x] 7.2 Unit tests for QR code generation
    - Test that connection string encodes and decodes correctly
    - Test that QR code output is valid PNG format
    - Test edge cases: empty auth token, long hostnames
    - _Requirements: 3.1_
    - _writes: crates/mobile_tunnel/src/qr_code.rs_

  - [x] 7.3 Settings page rendering tests
    - Test that the page renders correctly in `Stopped`, `Running`, and `Error` states
    - Test that QR code is present only when `Running`
    - Test that Start/Stop buttons have correct enabled state
    - Follow existing `settings_ui` test patterns (use `test` module in `settings_ui.rs`)
    - _Requirements: 2.1, 2.2, 2.3, 2.4, 3.1, 3.4_
    - _writes: crates/settings_ui/src/pages/mobile_access_setup.rs_

## Notes

- Tasks 2.1 and 2.2 depend on Task 1 (crate scaffolding)
- Task 3 (QR code) can proceed in parallel with Tasks 2.x once Task 1 is done
- Task 5 depends on Tasks 2 and 3 (needs `TunnelManager` and QR code generator)
- Task 6 (persistence) can proceed in parallel with Task 5 once Task 1 is done
- Task 7 (tests) is cumulative — write as each component is implemented
