# Implementation Plan: Files & Media

- [ ] 1. Implement file upload service with progress tracking
  - Multipart upload to agent API, progress callbacks, cancel support
  - _Requirements: 1.1–1.7_
  - _writes: iOS: `Services/FileUploadService.swift`; Android: `data/repository/FileUploadService.kt`_

- [ ] 2. Implement document picker integration
  - Photo library, camera, file picker options from attachment button
  - Upload progress indicator, cancel option, preview in input area
  - _Requirements: 1.1–1.7_
  - _writes: iOS: `Components/AttachmentPicker.swift`; Android: `ui/components/AttachmentPicker.kt`_

- [ ] 3. Implement image viewer
  - Inline thumbnail → tap for full-screen with zoom/pan
  - Save to photo library, share
  - _Requirements: 2.1, 2.2, 2.5, 2.6_
  - _writes: iOS: `Views/ImageViewer.swift`; Android: `ui/screens/ImageViewerScreen.kt`_

- [ ] 4. Implement PDF viewer
  - Preview card (file name, size, "Open") → in-app PDF viewer
  - Page navigation, search
  - _Requirements: 2.3, 2.4_
  - _writes: iOS: `Views/PDFViewer.swift`; Android: `ui/screens/PDFViewerScreen.kt`_

- [ ] 5. Implement code block highlighting and copy
  - Syntax highlighting, language label, copy button
  - _Requirements: 3.1–3.5_
  - _writes: iOS: `Components/CodeBlockView.swift` (from mobile-agent-chat); Android: `ui/components/SyntaxHighlighter.kt` (extend)_
