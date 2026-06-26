# Baymax Android

Android port of the Baymax iOS app, built with Jetpack Compose.

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
app/src/main/java/com/block/baymax/
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

## Building

1. Open the project in Android Studio
2. Sync Gradle files
3. Run on emulator or device (API 26+)

Or from command line:

```bash
./gradlew assembleDebug
```

## Requirements

- Android SDK 26+ (Android 8.0)
- Kotlin 1.9+
- Compose BOM 2023.10+

## API Compatibility

This app is designed to work with the same baymaxd API as the iOS app:

- `/status` - Connection test
- `/sessions` - List sessions  
- `/agent/start` - Start new agent
- `/sessions/{id}` - Get session
- `/reply` - Stream chat (SSE)

## License

See LICENSE file.
