# Baymax Android

Android client for the Baymax mobile HTTP/SSE protocol, built with Jetpack Compose.

## Features

- **Chat Interface**: Send messages and receive streaming responses from the Baymax API
- **Session Management**: View, create, and resume chat sessions
- **Settings**: Configure server URL and secret key
- **Trial Mode**: Connect to the demo server by default
- **Dark/Light Theme**: Automatic theme switching based on system settings

## Architecture

The app follows a clean architecture pattern with:

- **UI Layer**: Jetpack Compose screens and components
- **ViewModel**: State management with StateFlow
- **Data Layer**: Repository pattern with OkHttp for networking
- **DataStore**: Persistent settings storage

## Project Structure

```
app/src/main/java/com/simtropolis/baymax/
├── BaymaxApplication.kt      # Application class with DI
├── MainActivity.kt          # Entry point with navigation
├── data/
│   ├── api/
│   │   ├── BaymaxApiService.kt    # API client with SSE support
│   │   └── SettingsRepository.kt # Preferences management
│   └── model/
│       ├── Message.kt            # Message data models
│       ├── ChatSession.kt        # Session models
│       └── SSEEvent.kt           # SSE event types
└── ui/
    ├── theme/
    │   ├── Theme.kt              # Material 3 theme
    │   └── Type.kt               # Typography
    ├── components/
    │   ├── ChatInputView.kt      # Input field component
    │   ├── MessageBubble.kt      # Message display
    │   └── WelcomeCard.kt        # Welcome header
    └── screens/
        ├── HomeScreen.kt         # Main screen
        ├── ChatScreen.kt         # Chat interface
        ├── SettingsScreen.kt     # Settings
        └── *ViewModel.kt         # State management
```

## Building and Testing

From the repository root:

```bash
mobile/scripts/android-test.sh
mobile/scripts/android-build.sh --variant debug --artifact apk --version 1.0.0 --build-number 1
mobile/scripts/android-build.sh --variant release --artifact aab --signed false --version 1.0.0 --build-number 1
```

Artifacts are copied to `mobile/build/android/` with names that include platform, variant, signing mode, version, build number, and commit SHA.

## GitHub APK Download

To create an APK for direct download from GitHub:

1. Open the `mobile_release` workflow in GitHub Actions.
2. Run it manually with:
   - `platform=android`
   - `channel=artifact`
   - `publish=false`
   - the desired `version` and `build_number`
3. Download the `baymax-android-apk-<version>-<build>-<sha>` artifact from the completed workflow run.

This APK is built with Android's debug signing key so it can be installed for testing without Play Console signing secrets. Play/internal distribution still uses the release AAB path.

You can also build directly from `mobile/android`:

```bash
./gradlew assembleDebug
```

## Release Signing

Signed release builds use environment-provided keystore material:

```bash
ANDROID_KEYSTORE_BASE64=... \
ANDROID_KEYSTORE_PASSWORD=... \
ANDROID_KEY_ALIAS=... \
ANDROID_KEY_PASSWORD=... \
mobile/scripts/android-build.sh --variant release --artifact aab --signed true --version 1.0.0 --build-number 2
```

For Play internal publishing, the manual `mobile_release` workflow requires `ANDROID_PLAY_SERVICE_ACCOUNT_JSON_BASE64`. Publishing is handled by `mobile/scripts/android-publish.sh` through Fastlane `supply`.

## CI

`.github/workflows/mobile_android_ci.yml` runs script tests, readiness validation, Android unit tests, and a debug APK build when Android, shared mobile scripts, mobile build/publish specs, or the workflow change.

## Requirements

- Android SDK 26+ (Android 8.0)
- JDK 17+
- Kotlin 1.9+
- Compose BOM 2023.10+

## API Compatibility

This app is designed to work with the same baymaxed API as the iOS app:

- `/status` - Connection test
- `/sessions` - List sessions  
- `/agent/start` - Start new agent
- `/sessions/{id}` - Get session
- `/reply` - Stream chat (SSE)

## License

See LICENSE file.
