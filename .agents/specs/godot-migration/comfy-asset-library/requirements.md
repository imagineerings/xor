# Requirements: Comfy Asset Library

## Introduction

Sim needs Comfy's asset library behavior for generated outputs, uploads, model references, user files, tags, metadata, previews, and filesystem scans. This asset behavior is core world-model harness functionality because generated outputs, model references, and workflow files must be discoverable and reusable across harness jobs. This spec owns asset indexing and API semantics. It delegates media preview rendering to `rendering-media/`, model folder registration to `comfy-model-memory-runtime/`, and generated artifact provenance to shared world-model artifact models.

## Glossary

- **Asset**: Content identified by a hash, size, MIME type, and creation timestamp.
- **Asset Reference**: A user-facing or filesystem-facing record that points to asset content and carries name, tags, preview id, metadata, owner, and cache state.
- **Asset Seed Scan**: Background synchronization of model, input, and output roots into the asset database.
- **User Data**: Per-user settings, workflow files, and UI state stored under Sim user storage.
- **Cache State**: File path, mtime, missing status, verify flag, and enrichment level for filesystem-backed references.

## Requirements

### Requirement 1: Asset Data Model

**User Story:** As a user, I want generated and uploaded files tracked as assets so they can be searched, reused, downloaded, and attached to workflows.

#### Acceptance Criteria

1.1 WHEN a file is registered THEN THE system SHALL create or reuse content by hash and create an owner-scoped asset reference.
1.2 WHEN an asset reference is created THEN THE system SHALL store name, tags, preview id, user metadata, system metadata, job id, timestamps, and cache state.
1.3 IF multiple references share the same content hash THEN THE system SHALL preserve separate reference metadata without duplicating content metadata.

### Requirement 2: Asset API Operations

**User Story:** As a frontend or API client, I want CRUD, upload, download, tag, and hash operations for assets.

#### Acceptance Criteria

2.1 WHEN a client lists assets THEN THE system SHALL support owner scoping, include tags, exclude tags, name contains, metadata filters, offset pagination, cursor pagination, sort, and order.
2.2 WHEN a client uploads an asset THEN THE system SHALL accept multipart files, optional known hash, tags, name, MIME type, user metadata, and preview id.
2.3 WHEN a client creates an asset from an existing hash THEN THE system SHALL create a new reference only if content already exists.
2.4 WHEN a client downloads asset content THEN THE system SHALL stream the file with safe content type and content disposition.
2.5 WHEN a client updates or deletes an asset reference THEN THE system SHALL enforce owner access and preserve shared content unless explicitly orphan-cleaned by policy.

### Requirement 3: Tags and Metadata

**User Story:** As a user, I want assets organized by tags and metadata so model files and generated outputs are discoverable.

#### Acceptance Criteria

3.1 WHEN tags are added or removed THEN THE system SHALL report added, already-present, removed, missing, and total tag counts.
3.2 WHEN tags are listed THEN THE system SHALL support prefix, limit, offset, order, include-zero, and owner filters.
3.3 WHEN tag refinement is requested THEN THE system SHALL return a histogram for the current asset filter.
3.4 WHEN metadata filters are supplied THEN THE system SHALL support string, number, boolean, and JSON metadata fields.

### Requirement 4: Asset Seeding and Enrichment

**User Story:** As a maintainer, I want Sim to synchronize filesystem roots into assets without blocking normal generation.

#### Acceptance Criteria

4.1 WHEN the asset system starts THEN THE system SHALL optionally seed models, input, and output roots in the background.
4.2 WHEN a scan is running THEN THE system SHALL expose scanned, total, created, skipped, state, and errors.
4.3 WHEN scan cancellation is requested THEN THE system SHALL ask the running scan to stop and preserve already indexed rows.
4.4 WHEN prune is requested THEN THE system SHALL mark references outside known roots as missing without deleting content.
4.5 WHEN output files are generated THEN THE system SHALL register them and enqueue enrichment for output metadata and optional hashes.

### Requirement 5: User Data and Settings

**User Story:** As a Comfy-compatible frontend user, I want server-side user profiles, settings, and workflow files.

#### Acceptance Criteria

5.1 WHEN multi-user mode is enabled THEN THE system SHALL resolve user storage from request user identity and reject internal system users.
5.2 WHEN user data is listed THEN THE system SHALL support recursive listing, structured v2 listing, file info, and path splitting.
5.3 WHEN user data is created, read, moved, or deleted THEN THE system SHALL confine paths to that user's public storage root.
5.4 WHEN settings are read or written THEN THE system SHALL store them in the user's server-side settings file.

### Requirement 6: Sim Integration Boundary

**User Story:** As a Sim developer, I want the Comfy asset library to reuse Sim's storage and media systems.

#### Acceptance Criteria

6.1 IF Sim already has artifact, media preview, user storage, or secret infrastructure THEN THE asset library SHALL adapt those systems rather than adding a parallel storage stack.
6.2 WHEN asset records reference generated outputs THEN THE system SHALL include shared provenance identifiers.
6.3 IF the asset database is unavailable while assets are required THEN THE system SHALL fail startup with actionable diagnostics.
