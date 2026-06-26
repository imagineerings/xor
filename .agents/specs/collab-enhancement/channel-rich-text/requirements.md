# Requirements: Rich Text Formatting in Channel Messages

## Introduction

Currently, Baymax channel chat supports only plain text messages (sent via the `body` string field in `SendChannelMessage`). Mattermost supports full Markdown formatting including bold, italic, inline code, code blocks with syntax highlighting, blockquotes, lists (ordered/unordered), headings, links, and LaTeX math. Adding rich text rendering to channel messages will bring the chat experience on par with modern collaboration tools and is the highest-impact enhancement to existing channel functionality.

## Glossary

- **Markdown**: Lightweight markup language for adding formatting to plain text
- **Channel Message**: A chat message sent within a Baymax channel
- **WYSIWYG**: "What You See Is What You Get" — a rich editing mode showing formatted content as it will appear
- **Source Mode**: Raw Markdown editing mode showing the unformatted source text

## Requirements

### Requirement 1.1: Markdown Rendering in Channel Messages

**User Story:** As a channel participant, I want messages to render formatted Markdown, so that I can emphasize text, share code, and organize information clearly.

#### Acceptance Criteria

1. WHEN a channel message body contains Markdown syntax THEN THE ChannelMessage rendering SHALL display the formatted output (bold, italic, strikethrough, inline code, code blocks, blockquotes, ordered/unordered lists, headings, links, images)
2. IF a code block includes a language identifier THEN THE code block SHALL render with syntax highlighting appropriate to that language
3. IF the Markdown syntax is malformed THEN THE system SHALL render the message in a best-effort manner without crashing
4. THE system SHALL support at minimum: `**bold**`, `*italic*`, `~~strikethrough~~`, `` `inline code` ``, code fences with language, `> blockquotes`, `- lists`, `1. numbered lists`, `# headers`, `[links](url)`, `![images](url)`

### Requirement 1.2: Message Composition with Formatting Toolbar

**User Story:** As a channel participant, I want a formatting toolbar when composing messages, so that I can easily apply formatting without remembering Markdown syntax.

#### Acceptance Criteria

1. WHEN the user selects text in the composition area THEN THE system SHALL display a floating formatting toolbar with options for bold, italic, code, link, and blockquote
2. WHEN the user clicks a formatting button THEN THE system SHALL insert the appropriate Markdown syntax around the selected text (or at cursor if no selection)
3. THE system SHALL provide a keyboard shortcut toggle for each formatting option (Ctrl+B for bold, Ctrl+I for italic, etc.)
4. THE formatting toolbar SHALL respect the current theme's color scheme and styling

### Requirement 1.3: Source/Preview Mode Toggle

**User Story:** As a power user, I want to toggle between source and preview modes while composing, so that I can verify formatting before sending.

#### Acceptance Criteria

1. WHEN composing a message THEN THE system SHALL provide a toggle button to switch between source mode (raw Markdown) and preview mode (rendered output)
2. WHILE in preview mode THE system SHALL show a live rendered preview of the message as it will appear in the channel
3. WHEN switching back to source mode THE system SHALL preserve the Markdown content unchanged

### Requirement 1.4: Safe Rendering

**User Story:** As a system administrator, I want Markdown rendering to be safe, so that malicious content in messages cannot harm users or the system.

#### Acceptance Criteria

1. WHEN rendering Markdown THEN THE system SHALL sanitize all HTML output to prevent XSS attacks
2. IF a link URL uses an untrusted protocol (`javascript:`, `data:`, `file:`) THEN THE system SHALL strip or disable the link
3. WHEN rendering external images THEN THE system SHALL require explicit user click-to-load or respect a "load images from unknown sources" setting
4. THE system SHALL render all links with `rel="noopener noreferrer"` attributes

### Requirement 1.5: Backward Compatibility

**User Story:** As an existing user, I want my plain text messages to continue working, so that existing conversations remain readable.

#### Acceptance Criteria

1. WHEN a message contains no Markdown syntax THEN THE system SHALL render it identically to current plain text rendering
2. WHEN upgrading, all existing messages in the database SHALL remain intact and renderable
