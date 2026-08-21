# Implementation plan: Telemetry Disabled by Default

## Tasks

- [x] 1. Make the existing telemetry settings opt-in by default
  - _id: telemetry-disabled-default-settings_
  - _priority: P1_
  - _value: high_
  - _wave: 1_
  - _Depends on: none_
  - _reads: assets/settings/default.json, crates/settings_content/src/settings_content.rs, crates/client/src/client.rs, crates/zed/src/reliability.rs, crates/onboarding/src/basics_page.rs, crates/settings_ui/src/page_data.rs_
  - _writes: assets/settings/default.json, crates/settings_content/src/settings_content.rs, crates/client/src/telemetry.rs, docs/src/telemetry.md, docs/src/reference/all-settings.md_
  - _validation: cargo test -p client telemetry_disabled_by_default && cargo test -p client telemetry_explicit_opt_in_
  - _Requirements: 1.1, 1.3, 1.4, 2.1, 2.2, 2.4, 3.1, 3.2_
  - Outcome: Fresh configurations disable client metrics and diagnostics while existing settings and onboarding remain the explicit restoration path.
  - Design: D1, D3, D4
  - Done when: Tests observe false defaults and successful explicit opt-in for both categories, documentation identifies the temporary default and re-enable path, and existing reliability owners are unchanged.
  - _Evidence: `cargo test -p client telemetry_disabled_by_default` and `cargo test -p client telemetry_explicit_opt_in` passed on 2026-08-12; embedded settings, typed defaults, settings reference, and telemetry guide now describe the existing opt-in path._

- [x] 2. Prevent disabled telemetry from retaining or uploading metric events
  - _id: telemetry-disabled-outbound-guard_
  - _priority: P1_
  - _value: high_
  - _wave: 2_
  - _blocked_by: telemetry-disabled-default-settings_
  - _Depends on: 1_
  - _reads: crates/client/src/telemetry.rs, crates/http_client/src/http_client.rs_
  - _writes: crates/client/src/telemetry.rs_
  - _validation: cargo test -p client telemetry_disabled && cargo test -p client telemetry_flush && ./script/clippy -p client -p settings -p settings_content_
  - _Requirements: 1.2, 1.4, 2.1, 2.3, 2.4_
  - Outcome: Disabled telemetry performs no `/telemetry/events` HTTP attempt, queued events are discarded on disable, and explicit opt-in retains the existing uploader.
  - Design: D2, D3
  - Done when: Deterministic fake-client tests cover disabled event emission, direct flush, disable-after-queue, and enabled upload; focused regression tests, formatting, clippy, spec validation, and `git diff --check` pass.
  - _Evidence: Disabled emission/direct-flush and disable-after-queue tests recorded zero HTTP requests; explicit opt-in recorded one request; all 34 `client`, 30 `settings`, and 32 non-ignored `settings_content` tests passed, as did `./script/clippy -p client -p settings -p settings_content`, formatting, spec validation, and `git diff --check` on 2026-08-12._

## Completion checks

- Run `cargo fmt --all -- --check`.
- Run affected `client`, `settings`, and `settings_content` tests plus `./script/clippy -p client -p settings -p settings_content`.
- Revalidate this specification and run `git diff --check`.
- Do not commit or push.
