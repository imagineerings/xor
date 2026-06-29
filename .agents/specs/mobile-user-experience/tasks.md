# Implementation Plan: User Experience

- [ ] 1. Implement user profile view and edit screen
  - Display avatar, name, email, custom attributes (from agent API or collab server)
  - Edit: change name, avatar from camera/photo library
  - _Requirements: 1.1, 1.2_
  - _writes: iOS: `Views/ProfileView.swift`, `Views/EditProfileView.swift`; Android: `ui/screens/ProfileScreen.kt`, `ui/screens/EditProfileScreen.kt`_

- [ ] 2. Implement custom status picker
  - Emoji picker + text input + auto-clear duration selector
  - Display status next to name, auto-clear on timer expiry
  - _Requirements: 2.1–2.5_
  - _writes: iOS: `Views/CustomStatusPicker.swift`; Android: `ui/screens/CustomStatusScreen.kt`_

- [ ] 3. Implement theme manager
  - Light/dark/system, accent color, font size, clock format, timezone
  - CRT mode toggle for code blocks
  - _Requirements: 3.1–3.5_
  - _writes: iOS: `Services/ThemeManager.swift` (extend); Android: `data/repository/ThemeManager.kt` (extend)_

- [ ] 4. Implement onboarding flow
  - Welcome screen with Trial, Configure, Scan QR options
  - Only shown on first launch (no saved credentials)
  - _Requirements: 4.1, 4.2_
  - _writes: iOS: `Views/OnboardingView.swift`; Android: `ui/screens/OnboardingScreen.kt`_

- [ ] 5. Implement tutorial highlights
  - Highlight chat input, session list, settings on first use
  - Dismissable, persistent "don't show again"
  - _Requirements: 4.3, 4.4_
  - _writes: iOS: `Components/TutorialHighlight.swift`; Android: `ui/components/TutorialHighlight.kt`_
