# Baymax iOS Deployment

This guide documents the repository-owned iOS build, archive, and TestFlight entry points.

## Current Project

- Project: `mobile/ios/Baymax.xcodeproj`
- Scheme: `Baymax`
- Default bundle identifier: `com.simtropolis.baymaxchat`
- Default version/build: supplied by scripts with `--version` and `--build-number`
- Export template: `mobile/ios/ExportOptions.plist.template`

## Local Validation

Run from the repository root:

```bash
mobile/scripts/ios-build.sh --configuration Debug --version 1.0.0 --build-number 1
mobile/scripts/ios-archive.sh --signed false --export false --version 1.0.0 --build-number 1
```

Unsigned archives are artifact validation only. They are written under `mobile/build/ios/`.

## Signed Archive and IPA Export

Signed archive/export requires Apple Developer signing material from environment variables:

```bash
IOS_TEAM_ID=... \
IOS_SIGNING_CERTIFICATE_BASE64=... \
IOS_SIGNING_CERTIFICATE_PASSWORD=... \
IOS_PROVISIONING_PROFILE_BASE64=... \
mobile/scripts/ios-archive.sh --signed true --export true --version 1.0.0 --build-number 2
```

The script imports the certificate into a temporary keychain, installs the provisioning profile for the duration of the run, archives the app, and exports an IPA using `ExportOptions.plist.template`.

## TestFlight Publishing

The manual `.github/workflows/mobile_release.yml` workflow publishes to TestFlight only when:

- `platform=ios`
- `channel=testflight`
- `publish=true`

Required secrets:

- `IOS_TEAM_ID`
- `IOS_SIGNING_CERTIFICATE_BASE64`
- `IOS_SIGNING_CERTIFICATE_PASSWORD`
- `IOS_PROVISIONING_PROFILE_BASE64`
- `IOS_APP_STORE_CONNECT_KEY_ID`
- `IOS_APP_STORE_CONNECT_ISSUER_ID`
- `IOS_APP_STORE_CONNECT_API_KEY_BASE64`

The workflow builds a signed IPA, runs `mobile/scripts/ios-publish.sh`, writes release metadata, and uploads artifacts plus the mobile readiness summary.

## App Store Connect Setup

1. Create the iOS app record in App Store Connect with the bundle identifier used by the signing profile.
2. Create an App Store Connect API key and store the private key as base64 in `IOS_APP_STORE_CONNECT_API_KEY_BASE64`.
3. Create or export a distribution certificate as `.p12`, base64 encode it, and store it in `IOS_SIGNING_CERTIFICATE_BASE64`.
4. Create an App Store provisioning profile for the bundle ID, base64 encode it, and store it in `IOS_PROVISIONING_PROFILE_BASE64`.
5. Run the manual mobile release workflow with `platform=ios`, `channel=testflight`, and `publish=true`.

## Known Blockers

- Signed export and TestFlight upload require real Apple credentials and have not been exercised by local artifact-only validation.
- The current project builds and archives, but Xcode emits warnings about launch-screen/orientation configuration. Resolve those before App Store review.
- External TestFlight testing still requires Beta App Review and complete Test Information in App Store Connect.
