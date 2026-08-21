# Requirements: Telemetry Disabled by Default

## Problem

Zed currently enables client-side usage metrics and diagnostic uploads by default. Metric events are eventually posted to `api.zed.dev/telemetry/events`, which causes unnecessary outbound DNS and HTTP activity when telemetry has not been explicitly enabled and produces noisy startup failures in offline environments.

Zed already exposes `telemetry.metrics` and `telemetry.diagnostics` settings. This change makes those existing controls opt-in by default and closes the queued-event race without removing instrumentation or creating another configuration system.

## Scope

### In scope

- Default client-side usage metrics and diagnostic uploads to disabled in every build.
- Preserve the existing settings UI, onboarding toggles, and JSON settings as explicit opt-in paths.
- Prevent disabled usage metrics from queuing or uploading events, including events queued before the setting is turned off.
- Document the temporary default and re-enable location.

### Out of scope

- Removing telemetry events, event schemas, logs, crash-reporting code, or the telemetry client.
- Disabling functional network traffic for updates, authentication, hosted AI, collaboration, or other user-invoked services.
- Changing server-side accounting or operational metadata required by hosted services.
- Adding an environment variable, feature flag, build-time fork, or second consent system.

## Requirements

### Requirement 1: Disabled client-side telemetry default

**System outcome:** Zed starts without outbound client-side telemetry unless the user has explicitly enabled it.

#### Acceptance criteria

1. **1.1** WHEN Zed loads default settings in any release channel or build profile, THEN THE system SHALL set `telemetry.metrics` and `telemetry.diagnostics` to `false`.
2. **1.2** WHILE `telemetry.metrics` is disabled, THE telemetry client SHALL discard metric events without scheduling a flush, constructing a telemetry request, performing DNS resolution, or sending HTTP to `api.zed.dev/telemetry/events`.
3. **1.3** WHILE `telemetry.diagnostics` is disabled, THE existing reliability startup path SHALL not upload local or remote crash reports.
4. **1.4** WHEN telemetry is disabled, THEN THE rest of startup, hang detection, local logging, settings, updates, hosted services, and normal application behavior SHALL remain available.

### Requirement 2: Existing explicit opt-in and disable transitions

**User story:** As a Zed user, I want the existing telemetry settings to remain authoritative, so that I can explicitly enable or disable each telemetry category.

#### Acceptance criteria

1. **2.1** WHEN a user explicitly sets `telemetry.metrics` to `true` through existing settings or onboarding, THEN THE existing metric queue and uploader SHALL resume for subsequent events.
2. **2.2** WHEN a user explicitly sets `telemetry.diagnostics` to `true`, THEN THE existing diagnostic setting SHALL remain enabled for reliability paths that consult it.
3. **2.3** WHEN `telemetry.metrics` changes from enabled to disabled with queued events or a scheduled flush, THEN THE telemetry client SHALL cancel the scheduled flush, discard unsent events and user telemetry identity, and make no telemetry request for those events.
4. **2.4** THE implementation SHALL reuse `TelemetrySettings`, `TelemetrySettingsContent`, `SettingsStore`, and `client::telemetry::Telemetry` without introducing another gate or configuration source.

### Requirement 3: Temporary-default documentation

#### Acceptance criteria

1. **3.1** THE default settings and telemetry documentation SHALL state that client-side metrics and diagnostics are temporarily disabled by default.
2. **3.2** THE documentation SHALL identify Settings and the `telemetry.metrics` and `telemetry.diagnostics` JSON keys as the place to re-enable telemetry explicitly.

## Constraints

- The absence of a user setting is not consent; both client-side telemetry categories default to disabled.
- Previously explicit `true` values remain valid opt-ins.
- The metric uploader must check the authoritative runtime setting before consuming queued events or building the `/telemetry/events` request.
