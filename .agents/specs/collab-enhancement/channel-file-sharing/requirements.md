# Requirements: File Upload and Preview in Channels

## Introduction

Baymax channel chat currently sends only text messages. There is no mechanism to attach, upload, or preview files within channels. Mattermost provides comprehensive file support: upload via drag-and-drop or file picker, inline previews for images/PDFs/video, file metadata display, and file search. Adding file sharing will enable developers to share screenshots, logs, patches, and documents within collaborative channels.

## Glossary

- **File Attachment**: A file uploaded and associated with a channel message
- **Inline Preview**: A rendered thumbnail or preview shown directly in the channel
- **File Metadata**: Information about a file (name, size, type, upload date, uploader)
- **File Limit**: Maximum upload size and storage quota per user/workspace

## Requirements

### Requirement 4.1: File Upload via Drag-and-Drop and File Picker

**User Story:** As a channel participant, I want to attach files to my messages, so that I can share relevant documents, screenshots, and code snippets.

#### Acceptance Criteria

1. WHEN a user drags a file onto the message composition area THEN THE system SHALL show a drop zone overlay with visual feedback
2. WHEN the user drops a file onto the drop zone THEN THE system SHALL upload the file and attach it to the pending message
3. WHEN the user clicks a file attachment button in the compose toolbar THEN THE system SHALL open a native file picker dialog
4. WHEN the user selects a file in the picker THEN THE system SHALL upload the file and attach it to the pending message
5. WHEN the message is sent THEN THE system SHALL include the uploaded file(s) as attachments
6. THE system SHALL support uploading multiple files in a single message

### Requirement 4.2: File Inline Previews

**User Story:** As a channel participant, I want to see inline previews of shared files, so that I can view content without downloading.

#### Acceptance Criteria

1. WHEN a message contains an image attachment (PNG, JPEG, GIF, WebP, SVG) THEN THE system SHALL render it as an inline preview in the channel
2. WHEN a message contains a PDF attachment THEN THE system SHALL render a thumbnail preview with a "View PDF" link
3. WHEN a message contains a text/code file THEN THE system SHALL render a syntax-highlighted snippet preview
4. WHEN a user clicks on an image preview THEN THE system SHALL open a lightbox/gallery view for larger examination
5. THE system SHALL display video files with a player element and audio files with an audio player

### Requirement 4.3: File Metadata Display

**User Story:** As a channel participant, I want to see file metadata, so that I know what was shared and by whom.

#### Acceptance Criteria

1. WHEN a message has file attachments THEN THE system SHALL display for each file: filename, file size (formatted), file type icon, and uploader name
2. WHEN the user clicks the filename THEN THE system SHALL download the file
3. THE system SHALL display a download count or "N downloads" for files that have been downloaded

### Requirement 4.4: File Upload Limits and Storage

**User Story:** As a system administrator, I want configurable file upload limits, so that storage usage is controlled.

#### Acceptance Criteria

1. THE system SHALL enforce configurable maximum file size per upload
2. THE system SHALL enforce configurable total storage quota per user/workspace
3. WHEN a user exceeds the file size limit THEN THE system SHALL display a clear error message
4. WHEN a user exceeds the storage quota THEN THE system SHALL display a warning banner
5. THE system SHALL support configurable allowed file type extensions (allowlist/blocklist)

### Requirement 4.5: File Message Rendering in Channel

**User Story:** As a channel participant, I want files to render cleanly within the message flow, so that the channel stays readable.

#### Acceptance Criteria

1. WHEN a message contains both text and file attachments THEN THE text SHALL appear above the file previews
2. WHEN a message contains only file attachments THEN THE system SHALL display the file previews without extra spacing
3. THE file preview area SHALL be visually distinct from the message text
4. WHEN the user deletes a message THEN THE associated uploaded files SHALL be deleted as well
