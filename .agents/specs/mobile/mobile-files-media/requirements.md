# Requirements: Files & Media

## Introduction

The Sim mobile client needs to handle file attachments, media viewing, and code visualization. Users need to share files with the agent as context, view documents shared by collaborators, and examine code snippets with proper syntax highlighting. This spec draws from `mobile-dev`'s file management, document picker, media viewer, and code viewer features.

## Glossary

| Term | Definition |
|------|------------|
| **File Attachment** | A file added to a message as context for the agent. Can be an image, document, or code file. |
| **Document Picker** | A platform-native UI for selecting files from the device. |
| **PDF Viewer** | In-app rendering of PDF documents with page navigation. |
| **Syntax Highlighting** | Color-coded display of code with language-specific tokenization. |

## Requirements

### Requirement 1: File Attachments

**User Story:** As a mobile user, I want to attach files to my messages so the agent can use them as context.

1.1 THE app SHALL provide an attachment button (paperclip icon) in the chat input area.

1.2 WHEN the user taps the attachment button THEN THE app SHALL show options: Photo Library, Take Photo, Choose File, Paste.

1.3 WHEN the user selects a file THEN THE app SHALL upload it to the agent API and show a upload progress indicator with cancel option.

1.4 ON upload complete, the attachment preview SHALL appear in the input area above the text field.

1.5 WHEN the user sends the message, the attachment SHALL be included as context for the agent.

1.6 THE app SHALL support these file types: images (PNG, JPG, GIF, WebP), documents (PDF, TXT, Markdown, JSON, CSV), code files (all common extensions).

1.7 THE agent response SHALL be able to reference attached files (e.g., "I've reviewed the code in `main.swift`...").

### Requirement 2: Media Viewer

**User Story:** As a mobile user, I want to view images, PDFs, and other documents inline in the conversation.

2.1 WHEN an image is referenced in a message THEN THE app SHALL display it inline as a thumbnail.

2.2 WHEN the user taps an image thumbnail THEN THE app SHALL open a full-screen image viewer with zoom and pan.

2.3 WHEN a PDF is referenced in a message THEN THE app SHALL display a PDF preview with file name, size, and an "Open" button.

2.4 WHEN the user taps a PDF preview THEN THE app SHALL open an in-app PDF viewer with page navigation and search.

2.5 WHEN the agent generates an image (e.g., diagram, chart) THEN THE app SHALL display it inline.

2.6 THE app SHALL support saving images to the device's photo library via long-press or share action.

### Requirement 3: Code Blocks & Syntax Highlighting

**User Story:** As a mobile user, I want to read code blocks with proper syntax highlighting, so code is easy to understand.

3.1 WHEN the SSE stream or a message contains a code block THEN THE app SHALL render it with syntax highlighting for the specified language.

3.2 THE app SHALL support syntax highlighting for all common languages (Swift, Kotlin, Rust, TypeScript, Python, Go, etc.).

3.3 THE app SHALL show a language label on each code block.

3.4 THE app SHALL provide a "Copy" button on each code block.

3.5 THE app SHALL support long-press on code blocks to select and copy text.

### Requirement 4: Document Generation

**User Story:** As a mobile user, I want the agent to generate documents (PDF reports, markdown docs, etc.) that I can view and share.

4.1 WHEN the agent generates a document (PDF, markdown, HTML) THEN THE app SHALL offer to open or save it.

4.2 THE app SHALL support sharing generated documents via the platform share sheet.

## Existing Assets

- iOS: (basic, needs extension)
- Android: (basic, needs extension)
- mobile-dev: `app/components/files/`, `app/components/files_search/`, `app/components/expo_image/`, `app/components/progressive_image/`, `app/screens/gallery/`, `app/screens/pdf_viewer/`, `app/screens/code/`, `app/screens/latex/`, `app/constants/files.ts`, `app/constants/gallery.ts`, `app/constants/image.ts`
