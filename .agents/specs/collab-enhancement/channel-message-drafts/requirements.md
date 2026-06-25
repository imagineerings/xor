# Requirements: Message Drafts

## Introduction

Baymax channel message composition currently loses unsent content if the user navigates away or closes the window. Mattermost supports message drafts — composition state that persists across sessions. Adding drafts will prevent data loss and improve the composition experience.

## Glossary

- **Draft**: An unsent message composition that is saved locally (and optionally on the server)
- **Draft Indicator**: A visual badge showing that a channel has an unsaved draft

## Requirements

### Requirement 7.1: Auto-Save Drafts Locally

**User Story:** As a channel participant, I want my unsent message to be automatically saved, so that I don't lose my work if I navigate away or close the app.

#### Acceptance Criteria

1. WHEN a user types in the message composition area AND the content is non-empty THEN THE system SHALL auto-save the draft to local storage every 2 seconds
2. WHEN the user navigates away from a channel with an unsaved draft THEN THE system SHALL preserve the draft in local storage
3. WHEN the user returns to a channel that has a saved draft THEN THE system SHALL restore the draft content into the composition area
4. WHEN the user sends the message THEN THE system SHALL clear the saved draft

### Requirement 7.2: Draft Channel Indicators

**User Story:** As a channel participant, I want to see which channels have unsaved drafts, so that I can remember to finish composing.

#### Acceptance Criteria

1. WHEN a channel has an unsaved draft THEN THE system SHALL display a draft indicator icon (e.g., pencil icon or italicized channel name) in the channel sidebar
2. WHEN the draft is cleared by sending or discarding the message THEN THE system SHALL remove the draft indicator
3. THE draft indicator SHALL be visible to the user across all sessions on the same device

### Requirement 7.3: Discard Drafts

**User Story:** As a channel participant, I want to discard a draft, so that I can clear unwanted unsaved content.

#### Acceptance Criteria

1. WHEN a user has a draft AND presses Escape (or a "Discard" button) THEN THE system SHALL show a confirmation dialog
2. WHEN the user confirms discard THEN THE system SHALL clear the draft content and remove the draft indicator
3. WHEN the user cancels discard THEN THE system SHALL keep the draft intact

### Requirement 7.4: Draft Persistence Across Sessions

**User Story:** As a channel participant, I want my drafts to survive app restarts, so that I don't lose work.

#### Acceptance Criteria

1. THE system SHALL persist drafts to local disk (not just memory)
2. WHEN the app is closed and reopened THEN THE system SHALL restore all saved drafts
3. THE system SHALL limit drafts to a reasonable maximum count (e.g., 50 channels) and oldest drafts SHALL be evicted when the limit is exceeded
