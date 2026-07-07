# Requirements: Notifications

## Introduction

The Sim mobile client needs a comprehensive notification system covering push notifications (for agent responses and collab events when the app is backgrounded) and in-app notifications (toasts, alerts). This enables users to stay informed about agent task completion, incoming calls, channel messages, and other events. This spec draws from `mobile-dev`'s push notification infrastructure, notification preferences, and the Sim desktop's notification system.

## Glossary

| Term | Definition |
|------|------------|
| **Push Notification** | A remote notification delivered via APNs (iOS) or FCM (Android) when the app is in the background. |
| **In-App Notification** | A visual alert (toast, banner, snack bar) displayed while the app is in the foreground. |
| **Notification Preference** | User-configurable settings controlling what types of notifications are delivered and how. |
| **Push Proxy** | The Sim Push Notification Service that bridges agent/collab events to APNs/FCM. |
| **Badge** | The numbered badge on the app icon indicating unread notification count. |

## Requirements

### Requirement 1: Push Notifications

**User Story:** As a mobile user, I want to receive push notifications when my agent completes a task or a collaborator messages me, so I don't need to keep the app open.

1.1 WHEN the app is in the background AND the agent completes a response THEN THE system SHALL deliver a push notification with the response preview.

1.2 WHEN the app is in the background AND a collaborator sends a channel message THEN THE system SHALL deliver a push notification.

1.3 WHEN the app is in the background AND there is an incoming call THEN THE system SHALL deliver a push notification with caller information.

1.4 WHEN the user taps a push notification THEN THE app SHALL open the relevant screen:
   - Agent response → open the session
   - Channel message → open the channel
   - Incoming call → show the call accept UI

1.5 THE app SHALL register for push notifications on first launch, requesting platform permission.

1.6 THE app SHALL manage the push notification token (register with push proxy on connect, unregister on disconnect).

### Requirement 2: In-App Notifications

**User Story:** As a mobile user, I want to see notifications within the app so I don't miss important events while actively using it.

2.1 WHEN a new channel message arrives while the app is foregrounded THEN THE app SHALL show a brief toast notification.

2.2 WHEN an incoming call arrives while the app is foregrounded THEN THE app SHALL show an incoming call overlay.

2.3 WHEN a project is shared with the user while the app is foregrounded THEN THE app SHALL show a notification with accept/decline options.

2.4 THE in-app notification SHALL auto-dismiss after 5 seconds (except incoming call, which requires user action).

### Requirement 3: Notification Preferences

**User Story:** As a mobile user, I want to control which notifications I receive, so I'm not overwhelmed.

3.1 THE app SHALL provide a notification settings screen with toggles for:
   - Agent response notifications
   - Channel message notifications
   - Call notifications
   - Sound on/off for each category

3.2 THE app SHALL support per-channel notification override (all messages, mentions only, mute).

3.3 THE app SHALL support quiet hours / do-not-disturb scheduling.

### Requirement 4: Badge Management

**User Story:** As a mobile user, I want the app icon badge to reflect my unread count, so I know at a glance if there's something new.

4.1 THE app SHALL update the app icon badge to reflect total unread notifications.

4.2 WHEN the user opens the app or reads the relevant content THEN THE badge SHALL decrement accordingly.

## Existing Assets

- mobile-dev: `app/init/push_notifications.ts`, `app/constants/push_notification.ts`, `app/constants/push_proxy.ts`, `app/managers/network_manager.ts`, `app/screens/in_app_notification/`, `app/components/toast/`
- Sim desktop: `crates/collab_ui/src/notifications/` (incoming call, project shared notifications), `crates/notifications/` (notification store)
