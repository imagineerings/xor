# Design: Telemetry Disabled by Default

## Overview

Use Zed's existing telemetry settings as the only policy owner. Change the embedded and typed defaults to `false`, retain the existing UI and settings write paths as explicit opt-in, and harden the existing client telemetry state so disabling metrics clears pending outbound work before the uploader can construct or send a request.

## Decisions

### D1: Make the canonical settings defaults opt-in

<!-- impl: assets/settings/default.json#telemetry -->
<!-- impl: crates/settings_content/src/settings_content.rs#TelemetrySettingsContent -->

- Choice: Set both fields to `false` in `assets/settings/default.json` and `TelemetrySettingsContent::default`.
- Rationale: The embedded default settings are used across release channels and test/visual settings, while the typed default is used when settings writers materialize a missing telemetry object. Keeping both aligned prevents a UI or onboarding write from silently restoring the old default for the other field.
- Alternatives considered: A build flag or environment variable would create a second configuration mechanism and would not give users a durable explicit opt-in.
- Consequences: Existing explicit user `true` values continue to override the default. Fresh and unset configurations do not send client metrics or diagnostic reports.

### D2: Enforce disabling inside the existing telemetry state

<!-- impl: crates/client/src/telemetry.rs#TelemetryState::update_settings -->
<!-- impl: crates/client/src/telemetry.rs#Telemetry::flush_events_inner -->

- Choice: Centralize settings application in `client::telemetry::Telemetry`, clear queued events, scheduled flush state, timing state, and authenticated metric identity when metrics becomes disabled, and guard `flush_events_inner` before it consumes events or builds the request.
- Rationale: `report_event` already rejects disabled events, but the current settings observer only copies settings. A queued timer can therefore upload after an opt-out. Enforcing the transition and upload guard in the existing owner closes the outbound race without changing callers or instrumentation.
- Alternatives considered: Gating every `telemetry::event!` call would be incomplete and duplicate policy. Replacing the HTTP client or DNS resolver would affect unrelated application traffic.
- Consequences: Events emitted while disabled and events pending at the disable transition are not retained for a later opt-in. Re-enabling metrics resumes collection only for subsequent events.

### D3: Preserve diagnostic and application lifecycle owners

<!-- impl: crates/zed/src/reliability.rs#init -->

- Choice: Keep `zed::reliability` and all normal startup initialization unchanged; rely on its existing `diagnostics_enabled` checks with the new default.
- Rationale: Crash uploads are already conditionally started through the canonical setting. Removing reliability initialization would also remove local hang detection and logging, which are not outbound telemetry and must remain available.
- Consequences: Explicitly enabled diagnostics keep their existing behavior. Other networked product features are unaffected.

### D4: Document the temporary policy at the settings source and telemetry guide

<!-- impl: docs/src/telemetry.md#configuring-telemetry-settings -->

- Choice: Update default-setting comments, generated settings reference text, and `docs/src/telemetry.md` with the temporary default and explicit opt-in keys.
- Rationale: Developers and users need one clear restoration path without code archaeology.

## Failure and recovery

- A disabled client accepts telemetry instrumentation calls as no-ops; callers do not fail and application behavior does not depend on telemetry delivery.
- Disabling metrics drops unsent data and cancels the tracked flush task. A later explicit opt-in starts with an empty queue.
- `flush_events_inner` returns success without building a URL or invoking the HTTP client when metrics is disabled, including direct shutdown or hang-detection flush calls.
- Existing enabled-mode HTTP errors remain logged and isolated from application behavior.

## Traceability

Validates: Requirements 1.1, 1.2, 1.3, 1.4, 2.1, 2.2, 2.3, 2.4, 3.1, 3.2

| Criterion | Design coverage | Verification type | Planned check / expected signal |
| --- | --- | --- | --- |
| 1.1 | D1 | Integration | `cargo test -p client telemetry_disabled_by_default` observes both canonical settings as false |
| 1.2 | D2 | Integration | Disabled event/flush test records zero fake HTTP calls and empty queue state |
| 1.3 | D1, D3 | Static + integration | Default-setting test reports diagnostics false; existing reliability gate remains unchanged |
| 1.4 | D3 | Regression | Focused client tests and affected-crate clippy pass without altering startup owners |
| 2.1 | D1, D2 | Integration | Explicit metric opt-in test sends one subsequent telemetry request |
| 2.2 | D1, D3 | Integration | Settings test proves explicit diagnostics opt-in overrides the default |
| 2.3 | D2 | Integration | Disable-after-queue test clears state and records zero HTTP calls |
| 2.4 | D1-D3 | Static | Diff contains no new configuration owner or telemetry registry |
| 3.1 | D4 | Review | Default settings and telemetry documentation describe temporary disabled default |
| 3.2 | D4 | Review | Documentation names Settings and both existing JSON keys |
