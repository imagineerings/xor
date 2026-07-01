# Design: Notifications

## 1. Overview

The notification system has two delivery channels: push (background via APNs/FCM) and in-app (foreground via toasts/overlays). Both share a common event data model and preference system.

### Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Push provider | Baymax Push Proxy Service (existing) | Already deployed; mobile-dev uses same |
| In-app toasts | Platform-native (iOS: Toast/SwiftUI overlay, Android: Snackbar) | Consistent with platform conventions |
| Preference storage | UserDefaults / DataStore | Simple key-value; no complex queries |
| Badge | Platform API (iOS: UIApplication.applicationIconBadgeNumber, Android: NotificationChannel badge) | Standard behavior |

## 2. Tasks

- [ ] 1. Push notification registration and token management
- [ ] 2. Remote push handling (tap → deep link to correct screen)
- [ ] 3. In-app notification toasts and overlays
- [ ] 4. Notification preferences settings screen
- [ ] 5. Badge management
