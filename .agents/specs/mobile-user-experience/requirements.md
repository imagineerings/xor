# Requirements: User Experience

## Introduction

The Baymax mobile client needs polished user experience features including user profiles, custom status, theming, and onboarding. These features make the app feel complete and help users manage their identity and preferences. This spec draws from `mobile-dev`'s profile management, custom status, theme settings, and tutorial flows.

## Glossary

| Term | Definition |
|------|------------|
| **User Profile** | The user's display information: avatar, display name, email, and custom attributes. |
| **Custom Status** | A user-set status message with emoji (e.g., ":palm_tree: On vacation") that auto-clears after a set duration. |
| **Theme** | The visual appearance of the app — color scheme, typography, and component styling. |
| **CRT Mode** | A display mode that applies CRT monitor visual effects (scanlines, glow) to the code interface. |
| **Onboarding** | The first-run experience that introduces the user to the app's features. |

## Requirements

### Requirement 1: User Profile

**User Story:** As a mobile user, I want to view and edit my profile, so I can control how others see me.

1.1 THE app SHALL display the user's profile showing: avatar, display name, email, and any custom attributes.

1.2 WHEN the user taps "Edit Profile" THEN THE app SHALL allow changing: display name, avatar (from camera or photo library).

1.3 WHEN the user is connected to a collab server THEN THE profile SHALL sync with the collab server user profile.

### Requirement 2: Custom Status

**User Story:** As a mobile user, I want to set a custom status with an emoji and optional auto-clear, so others know my availability.

2.1 THE app SHALL provide a custom status picker accessible from the profile or settings.

2.2 THE status picker SHALL show: emoji selector + text input + auto-clear duration selector (30min, 1hr, 4hrs, today, this week, never).

2.3 WHEN the user sets a custom status THEN THE app SHALL display it next to the user's name in the contact list and profile.

2.4 WHEN the auto-clear duration expires THEN THE app SHALL automatically clear the status.

2.5 THE app SHALL show a "Clear After" indicator on the current status showing when it will auto-clear.

### Requirement 3: Theme & Display

**User Story:** As a mobile user, I want to customize the app's appearance to my preference.

3.1 THE app SHALL support light mode and dark mode theme switching (manual or follow system).

3.2 THE app SHALL support selecting accent/tint colors.

3.3 THE app SHALL support adjusting font size (small, medium, large).

3.4 THE app SHALL support setting the time format (12-hour vs 24-hour) and timezone.

3.5 THE app SHALL support CRT mode for code blocks (scanline overlay effect).

### Requirement 4: Onboarding & Tutorials

**User Story:** As a new user, I want to be guided through the app's features, so I can quickly understand how to use it.

4.1 WHEN the app is launched for the first time (no saved credentials) THEN THE app SHALL show a welcome screen introducing Baymax.

4.2 THE onboarding SHALL offer:
   - "Try Trial Mode" — connect to demo server
   - "Configure Server" — enter URL and secret
   - "Scan QR Code" — configure via QR scan

4.3 THE app SHALL show a tutorial highlight on key UI elements on first use (chat input, session list, settings button).

4.4 THE app SHALL allow the user to dismiss tutorial highlights and not show them again.

## Existing Assets

- iOS: `SplashScreenView.swift`, `WelcomeView.swift`, `WelcomeCard.swift`, `ThemeManager.swift`
- Android: `SplashScreen.kt`, `WelcomeCard.kt`, `theme/Theme.kt`, `theme/Type.kt`
- mobile-dev: `app/screens/edit_profile/`, `app/screens/custom_status/`, `app/screens/custom_status_clear_after/`, `app/screens/settings/` (display settings), `app/screens/onboarding/`, `app/components/custom_status/`, `app/components/tutorial_highlight/`, `app/constants/tutorial.ts`
