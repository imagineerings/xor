# Baymax Mobile

This directory contains the Android and iOS clients for connecting to Baymax agents over the mobile HTTP/SSE protocol. Both apps support local or remote agent URLs, QR/deep-link configuration, session listing, chat, settings, trial mode, and readiness checks for tunneling-enabled mobile access.

## Local Build Entry Points

Run commands from the repository root.

```bash
mobile/scripts/tests/run.sh
mobile/scripts/mobile-readiness-check.sh
mobile/scripts/android-test.sh
mobile/scripts/android-build.sh --variant debug --artifact apk --version 1.0.0 --build-number 1
mobile/scripts/android-build.sh --variant release --artifact aab --signed false --version 1.0.0 --build-number 1
mobile/scripts/ios-build.sh --configuration Debug --version 1.0.0 --build-number 1
mobile/scripts/ios-archive.sh --signed false --export false --version 1.0.0 --build-number 1
```

Artifacts are written under `mobile/build/`:

- Android APK/AAB: `mobile/build/android/`
- iOS archives/exports: `mobile/build/ios/`
- Release metadata: `mobile/build/release-metadata/`
- Readiness manual summary: `mobile/build/readiness/manual-summary.md`

## GitHub Workflows

- `.github/workflows/mobile_android_ci.yml` validates shared scripts, mobile readiness, Android tests, and an Android debug APK.
- `.github/workflows/mobile_ios_ci.yml` validates shared scripts, mobile readiness, and an iOS debug build.
- `.github/workflows/mobile_release.yml` is manual. It accepts `platform`, `channel`, `version`, `build_number`, and `publish` inputs, builds platform artifacts, writes release metadata, and uploads the readiness summary.

`publish=false` is artifact-only mode. For Android, artifact-only mode also uploads a downloadable installable APK named `baymax-android-apk-<version>-<build>-<sha>` in the workflow run artifacts. Use `platform=android`, `channel=artifact`, and `publish=false` when you want an APK that can be downloaded from GitHub Actions.

`publish=true` is intentionally gated:

- Android publishing requires `platform=android` and `channel=play-internal`.
- iOS publishing requires `platform=ios` and `channel=testflight`.
- `platform=all` with `publish=true` is rejected because each store needs separate channel semantics.

## CI Secrets

Android signed builds and Play internal publishing require:

- `ANDROID_KEYSTORE_BASE64`
- `ANDROID_KEYSTORE_PASSWORD`
- `ANDROID_KEY_ALIAS`
- `ANDROID_KEY_PASSWORD`
- `ANDROID_PLAY_SERVICE_ACCOUNT_JSON_BASE64`

Optional Android repository variable:

- `ANDROID_PACKAGE_NAME` defaults to `com.simtropolis.baymaxchat`.

iOS signed archive/export and TestFlight publishing require:

- `IOS_TEAM_ID`
- `IOS_SIGNING_CERTIFICATE_BASE64`
- `IOS_SIGNING_CERTIFICATE_PASSWORD`
- `IOS_PROVISIONING_PROFILE_BASE64`
- `IOS_APP_STORE_CONNECT_KEY_ID`
- `IOS_APP_STORE_CONNECT_ISSUER_ID`
- `IOS_APP_STORE_CONNECT_API_KEY_BASE64`

## Readiness Validation

`mobile/feature-readiness.yml` tracks mobile feature readiness across Android, iOS, and desktop-enabling work. It includes connection configuration, session listing, chat, SSE streaming, settings, trial mode, voice, parity, build/publish, and remote tunneling.

`mobile/scripts/mobile-readiness-check.sh` validates checklist schema, duplicate IDs, spec references, deep-link declarations, QR/configuration handlers, build/publish scripts, and workflow YAML. Manual items are summarized in `mobile/build/readiness/manual-summary.md`.

## Known Blockers

- Store publishing paths are wired but not verified without real signing and store credentials.
- iOS unsigned archives build locally, but App Store/TestFlight polish still needs launch screen/orientation cleanup before submission.
- Tunneling readiness is tracked as a feature-readiness item; end-to-end remote tunnel validation still needs device testing against the desktop tunnel flow.
