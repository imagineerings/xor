# Design: Files & Media

## 1. Overview

File and media features enable users to attach files as agent context and view documents inline. The architecture uses platform-native document pickers and media viewers with a shared upload service that communicates with the agent API.

### Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Document picker | Platform-native (iOS: UIDocumentPickerViewController, Android: ActivityResultContracts) | Best UX, permission handling |
| Image picker | Platform-native (iOS: UIImagePickerController/PHPicker, Android: ActivityResultContracts.GetContent) | Standard platform integration |
| PDF viewer | Platform-native (iOS: PDFKit, Android: PdfRenderer) | No external dependency |
| File upload | Multipart POST to agent API | Standard HTTP, progress tracking with URLSession/OkHttp |

## 2. Tasks

- [ ] 1. File upload service with progress tracking
- [ ] 2. Document picker integration (attach button → picker → upload)
- [ ] 3. Image viewer (thumbnail, full-screen with zoom)
- [ ] 4. PDF viewer (in-app with page navigation)
- [ ] 5. Code block syntax highlighting and copy
