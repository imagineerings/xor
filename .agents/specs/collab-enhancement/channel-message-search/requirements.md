# Requirements: Message Search in Channels

## Introduction

Baymax channels currently provide no mechanism to search past messages. Mattermost provides full-text search across messages with filters for channels, users, dates, and content types (`posts.yaml` API). Adding message search will make channels useful as a knowledge base where past discussions can be retrieved.

## Glossary

- **Full-Text Search**: Search across the content of all channel messages
- **Search Index**: An index maintained for efficient text search queries
- **Search Filter**: Criteria to narrow search results (channel, user, date range, etc.)
- **Search Result**: A matched message with context showing surrounding content

## Requirements

### Requirement 5.1: Full-Text Message Search

**User Story:** As a channel participant, I want to search across all channel messages I have access to, so that I can find past discussions and decisions.

#### Acceptance Criteria

1. WHEN a user enters a search query in the channel search bar THEN THE system SHALL return messages matching the query text
2. THE system SHALL search across all channels the user is a member of
3. THE system SHALL support case-insensitive search
4. THE system SHALL support partial word matching (prefix search)
5. WHEN there are no matches THEN THE system SHALL display a "No results found" message with suggestions

### Requirement 5.2: Search Filters

**User Story:** As a channel participant, I want to filter search results, so that I can narrow down to relevant messages.

#### Acceptance Criteria

1. THE system SHALL support filtering by specific channel (`in:channel-name`)
2. THE system SHALL support filtering by specific user (`from:username`)
3. THE system SHALL support filtering by date range (`before:2024-01-01 after:2023-01-01`)
4. THE system SHALL support combining multiple filters in a single query
5. THE system SHALL support quoted strings for exact phrase matching

### Requirement 5.3: Search Results Display

**User Story:** As a channel participant, I want to see search results with context, so that I can understand where the match occurs.

#### Acceptance Criteria

1. WHEN search results are displayed THEN THE system SHALL show each message with surrounding context (previous/next messages)
2. WHEN search results are displayed THEN THE matched text SHALL be highlighted within each result
3. WHEN a user clicks a search result THEN THE system SHALL navigate to that message in the channel and scroll to its position
4. THE system SHALL show the channel name and timestamp for each result
5. THE system SHALL paginate results (e.g., 20 per page) with a "Load more" option

### Requirement 5.4: Search Indexing

**User Story:** As a system administrator, I want message search to be efficient, so that queries return quickly even with large message volumes.

#### Acceptance Criteria

1. THE system SHALL maintain a search index of all channel messages
2. WHEN a new message is sent THEN THE system SHALL update the index within a configurable delay (near real-time)
3. WHEN a message is edited THEN THE system SHALL update the index with the new content
4. WHEN a message is deleted THEN THE system SHALL remove it from the index
5. THE system SHALL support rebuilding the search index from scratch if needed
