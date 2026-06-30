# Implementation Plan: Mobile Build and Publish

## Overview

This plan turns mobile build and publishing into active, reproducible repository behavior. The order starts with shared script entry points and metadata generation, then makes Android release artifacts work, then restores iOS build/archive capability, then adds active root GitHub workflows, publishing gates, feature readiness validation, and developer documentation.

The first useful milestone is artifact-only Android release builds. iOS publishing depends on adding or restoring Xcode project metadata under `mobile/ios`.

## Tasks

- [x] 1. Add shared mobile build script foundation
  - Create `mobile/scripts/` with shell scripts for version validation, metadata generation, and common logging/error helpers
  - Implement validation for platform, channel, version, and build number inputs
  - Implement release metadata JSON generation with platform, channel, version, build number, commit SHA, artifacts, publish status, and timestamp
  - Add script-level tests for version/build-number validation and metadata generation where practical
  - _Requirements: 3.3, 3.4, 5.1, 5.3, 5.4, 5.5, 7.1_
  - _writes: `mobile/scripts/common.sh`, `mobile/scripts/validate-version.sh`, `mobile/scripts/write-release-metadata.sh`, `mobile/scripts/tests/`_

- [x] 2. Implement Android local build and test entry points
  - [x] 2.1 Add Android test and debug build scripts
    - Create `mobile/scripts/android-test.sh` to run Gradle unit tests from `mobile/android`
    - Create `mobile/scripts/android-build.sh` with debug build support and predictable artifact output under `mobile/build/android/`
    - Ensure scripts fail with clear messages when JDK, Android SDK, or Gradle wrapper execution fails
    - _Requirements: 1.1, 1.5, 3.4, 7.1, 7.4_
    - _writes: `mobile/scripts/android-test.sh`, `mobile/scripts/android-build.sh`_

  - [x] 2.2 Wire Android version inputs into Gradle
    - Update `mobile/android/app/build.gradle.kts` to read version name and version code from Gradle properties or environment-backed script inputs
    - Preserve current defaults for local debug builds
    - Add validation in build scripts before passing version values to Gradle
    - _Requirements: 1.4, 5.2, 5.3, 5.4_
    - _writes: `mobile/android/app/build.gradle.kts`, `mobile/scripts/android-build.sh`_

  - [x] 2.3 Add Android release signing and AAB output
    - Configure Android release signing from environment-provided keystore material
    - Decode keystore material into a temporary file outside source-controlled paths
    - Produce signed AAB when signing material is present and requested
    - Produce explicitly named unsigned artifacts only in artifact-only unsigned mode
    - Ensure secret values are never printed
    - _Requirements: 1.2, 1.3, 4.4, 4.5_
    - _writes: `mobile/android/app/build.gradle.kts`, `mobile/scripts/android-build.sh`_

- [x] 3. Restore iOS project buildability
  - [x] 3.1 Add or restore iOS Xcode project metadata
    - Add an Xcode project or workspace under `mobile/ios` that includes existing Swift sources in `mobile/ios/Baymax`
    - Configure bundle identifier, marketing version, build number, deployment target, app icons, Info.plist, and entitlements as needed
    - Ensure the project can be discovered by `xcodebuild` from scripts
    - _Requirements: 2.1, 2.2, 2.5, 7.5_
    - _writes: `mobile/ios/Baymax.xcodeproj/`, `mobile/ios/Baymax/*.entitlements`_

  - [x] 3.2 Add iOS build and archive scripts
    - Create `mobile/scripts/ios-build.sh` for simulator/device compile validation
    - Create `mobile/scripts/ios-archive.sh` for archive and optional IPA export
    - Fail with a clear setup blocker if Xcode project metadata or `xcodebuild` is missing
    - Copy archive/export outputs into `mobile/build/ios/`
    - _Requirements: 2.1, 2.2, 2.4, 3.4, 7.1, 7.4, 7.5_
    - _writes: `mobile/scripts/ios-build.sh`, `mobile/scripts/ios-archive.sh`_

  - [x] 3.3 Add iOS signing and export configuration
    - Add checked-in export options template without secrets
    - Read team ID, bundle identifier, certificate, provisioning profile, and App Store Connect inputs from environment variables
    - Import signing material into a temporary keychain during CI archive/export
    - Fail before export or upload when required signing material is missing
    - Ensure secret values are never printed
    - _Requirements: 2.3, 2.4, 4.2, 4.3, 4.5_
    - _writes: `mobile/ios/ExportOptions.plist.template`, `mobile/scripts/ios-archive.sh`_

- [x] 4. Add active root-level mobile CI workflows
  - [x] 4.1 Add Android CI workflow
    - Create `.github/workflows/mobile_android_ci.yml`
    - Trigger on PRs and pushes that change Android app files, shared mobile scripts, or the mobile build/publish spec
    - Set up JDK and Android build cache
    - Run `mobile/scripts/android-test.sh` and a debug build through `mobile/scripts/android-build.sh`
    - _Requirements: 3.1, 3.4, 3.5_
    - _writes: `.github/workflows/mobile_android_ci.yml`_

  - [x] 4.2 Add iOS CI workflow
    - Create `.github/workflows/mobile_ios_ci.yml`
    - Trigger on PRs and pushes that change iOS app files, shared mobile scripts, or the mobile build/publish spec
    - Set up Xcode on macOS runner
    - Run `mobile/scripts/ios-build.sh`
    - Preserve a clear known-blocker failure until iOS project metadata exists
    - _Requirements: 3.2, 3.4, 3.5_
    - _writes: `.github/workflows/mobile_ios_ci.yml`_

- [x] 5. Add mobile release workflow in artifact-only mode
  - Create `.github/workflows/mobile_release.yml` with manual inputs for platform, channel, version, build number, and publish flag
  - Support Android, iOS, and all-platform builds using the shared scripts
  - Upload artifacts and release metadata to GitHub Actions artifacts
  - Default to artifact-only mode with no external store upload
  - Upload an installable Android APK for direct GitHub download when Android runs in artifact-only mode
  - Ensure artifact names include platform, channel, version, and commit SHA
  - _Requirements: 3.3, 3.6, 4.4, 5.1, 5.3, 5.4, 5.5_
  - _writes: `.github/workflows/mobile_release.yml`_

- [x] 6. Add external tester-channel publishing gates
  - [x] 6.1 Add Android Play Console upload path
    - Add optional Play Console upload when workflow platform includes Android, channel targets Play, and publish is true
    - Validate service account credentials and track input before upload
    - Require signed AAB for publishing
    - _Requirements: 4.1, 4.3, 4.5_
    - _writes: `.github/workflows/mobile_release.yml`, `mobile/scripts/android-publish.sh`_

  - [x] 6.2 Add iOS TestFlight upload path
    - Add optional App Store Connect upload when workflow platform includes iOS, channel targets TestFlight, and publish is true
    - Validate App Store Connect API credentials before upload
    - Require exported IPA for publishing
    - _Requirements: 4.2, 4.3, 4.5_
    - _writes: `.github/workflows/mobile_release.yml`, `mobile/scripts/ios-publish.sh`_

- [x] 7. Add mobile feature readiness validation
  - [x] 7.1 Create feature readiness checklist
    - Add `mobile/feature-readiness.yml` with items derived from existing mobile specs
    - Include implemented cross-platform areas: connection configuration, session listing, agent chat, SSE streaming, settings, trial mode, voice where available, Android/iOS parity, and remote connection/tunneling contracts
    - Mark each item as automated or manual with platform scope and spec paths
    - _Requirements: 6.1, 6.2, 6.3, 6.5_
    - _writes: `mobile/feature-readiness.yml`_

  - [x] 7.2 Implement readiness validation script
    - Create `mobile/scripts/mobile-readiness-check.sh`
    - Validate checklist schema, duplicate IDs, platform scopes, and referenced spec paths
    - Run automated static checks for Android and iOS deep-link declarations
    - Run automated fixture checks for shared QR/deep-link payload parsing where platform tests exist
    - Generate a manual validation summary for non-automated items
    - _Requirements: 6.3, 6.4, 6.5, 6.6_
    - _writes: `mobile/scripts/mobile-readiness-check.sh`, `mobile/readiness-fixtures/`_

  - [x] 7.3 Wire readiness validation into CI and release workflows
    - Run readiness validation when shared mobile scripts, mobile specs, Android app, iOS app, or desktop QR producer code changes
    - Ensure shared contract mismatches fail CI
    - Upload manual validation summary as a release artifact
    - _Requirements: 6.4, 6.5, 6.6_
    - _writes: `.github/workflows/mobile_android_ci.yml`, `.github/workflows/mobile_ios_ci.yml`, `.github/workflows/mobile_release.yml`_

- [x] 8. Document mobile build and publishing entry points
  - Update mobile documentation with local Android debug/release commands, iOS build/archive commands, and artifact output paths
  - Document required toolchains for Android SDK/JDK and Xcode
  - Document required CI secrets for Android signing, iOS signing, Play Console upload, and App Store Connect upload
  - Document which workflows validate only and which can publish externally
  - Document any remaining setup blockers until resolved
  - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5_
  - _writes: `mobile/README.md`, `mobile/android/README.md`, `mobile/ios/DEPLOYMENT.md`_

## Notes

- Do not move or rely on workflows under `mobile/.github`; root workflows under `.github/workflows/` are the active CI entry points.
- Android artifact-only release builds are available; external Play publishing still requires real signing and Play service-account secrets.
- iOS build/archive project metadata is present; signed export and TestFlight publishing still require real Apple signing and App Store Connect secrets.
- Signing material must come from environment variables or GitHub Secrets and must not be committed.
