# Requirements Document: Mobile Build and Publish

## Introduction

Baymax mobile tunneling work needs installable Android and iOS builds so remote connection flows can be tested on real devices and distributed to testers. Today the mobile tree contains Android Gradle project files, iOS Swift sources, and some deployment notes, but there is no active repo-level mobile build/publish specification that defines signed release artifacts, CI validation, or store/TestFlight publishing.

This feature establishes the build and distribution foundation for both mobile clients. It covers local build commands, CI workflows, release signing, versioning, artifact publication, and the minimum project metadata needed to produce Android and iOS installable builds.

## Glossary

- **Android App Bundle (AAB)**: The signed Android release artifact used for Play Console distribution.
- **APK**: Android installable package. Debug APKs are useful for development; signed release APKs may be useful for direct tester distribution.
- **IPA**: iOS installable archive exported from an Xcode archive for TestFlight or App Store distribution.
- **TestFlight**: Apple's beta distribution channel for iOS apps.
- **Play Console**: Google's app distribution console for Android apps.
- **Signing Material**: Certificates, provisioning profiles, keystores, passwords, API keys, and issuer IDs required to sign and upload mobile builds.
- **Release Channel**: A named distribution target such as debug artifact, internal tester build, TestFlight, or Play internal testing.
- **Mobile Release Workflow**: A GitHub Actions workflow that builds, signs, uploads, and records metadata for mobile artifacts.

## Requirements

### Requirement 1: Android Release Build

**User Story:** As a Baymax developer, I want Android release builds to be reproducible from the repository, so that I can publish the mobile app for device testing and distribution.

#### Acceptance Criteria

1.1 WHEN a developer runs the documented Android release build command THEN THE system SHALL produce a release artifact from `mobile/android`.

1.2 WHEN signing material is available THEN THE Android release build SHALL produce a signed AAB suitable for Play Console upload.

1.3 IF signing material is unavailable THEN THE Android release build SHALL fail with a clear message or produce only explicitly named unsigned artifacts.

1.4 THE Android project SHALL define application ID, version name, and version code from a reproducible source that can be controlled by CI.

1.5 WHEN the Android build runs in CI THEN THE system SHALL run the Android unit tests before publishing release artifacts.

### Requirement 2: iOS Archive Build

**User Story:** As a Baymax developer, I want the iOS app to archive from this repository, so that I can upload builds to TestFlight.

#### Acceptance Criteria

2.1 WHEN a developer runs the documented iOS archive command THEN THE system SHALL archive the iOS app from files stored in `mobile/ios`.

2.2 THE iOS project SHALL include the project or workspace metadata required by `xcodebuild`.

2.3 WHEN signing material is available THEN THE iOS archive workflow SHALL export an IPA suitable for TestFlight upload.

2.4 IF signing material is unavailable THEN THE iOS archive workflow SHALL fail with a clear message before attempting upload.

2.5 THE iOS project SHALL define bundle identifier, marketing version, and build number from a reproducible source that can be controlled by CI.

### Requirement 3: CI Build and Artifact Workflows

**User Story:** As a Baymax maintainer, I want active root-level CI workflows for mobile builds, so that mobile regressions are caught and release artifacts are consistently produced.

#### Acceptance Criteria

3.1 WHEN a pull request changes files under `mobile/android` THEN THE CI system SHALL build and test the Android app.

3.2 WHEN a pull request changes files under `mobile/ios` THEN THE CI system SHALL build the iOS app or run the fastest available compile validation.

3.3 WHEN a mobile release workflow is manually dispatched THEN THE CI system SHALL build the requested platform and upload artifacts with commit SHA, version, and build number metadata.

3.4 IF a mobile build fails THEN THE CI system SHALL surface the failing platform, command, and log location in the workflow result.

3.5 THE active workflows SHALL run from the repository root and SHALL NOT depend on inactive workflows under `mobile/.github`.

3.6 WHEN the Android mobile release workflow is manually dispatched in artifact-only mode THEN THE CI system SHALL upload an installable APK artifact for direct download from GitHub.

### Requirement 4: Publishing to Tester Channels

**User Story:** As a release owner, I want mobile release workflows to upload builds to tester channels, so that remote tunneling can be validated outside local development.

#### Acceptance Criteria

4.1 WHEN Android publishing is enabled for a release workflow THEN THE system SHALL upload the signed AAB to a configured Play Console track.

4.2 WHEN iOS publishing is enabled for a release workflow THEN THE system SHALL upload the exported IPA to App Store Connect for TestFlight processing.

4.3 IF publishing credentials are missing or invalid THEN THE workflow SHALL fail before artifact upload with a clear credential error.

4.4 THE release workflow SHALL support a dry-run or artifact-only mode that builds signed artifacts without uploading to external stores.

4.5 THE workflow SHALL avoid printing signing secrets, API keys, keystore passwords, or provisioning profile contents in logs.

### Requirement 5: Versioning and Release Metadata

**User Story:** As a release owner, I want mobile versions to be generated consistently, so that Android and iOS builds can be traced back to source revisions.

#### Acceptance Criteria

5.1 WHEN a mobile release workflow runs THEN THE system SHALL record the git commit SHA, platform, version, build number, and artifact names.

5.2 WHEN a release build is produced THEN THE Android `versionCode` and iOS build number SHALL be monotonically increasing for their target publishing channels.

5.3 IF a user supplies an explicit version or build number through workflow inputs THEN THE system SHALL validate the value before applying it.

5.4 THE system SHALL provide a default build numbering strategy for manual CI dispatches.

5.5 WHERE release artifacts are uploaded to GitHub, THE artifact names SHALL include platform, channel, version, and commit SHA.

### Requirement 6: Mobile Feature Readiness Validation

**User Story:** As a developer validating mobile releases, I want published builds to verify the implemented mobile feature surface, so that tester builds are useful for validating the mobile specs on real devices.

#### Acceptance Criteria

6.1 THE mobile build validation SHALL define a feature readiness checklist derived from the mobile specs under `.agents/specs/mobile`.

6.2 THE feature readiness checklist SHALL include currently implemented cross-platform features such as connection configuration, session listing, agent chat, SSE streaming, settings, trial mode, voice where available, and Android/iOS parity checks.

6.3 THE feature readiness checklist SHALL include remote connection and tunneling checks, including deep-link scheme declarations and QR/deep-link payload compatibility across Android, iOS, and the desktop QR generator.

6.4 WHEN a mobile release validation workflow runs THEN THE system SHALL execute automated checks for every feature readiness item that can be validated without manual tester interaction.

6.5 IF a feature readiness item cannot be automated yet THEN THE system SHALL record it as a manual validation item with platform scope, expected behavior, and the spec requirement it traces to.

6.6 IF a shared mobile contract changes, such as the QR/deep-link payload format or API response model, THEN THE system SHALL fail validation until Android, iOS, and any desktop producer agree on the new contract.

### Requirement 7: Documentation and Developer Entry Points

**User Story:** As a Baymax developer, I want clear mobile build commands and CI entry points, so that I can build and publish without reverse-engineering platform setup.

#### Acceptance Criteria

7.1 THE repository SHALL document local Android debug, Android release, iOS build, and iOS archive commands.

7.2 THE repository SHALL document the required CI secrets for Android signing, iOS signing, Play Console upload, and App Store Connect upload.

7.3 THE repository SHALL document which workflows are validation-only and which workflows can publish externally.

7.4 IF a command requires platform-specific tools such as Xcode or Android SDK THEN THE documentation SHALL state the required toolchain.

7.5 THE documentation SHALL identify known setup blockers, including missing iOS project metadata, until those blockers are resolved.
