# Implementation Plan: File Upload and Preview in Channels

## Overview

Add file attachment support to channel messages: upload via drag-and-drop or file picker, inline previews for images/PDFs/video/audio/code, file metadata display, and storage management. Work is organised into incremental layers — proto types, server storage, server RPCs, client upload pipeline, client rendering, and tests — each building on the previous.

- **proto** crate: new `FileAttachment` message, `GetFileUploadUrl` / `ConfirmFileUpload` RPCs, file fields on `ChannelMessage`
- **collab** crate: `FileStore` backed by S3 presigned URLs and `channel_files` DB table, RPC handler implementations, storage limits
- **client** crate: typed Rust wrappers for the new RPCs, `FileAttachment` model
- **collab_ui** crate: `UploadManager` entity, drag-and-drop zone + file picker in compose area, `FileAttachmentRenderer` with per-type previews
- **Server** (collab crate uses Go; proto and Rust client use Rust): all server-side logic written in Go

## Tasks

### Phase 1: Protocol & Data Layer

- [x] 1. Define protobuf messages and RPCs
  - Add `FileAttachment` message, `GetFileUploadUrl` / `GetFileUploadUrlResponse` messages, `ConfirmFileUpload` / `ConfirmFileUploadResponse` messages to the proto schema.
  - Add `repeated FileAttachment files` field to `ChannelMessage`.
  - Register new RPCs in the service definition.
  - _Requirements: 4.1_
  - _writes: proto/src/proto/ffi/channel_messages.proto_
  - _Completed: Added `FileAttachment`, upload URL and confirm upload messages, `ChannelMessage.files`, envelope variants, and typed request mappings in the live Rust proto schema._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto`; `CARGO_INCREMENTAL=0 cargo check -p client -p collab --features collab/test-support`._

- [x] 2. Generate Rust protobuf types and RPC stubs
  - Run the proto code generator to produce Rust types for the new messages and RPC client/server traits.
  - _Requirements: 4.1_
  - _writes: crates/proto/src/proto.rs_ (generated)
  - _Completed: Verified the repo's `prost` build generates the new file-sharing messages and typed RPC traits from `OUT_DIR` during the proto crate build._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto`; `CARGO_INCREMENTAL=0 cargo check -p client -p collab --features collab/test-support`._

- [x] 3. Run database migration to create `channel_files` table
  - Write the SQL migration to create `channel_files` (UUID primary key, `channel_id`, `message_id`, `filename`, `file_size`, `mime_type`, `storage_path`, `uploader_id`, `image_width`, `image_height`, `duration_ms`, `created_at`, with foreign keys and indexes).
  - _Requirements: 4.1, 4.3_
  - _writes: crates/collab/src/migrations/YYYYMMDDHHMMSS_create_channel_files.sql_
  - _Completed: Added the Postgres `channel_files` migration and matching SQLite integration-test schema with channel/message/uploader references, metadata fields, upload confirmation timestamp, and lookup indexes._
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab --features collab/test-support`; `git diff --check`._

### Phase 2: Server — FileStore & RPC Handlers

- [x] 4. Implement `FileStore` with S3 presigned URL generation
  - Create `FileStore` struct with S3 client, DB pool, and config (`MaxFileSize`, `AllowedTypes`, `StorageBucket`).
  - Implement `GenerateUploadUrl`: validate file size and mime type against config, generate S3 presigned upload URL, insert a pending file record into `channel_files`, return URL with headers and file ID.
  - Implement `ConfirmUpload`: mark the file record as uploaded, generate download URL, return `FileAttachment`.
  - Implement `GetFileMetadata`: lookup by file ID.
  - Enforce `Property 5.1` (file size) and `Property 5.2` (file type allowlist).
  - _Requirements: 4.1, 4.4_
  - _writes: crates/collab/src/storage/file_store.go_
  - _Completed: Added a Rust `FileStore` with S3 presigned PUT/GET URL generation, pending `channel_files` inserts, upload confirmation, metadata lookup, file-size/type validation, proto conversion, and a SeaORM `channel_files` entity._
  - _Actual writes: crates/collab/src/db/file_store.rs; crates/collab/src/db/tables/channel_file.rs; crates/collab/src/db.rs; crates/collab/src/db/tables.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab --features collab/test-support`; `git diff --check`._

- [x] 5. Implement server RPC handlers for file upload
  - Wire `GetFileUploadUrl` handler calling `FileStore.GenerateUploadUrl`.
  - Wire `ConfirmFileUpload` handler calling `FileStore.ConfirmUpload`.
  - Wire file metadata lookup for existing messages.
  - Return proper error codes (413 for size, 415 for type, 503 for S3 unavailable).
  - _Requirements: 4.1, 4.4_
  - _writes: crates/collab/src/rpc/file_upload.go_
  - _Completed: Registered Rust `GetFileUploadUrl` and `ConfirmFileUpload` handlers in collab RPC, wired them to `FileStore`, added upload size/type/storage config, enforced channel upload permission and uploader-owned confirmation, and added typed file-upload RPC error codes._
  - _Actual writes: crates/collab/src/rpc.rs; crates/collab/src/lib.rs; crates/collab/src/db/file_store.rs; crates/collab/tests/integration/test_server.rs; crates/proto/proto/sim.proto_
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto -p client -p collab --features collab/test-support`; `git diff --check`._

- [x] 6. Implement file cleanup on message deletion
  - When a channel message is deleted, query associated `channel_files` rows.
  - Delete file metadata records from the database (ON DELETE SET NULL or explicit delete).
  - Schedule S3 object deletion (garbage collection or inline delete).
  - _Requirements: 4.5_
  - _writes: crates/collab/src/storage/file_store.go_ (add `DeleteFile`)
  - _writes: crates/collab/src/rpc/channel_messages.go_ (hook into delete path)
  - _Completed: Added Rust `FileStore::delete_message_files` to remove `channel_files` metadata for a deleted message and attempt S3 object deletion, then hooked channel message deletion to invoke cleanup with logged error visibility._
  - _Actual writes: crates/collab/src/db/file_store.rs; crates/collab/src/rpc.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab --features collab/test-support`; `git diff --check`._

### Phase 3: Client — RPC Integration & UploadManager

- [x] 7. Add Rust client RPC methods for file upload
  - Add `get_file_upload_url` and `confirm_file_upload` methods on the `Client` or `RpcClient` struct.
  - Add `upload_file_to_s3` helper to perform the raw PUT to the presigned URL with progress reporting.
  - _Requirements: 4.1_
  - _writes: crates/client/src/rpc.rs_
  - _Completed: Added typed client file upload models, `get_file_upload_url`, `upload_file_to_s3`, and `confirm_file_upload` methods using the existing RPC and HTTP client abstractions._
  - _Actual writes: crates/client/src/file_upload.rs; crates/client/src/channel_chat.rs; crates/client/src/client.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p client`; `git diff --check`._

- [x] 8. Implement `UploadManager` entity
  - Create `UploadManager` struct holding `Arc<Client>` and `active_uploads: HashMap<FileId, UploadProgress>`.
  - Implement `upload_file(channel_id, file_path, cx)` → `Task<Result<FileAttachment>>`: request URL via RPC, upload to S3 with progress, confirm upload via RPC.
  - Implement `uploads_for_channel(channel_id)` for progress bar rendering.
  - Implement `cancel_upload(file_id)` to abort an in-flight upload.
  - Implement `UploadProgress` with `progress: f32` and `UploadStatus` enum.
  - _Requirements: 4.1_
  - _writes: crates/collab_ui/src/channel_file_upload.rs_
  - _Completed: Added a collab UI `UploadManager` entity that reads local files off the foreground thread, requests presigned upload URLs, uploads through the client helper, confirms completed files, exposes per-channel upload progress, and supports cancelled/failed/completed upload states._
  - _Actual writes: crates/collab_ui/src/channel_file_upload.rs; crates/collab_ui/src/collab_ui.rs; crates/collab_ui/src/channel_chat.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `rustfmt --edition 2021 --check crates/collab_ui/src/channel_file_upload.rs`; `git diff --check`._

- [x] 9. Integrate `UploadManager` into the app
  - Initialize `UploadManager` as an `Entity<UploadManager>` in the app shared state.
  - Provide access to upload manager from channel contexts.
  - _Requirements: 4.1_
  - _writes: crates/collab_ui/src/app.rs_
  - _Completed: Registered `UploadManager` during collab UI initialization and exposed `UploadManager::global(cx)` so channel UI code can retrieve the shared upload manager entity._
  - _Actual writes: crates/collab_ui/src/channel_file_upload.rs; crates/collab_ui/src/collab_ui.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `rustfmt --edition 2024 --check crates/collab_ui/src/channel_file_upload.rs`; `git diff --check`._

### Phase 4: Client — Compose Area with Drag-and-Drop & File Picker

- [x] 10. Add drag-and-drop zone to the message compose area
  - Attach GPUI file-drop event handlers to the compose element.
  - Show a drop zone overlay with visual feedback on `on_drag_over` / `on_drag_enter`.
  - On `on_drop`, extract dropped file paths and call `UploadManager::upload_file`.
  - _Requirements: 4.1_
  - _writes: crates/collab_ui/src/channel_chat/compose_area.rs_
  - _Completed: Added `ExternalPaths` drag-over/drop handling to the channel composer, including drop-target background feedback and dropped-path upload dispatch through the shared `UploadManager`._
  - _Actual writes: crates/collab_ui/src/channel_chat.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `rustfmt --edition 2024 crates/collab_ui/src/channel_chat.rs crates/collab_ui/src/channel_file_upload.rs`; `git diff --check`._

- [x] 11. Add file attachment button and file picker
  - Add a paperclip / attach-file icon button to the compose toolbar.
  - On click, open a native file picker dialog (using GPUI file dialog support).
  - On file selection, call `UploadManager::upload_file`.
  - _Requirements: 4.1_
  - _writes: crates/collab_ui/src/channel_chat/compose_area.rs_
  - _Completed: Added a paperclip icon button to the composer controls that opens GPUI's multi-file picker and uploads selected files through `UploadManager`._
  - _Actual writes: crates/collab_ui/src/channel_chat.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `rustfmt --edition 2024 crates/collab_ui/src/channel_chat.rs crates/collab_ui/src/channel_file_upload.rs`; `git diff --check`._

- [x] 12. Show upload progress in the compose area
  - Render upload progress indicators (progress bars) below the compose input for active uploads.
  - Show filenames and progress percentage per file.
  - Handle failed uploads with inline error message and retry affordance.
  - Handle upload cancellation via the `cancel_upload` method.
  - _Requirements: 4.1_
  - _writes: crates/collab_ui/src/channel_chat/compose_area.rs_
  - _Completed: Rendered per-channel upload rows below the composer with filename, status text, progress bar, cancel, retry, and remove controls, and taught `UploadProgress` to retain the original file path for retry._
  - _Actual writes: crates/collab_ui/src/channel_chat.rs; crates/collab_ui/src/channel_file_upload.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `rustfmt --edition 2024 crates/collab_ui/src/channel_chat.rs crates/collab_ui/src/channel_file_upload.rs`; `git diff --check`._

### Phase 5: Client — File Preview Rendering

- [x] 13. Implement `FileAttachmentRenderer` with file kind detection
  - Create `FileAttachmentRenderer` struct.
  - Implement `detect_file_kind(mime_type, filename) -> FileKind` dispatching on MIME type / extension categories: `Image`, `Video`, `Audio`, `Pdf`, `Code`, `Other`.
  - Implement `render(file, window, cx) -> AnyElement` dispatching to per-kind renderers.
  - _Requirements: 4.2_
  - _writes: crates/collab_ui/src/channel_chat/file_renderer.rs_
  - _Completed: Added a reusable `FileAttachmentRenderer` with MIME/extension-based `FileKind` detection, render dispatch, remote image rendering, and file-card fallback rendering._
  - _Actual writes: crates/collab_ui/src/channel_chat/file_renderer.rs; crates/collab_ui/src/channel_chat.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `rustfmt --edition 2024 crates/collab_ui/src/channel_chat.rs crates/collab_ui/src/channel_chat/file_renderer.rs`; `git diff --check`._

- [ ] 14. Render image inline previews
  - Add `render_image_preview`: load and display image using existing `Image` element or `media` crate.
  - Support PNG, JPEG, GIF, WebP, SVG.
  - Click-to-open lightbox/gallery view for larger examination.
  - _Requirements: 4.2_
  - _writes: crates/collab_ui/src/channel_chat/file_renderer.rs_

- [ ] 15. Render PDF thumbnails
  - Add `render_pdf_thumbnail`: show a PDF icon thumbnail with filename and a "View PDF" link/button.
  - Open PDF in external viewer or existing PDF preview component on click.
  - _Requirements: 4.2_
  - _writes: crates/collab_ui/src/channel_chat/file_renderer.rs_

- [ ] 16. Render video and audio players
  - Add `render_video_player`: embed video element with playback controls.
  - Add `render_audio_player`: embed audio element with playback controls.
  - Use existing `media` crate functionality.
  - _Requirements: 4.2_
  - _writes: crates/collab_ui/src/channel_chat/file_renderer.rs_

- [ ] 17. Render code snippets with syntax highlighting
  - Add `render_code_snippet`: fetch file content, detect language from extension, render first N lines with syntax highlighting.
  - Add "Show more" expand action for files exceeding the preview line limit.
  - _Requirements: 4.2_
  - _writes: crates/collab_ui/src/channel_chat/file_renderer.rs_

- [x] 18. Render fallback file cards for other types
  - Add `render_file_card`: show file type icon, filename, formatted file size, uploader name, download count.
  - Click on filename triggers download.
  - _Requirements: 4.3_
  - _writes: crates/collab_ui/src/channel_chat/file_renderer.rs_
  - _Completed: Added fallback file cards with kind-specific icons, filename, formatted size, MIME type, uploader id, image/duration metadata when present, and a download action._
  - _Actual writes: crates/collab_ui/src/channel_chat/file_renderer.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `rustfmt --edition 2024 crates/collab_ui/src/channel_chat.rs crates/collab_ui/src/channel_chat/file_renderer.rs`; `git diff --check`._

### Phase 6: Client — Channel Message Integration

- [x] 19. Update channel message to display file attachments
  - Modify the `ChannelMessage` model to hold `Vec<FileAttachment>` from proto.
  - In channel message rendering, render text content then append `FileAttachmentRenderer` output for each file.
  - Ensure correct layout: text above file previews, visually distinct preview area, no extra spacing when only files.
  - _Requirements: 4.5_
  - _writes: crates/collab_ui/src/channel_chat/message.rs_
  - _Completed: Rendered proto `ChannelMessage.files` after message text in both main channel messages and thread replies by converting each proto attachment into the client `FileAttachment` model and dispatching to `FileAttachmentRenderer`._
  - _Actual writes: crates/collab_ui/src/channel_chat.rs; crates/collab_ui/src/channel_chat/file_renderer.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `cargo test -p collab_ui file_renderer --features collab_ui/test-support`; `rustfmt --edition 2024 --check crates/collab_ui/src/channel_chat.rs crates/collab_ui/src/channel_chat/file_renderer.rs`; `git diff --check`._

- [x] 20. Wire file attachments into message send flow
  - When composing a message, collect the `FileAttachment` handles from completed uploads.
  - Include file IDs in the send-message RPC payload.
  - Ensure attached files are sent alongside the message text in a single RPC.
  - _Requirements: 4.1_
  - _writes: crates/collab_ui/src/channel_chat/compose_area.rs_
  - _Completed: Added `file_ids` to the send-message request contract, attached confirmed uploads to newly-created channel messages on the server, and taught the channel composer to send completed upload IDs with the message while blocking in-flight uploads and clearing sent upload rows._
  - _Actual writes: crates/proto/proto/channel.proto; crates/client/src/channel_chat.rs; crates/client/src/client.rs; crates/collab/src/db/file_store.rs; crates/collab/src/rpc.rs; crates/collab/tests/integration/channel_chat_tests.rs; crates/collab/tests/integration/channel_chat_ui_tests.rs; crates/collab_ui/src/channel_chat.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo check -p proto -p client`; `CARGO_INCREMENTAL=0 cargo check -p collab --features collab/test-support`; `CARGO_INCREMENTAL=0 cargo check -p collab_ui --features collab_ui/test-support`; `git diff --check`._

### Phase 7: Testing

- [x] 21. Unit-test `FileStore` validation logic
  - Test `GenerateUploadUrl` rejects files exceeding `MaxFileSize`.
  - Test `GenerateUploadUrl` rejects MIME types not in `AllowedTypes`.
  - Test `GenerateUploadUrl` rejects empty filenames.
  - Test `ConfirmUpload` returns full `FileAttachment` on success.
  - Test `GetFileMetadata` returns saved metadata.
  - Test `DeleteFile` removes metadata record.
  - _Requirements: 4.4, Design §7_
  - _writes: crates/collab/src/storage/file_store_test.go_
  - _Completed: Added FileStore DB tests covering max-size, MIME allowlist, empty filename rejection, upload URL row creation, upload confirmation metadata, metadata lookup, attach-to-message, and message-file metadata deletion through a test-support URL-backed FileStore._
  - _Actual writes: crates/collab/src/db/file_store.rs; crates/collab/tests/integration/db_tests.rs; crates/collab/tests/integration/db_tests/file_store_tests.rs; crates/collab/src/rpc.rs; crates/proto/proto/sim.proto_
  - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests test_file_store_validation_sqlite --features test-support`; `CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests test_file_store_metadata_lifecycle_sqlite --features test-support`; `CARGO_INCREMENTAL=0 cargo check -p proto -p collab --features collab/test-support`; `git diff --check`._

- [x] 22. Integration-test upload lifecycle (server-side)
  - Test: upload request → generate URL → confirm → fetch message → verify `FileAttachment` present.
  - Test: upload oversized file returns 413 error.
  - Test: upload disallowed MIME type returns 415 error.
  - Test: confirm non-existent file ID returns appropriate error.
  - _Requirements: 4.1, 4.4, Design §7_
  - _writes: crates/collab/src/rpc/file_upload_test.go_
  - _Completed: Added a channel upload lifecycle RPC integration test covering oversized upload rejection, missing upload confirmation failure, upload URL generation, upload confirmation, send-message attachment wiring, and fetched message history retaining `FileAttachment` metadata; added test-server `GlobalClient` initialization and test-support FileStore routing for RPC tests without S3._
  - _Actual writes: crates/collab/src/db/file_store.rs; crates/collab/src/rpc.rs; crates/collab/tests/integration/channel_chat_tests.rs; crates/collab/tests/integration/test_server.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests test_channel_file_upload_lifecycle_rpc --features test-support`; `CARGO_INCREMENTAL=0 cargo check -p proto -p collab --features collab/test-support`; `git diff --check`._

- [x] 23. Integration-test file deletion on message deletion
  - Create message with file attachment, delete message, verify `channel_files` rows are cleaned up and S3 object is deleted (or scheduled for GC).
  - _Requirements: 4.5, Design Property 5.4_
  - _writes: crates/collab/src/rpc/channel_messages_test.go_
  - _Completed: Extended the channel file upload lifecycle RPC integration test to delete the attached message through the client RPC and verify the associated `channel_files` metadata row is removed._
  - _Actual writes: crates/collab/tests/integration/channel_chat_tests.rs_
  - _Validation: `CARGO_INCREMENTAL=0 cargo test -p collab --test collab_tests test_channel_file_upload_lifecycle_rpc --features test-support`; `CARGO_INCREMENTAL=0 cargo check -p proto -p collab --features collab/test-support`; `git diff --check`._

- [ ] 24. Unit-test client `UploadManager`
  - Test upload state machine: Pending → RequestingUrl → Uploading → Confirming → Complete.
  - Test cancellation transitions cleanly.
  - Test concurrent uploads are tracked separately.
  - _Requirements: 4.1_
  - _writes: crates/collab_ui/src/channel_file_upload.rs_ (add `#[cfg(test)] mod tests`)

- [ ] 25. UI test drag-and-drop in compose area
  - Simulate file-drop events on the compose area element.
  - Verify drop zone overlay appears on drag enter and disappears on drag leave.
  - Verify `UploadManager::upload_file` is invoked on drop.
  - _Requirements: 4.1, Design §7_
  - _writes: crates/collab_ui/src/channel_chat/compose_area.rs_ (add `#[cfg(test)] mod tests`)

- [ ] 26. UI test file picker button
  - Simulate click on attach-file button.
  - Verify file picker dialog is opened.
  - Mock file selection and verify upload is started.
  - _Requirements: 4.1, Design §7_
  - _writes: crates/collab_ui/src/channel_chat/compose_area.rs_

- [ ] 27. Test all file renderers
  - Test image preview renders correctly for supported formats.
  - Test PDF thumbnail renders with expected elements.
  - Test video/audio player renders controls.
  - Test code snippet renders with syntax highlighting.
  - Test file card shows filename, size, uploader, download count.
  - _Requirements: 4.2, 4.3, Design §7_
  - _writes: crates/collab_ui/src/channel_chat/file_renderer.rs_ (add `#[cfg(test)] mod tests`)

- [ ] 28. Security test presigned URL constraints
  - Verify presigned URL cannot upload files larger than the requested size.
  - Verify client cannot spoof MIME type after URL is generated (server-side enforcement on confirm).
  - _Requirements: 4.4, Design §7_
  - _writes: crates/collab/src/rpc/file_upload_test.go_ (add security test cases)

### Phase 8: Configuration & Final Wiring

- [ ] 29. Add server configuration for file storage limits
  - Add `FileStoreConfig` fields to server configuration: `MaxFileSize`, `AllowedTypes`, `StorageBucket`, `StoragePrefix`.
  - Wire configuration into `FileStore` initialisation.
  - _Requirements: 4.4_
  - _writes: crates/collab/src/config.go_

- [ ] 30. Verify thumbnail generation for images
  - When an image is uploaded via `ConfirmUpload`, generate a thumbnail (max 400px wide) server-side and store its path.
  - Serve the thumbnail URL for inline previews in the channel.
  - _Requirements: 4.2, Design Property 5.5_
  - _writes: crates/collab/src/storage/file_store.go_ (add thumbnail generation)

- [ ] 31. Add download count tracking
  - Add `download_count` column to `channel_files` (if not already present in migration).
  - Increment counter when a file is downloaded.
  - Display download count in file card renderer.
  - _Requirements: 4.3_
  - _writes: crates/collab/src/storage/file_store.go_
  - _writes: crates/collab_ui/src/channel_chat/file_renderer.rs_

- [ ] 32. End-to-end smoke test
  - Spin up dev environment with S3-compatible storage (e.g. MinIO).
  - Upload a file via the UI, verify it appears as an inline preview.
  - Upload an image, verify thumbnail is shown.
  - Delete the message, verify file is no longer accessible.
  - Attempt to upload a file exceeding the limit, verify error.
  - _Requirements: 4.1–4.5, Design §7_
