# Design: Channel Join Requests

## 1. Overview

Sim private channels (`ChannelVisibility::Members`) currently require an explicit invitation from a channel admin or member to join. Users who discover a private channel (through the channel tree or a shared link) see no path to request access — they must find an existing member to invite them. This design adds a join request workflow, enabling users to request access to private channels and channel admins to approve or deny those requests through the existing notification and member management systems.

**Key decisions:**

- **Proto changes**: New `RequestJoinChannel` and `RespondToJoinRequest` RPC messages, new `JoinRequestAdded` and `JoinRequestResponded` push messages. The existing `UpdateChannels` message gains a `pending_request_counts` field for state sync. <!-- impl: crates/proto/proto/channel.proto#UpdateChannels -->
- **Database**: New `channel_join_requests` table keyed by `(channel_id, user_id)`, with an optional `reason` text field and a `created_at` timestamp. A background expiry job handles auto-rejection after a configurable TTL (default 7 days).
- **Only Admins can respond**: Following the principle of least privilege, only users with `ChannelRole::Admin` on the target channel can approve or deny join requests. (Future extension: grant this power to members with a "Manage" permission.)
- **Notifications reuse**: New `Notification` enum variants (`JoinRequest`, `JoinRequestApproved`, `JoinRequestDenied`) follow the existing serde-based notification pattern in `rpc::Notification`. The notification store loads the requester by ID for admin-facing join requests; approval and denial notifications render the channel name and any supplied denial reason. <!-- impl: crates/rpc/src/notification.rs#Notification -->
- **UI integration**: The "Request to Join" button replaces channel content for non-members viewing a private channel. The admin review UI lives inside the existing channel member management modal.
- **Duplicate prevention**: The database provides a unique constraint on `(channel_id, user_id)` for active (pending) requests. The server also checks for existing pending requests before inserting.

## 2. Architecture

```mermaid
flowchart TB
    subgraph Client_A[Requester Client]
        A[Channel Tree] -->|click private channel| B[RequestToJoinPanel]
        B -->|fill reason (optional)| C[RequestJoinChannel RPC]
        D[Notification Store] -->|JoinRequestResponse push| E[Toast + Notification]
        E -->|click notification| F[Open Channel View]
    end

    subgraph Server
        C --> G[collab: handle_request_join]
        G --> H[(channel_join_requests)]
        G --> I[Broadcast JoinRequestAdded to admins]
        
        J[collab: handle_respond_join_request] --> K{Approve?}
        K -->|yes| L[channel_member insert]
        K -->|no| M[Delete from channel_join_requests]
        L --> N[Broadcast JoinRequestResponded to requester]
        M --> N
        
        O[Background expiry job] -->|scan expired| P[Delete expired rows + notify]
    end

    subgraph Client_B[Admin Client]
        Q[Notification Store] -->|JoinRequestAdded push| R[Notification in tray]
        S[Channel Member Management] -->|open pending tab| T[GetPendingJoinRequests RPC]
        U[Pending Requests List] -->|click request| V[Request Detail Panel]
        V -->|approve| W[RespondToJoinRequest RPC]
        V -->|deny| W
    end
```

### Components

| Component | Responsibility |
|---|---|
| `RequestToJoinPanel` | UI shown to non-members of a private channel; contains "Request to Join" button and optional reason field |
| `PendingRequestsStore` (client) | Caches pending join requests for the current user's admin channels; syncs via `UpdateChannels` |
| `PendingRequestList` | UI component in the member management modal listing all pending requests with timestamps |
| `RequestDetailPanel` | Detail view for a single request showing requester profile, reason, and Approve/Deny buttons |
| `JoinRequestStore` (server) | CRUD for join requests in the database; handles expiry, duplicate detection |
| `Notification variants` | New `JoinRequest`, `JoinRequestApproved`, `JoinRequestDenied` notification types |

## 3. Components and Interfaces

### 3.1 Protobuf Changes

```protobuf
// ---- RPCs (request/response) ----

// Sent by a user requesting to join a private channel.
message RequestJoinChannel {
    uint64 channel_id = 1;
    optional string reason = 2;  // optional context for the admin
}

message RequestJoinChannelResponse {
    bool success = 1;
}

// Sent by an admin approving or denying a join request.
message RespondToJoinRequest {
    uint64 channel_id = 1;
    uint64 requesting_user_id = 2;
    bool approve = 3;
    optional string denial_reason = 4;  // optional reason sent to the user on denial
}

message RespondToJoinRequestResponse {
    bool success = 1;
}

// ---- Entity messages (for Get* pattern) ----

message GetPendingJoinRequests {
    uint64 channel_id = 1;
}

message GetPendingJoinRequestsResponse {
    repeated PendingJoinRequest requests = 1;
}

message PendingJoinRequest {
    uint64 user_id = 1;
    optional string reason = 2;
    uint64 created_at = 3;  // unix timestamp
}

// ---- Push messages (broadcast) ----

// Pushed to all admins of a channel when a new join request arrives.
message JoinRequestAdded {
    uint64 channel_id = 1;
    uint64 requesting_user_id = 2;
    optional string reason = 3;
    uint64 created_at = 4;
}

// Pushed to a user when their join request is approved or denied.
message JoinRequestResponded {
    uint64 channel_id = 1;
    bool approved = 2;
    optional string denial_reason = 3;
}

// ---- State sync ----

// Extend UpdateChannels to include pending request counts.
// (Add as a new field on the existing UpdateChannels message)
message UpdateChannels {
    // ...existing fields...
    repeated PendingRequestCount pending_request_counts = 16;  // [channel_id → count] for admin'd channels
}

message PendingRequestCount {
    uint64 channel_id = 1;
    uint32 count = 2;
}
```

### 3.2 Database Schema

```sql
-- New table for join requests
CREATE TABLE channel_join_requests (
    id BIGINT PRIMARY KEY GENERATED ALWAYS AS IDENTITY,
    channel_id BIGINT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    -- Enforce at most one pending request per (channel, user)
    UNIQUE (channel_id, user_id)
);

-- Index for admin queries: find all requests for a channel
CREATE INDEX idx_join_requests_channel ON channel_join_requests (channel_id);

-- Index for expiry queries: find all requests older than TTL
CREATE INDEX idx_join_requests_created_at ON channel_join_requests (created_at);
```

### 3.3 Server-side: JoinRequestStore

```go
type JoinRequestStore struct {
    db *sqlx.DB
}

// RequestJoin creates a new pending join request. Returns error if a pending
// request already exists (unique constraint violation).
func (s *JoinRequestStore) RequestJoin(channelID, userID uint64, reason string) error

// PendingRequestExists checks if a user already has a pending request for a channel.
func (s *JoinRequestStore) PendingRequestExists(channelID, userID uint64) (bool, error)

// ApproveJoinRequest approves a pending request. Deletes the request row and
// creates a channel_member row. Returns the requesting user_id.
func (s *JoinRequestStore) ApproveJoinRequest(channelID, userID uint64) error

// DenyJoinRequest denies a pending request. Deletes the request row.
func (s *JoinRequestStore) DenyJoinRequest(channelID, userID uint64) error

// GetPendingRequests returns all pending join requests for a channel.
func (s *JoinRequestStore) GetPendingRequests(channelID uint64) ([]PendingJoinRequest, error)

// ExpireOldRequests deletes (and returns) all requests older than the given threshold.
func (s *JoinRequestStore) ExpireOldRequests(threshold time.Time) ([]ExpiredRequest, error)

// CountPendingRequests returns the number of pending requests for a channel.
func (s *JoinRequestStore) CountPendingRequests(channelID uint64) (uint32, error)
```

The production implementation uses a SeaORM-backed `JoinRequestStore`. Approval deletes the pending request and writes an accepted `Member` membership in one transaction; an existing invitation or ban is restored to that membership state. Expiry returns the associated channel name for notification delivery. <!-- impl: crates/collab/src/db/join_request_store.rs#JoinRequestStore -->

**Rust (collab crate) integration** — the store methods are called from the RPC handler in `collab/src/api/channel.rs`:

```rust
// In the RPC dispatch
async fn handle_request_join(
    &self,
    mut request: TypedEnvelope<proto::RequestJoinChannel>,
    cx: &mut RpcSession,
) -> Result<proto::RequestJoinChannelResponse> {
    let user_id = cx.user_id()?;
    let channel_id = request.payload.channel_id.into();

    // 1. Verify channel exists and is private
    let channel = self.db.get_channel(channel_id).await?;
    if channel.visibility != ChannelVisibility::Members {
        Err(anyhow!("Cannot request to join a public channel — join directly"))?;
    }

    // 2. Verify user is not already a member
    let role = self.db.channel_role_for_user(&channel, user_id).await?;
    if role.is_some() && role != Some(ChannelRole::Banned) {
        Err(anyhow!("You are already a member of this channel"))?;
    }

    // 3. Check for duplicate pending request
    let existing = self.db.pending_join_request_exists(channel_id, user_id).await?;
    if existing {
        Err(ErrorCode::AlreadyJoinRequested.anyhow())?;
    }

    // 4. Insert request
    self.db.request_join_channel(channel_id, user_id, request.payload.reason).await?;

    // 5. Notify all admins of the channel
    let admins = self.db.get_channel_admins(channel_id).await?;
    for admin_id in admins {
        self.notification_store
            .create_notification(
                admin_id,
                Notification::JoinRequest {
                    channel_id: channel_id.0,
                    channel_name: channel.name.clone(),
                    requesting_user_id: user_id.0,
                    requesting_user_name: ...,
                    reason: request.payload.reason,
                },
            )
            .await?;
    }

    Ok(proto::RequestJoinChannelResponse { success: true })
}
```

The production handler validates private-channel eligibility before using `JoinRequestStore`; it then creates notifications for accepted channel admins and pushes `JoinRequestAdded` to their active connections. <!-- impl: crates/collab/src/rpc.rs#request_join_channel -->

### 3.4 Server-side: RPC Handler for RespondToJoinRequest

```rust
async fn handle_respond_join_request(
    &self,
    mut request: TypedEnvelope<proto::RespondToJoinRequest>,
    cx: &mut RpcSession,
) -> Result<proto::RespondToJoinRequestResponse> {
    let admin_id = cx.user_id()?;
    let channel_id: ChannelId = request.payload.channel_id.into();
    let requesting_user_id: UserId = request.payload.requesting_user_id.into();

    // 1. Verify the responding user is an admin of this channel
    let channel = self.db.get_channel(channel_id).await?;
    self.db.check_user_is_channel_admin(&channel, admin_id).await?;

    // 2. Verify the request still exists
    let request_exists = self.db.pending_join_request_exists(channel_id, requesting_user_id).await?;
    if !request_exists {
        Err(anyhow!("This join request no longer exists (it may have been already handled or expired)"))?;
    }

    if request.payload.approve {
        // 3a. Approve: add as member, delete request
        self.db.approve_join_request(channel_id, requesting_user_id).await?;

        // Notify the requester
        self.notification_store
            .create_notification(
                requesting_user_id,
                Notification::JoinRequestApproved {
                    channel_id: channel_id.0,
                    channel_name: channel.name.clone(),
                },
            )
            .await?;
    } else {
        // 3b. Deny: delete request
        self.db.deny_join_request(channel_id, requesting_user_id).await?;

        // Notify the requester
        self.notification_store
            .create_notification(
                requesting_user_id,
                Notification::JoinRequestDenied {
                    channel_id: channel_id.0,
                    channel_name: channel.name.clone(),
                    reason: request.payload.denial_reason,
                },
            )
            .await?;
    }

    Ok(proto::RespondToJoinRequestResponse { success: true })
}
```

The production handler verifies the responder is a channel admin before resolving the pending request; it then notifies and pushes the outcome to the requester. <!-- impl: crates/collab/src/rpc.rs#respond_to_join_request -->

### 3.5 Server-side: Background Expiry Job

```rust
// Run periodically (e.g., every hour) to expire stale requests.
pub async fn expire_join_requests(db: &Database) -> Result<()> {
    let ttl = std::env::var("CHANNEL_JOIN_REQUEST_TTL_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7 * 24 * 60 * 60); // default 7 days

    let threshold = Utc::now() - chrono::Duration::seconds(ttl);
    let expired = db.expire_join_requests(threshold).await?;

    for request in expired {
        // Notify the requester that their request expired
        db.create_notification(
            request.user_id,
            Notification::JoinRequestDenied {
                channel_id: request.channel_id.0,
                channel_name: request.channel_name,
                reason: Some("Your join request has expired.".into()),
            },
        ).await?;
    }

    Ok(())
}
```

The collab RPC server starts this job. It runs immediately and then hourly; invalid or negative `CHANNEL_JOIN_REQUEST_TTL_SECS` values fall back to the seven-day default. Each expiry also sends the new zero count to connected channel admins. <!-- impl: crates/collab/src/rpc.rs#run_join_request_expiry_loop -->

### 3.6 Client-side: RequestToJoinPanel

```rust
pub struct RequestToJoinPanel {
    channel_id: ChannelId,
    reason: SharedString,       // editable text, empty by default
    state: RequestState,        // Idle | Sending | Sent | AlreadyRequested | Error
}

enum RequestState {
    Idle,
    Sending,
    Sent,
    AlreadyRequested,
    Error(SharedString),
}

impl RequestToJoinPanel {
    /// Renders the "Request to Join" UI in place of channel content.
    pub fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self.state {
            RequestState::Sent => {
                // Show: "Join request sent. You'll be notified when a channel admin responds."
                div().child(self.sent_message())
            }
            RequestState::AlreadyRequested => {
                // Show: "You have already requested to join this channel."
                div().child(self.pending_message())
            }
            RequestState::Error(msg) => {
                // Show the error with retry button
                div().child(self.error_view(msg, window, cx))
            }
            _ => {
                // Show the reason text field + "Request to Join" button
                div()
                    .child(self.reason_input())
                    .child(self.submit_button(window, cx))
            }
        }
    }

    fn submit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.state = RequestState::Sending;
        let channel_id = self.channel_id;
        let reason = self.reason.clone();
        cx.spawn(async move |this, cx| {
            let result = cx
                .update(|cx| {
                    // Access the client via entity store or global
                    // ...
                })
                .unwrap()
                .request(proto::RequestJoinChannel {
                    channel_id: channel_id.0,
                    reason: if reason.is_empty() { None } else { Some(reason.to_string()) },
                })
                .await;

            this.update(cx, |this, _| {
                match result {
                    Ok(_) => this.state = RequestState::Sent,
                    Err(e) => this.state = RequestState::Error(e.to_string().into()),
                }
            }).ok();
        }).detach();
    }
}
```

**Integration point**: When a user clicks a private channel in the `CollabPanel` that they do not have an accepted membership for, instead of opening `ChannelView`, the workspace shows `RequestToJoinPanel`. The `ChannelStore` already tracks channel role — when `role` is `None`, the panel is shown.

### 3.7 Client-side: Pending Requests List (in Member Management)

The channel member management modal (existing) gains a new tab for pending join requests:

```rust
pub struct PendingRequestsList {
    channel_id: ChannelId,
    requests: Vec<PendingRequestViewModel>,
    loading: bool,
    badge_count: u32,
}

pub struct PendingRequestViewModel {
    user: Arc<User>,
    reason: Option<SharedString>,
    created_at: OffsetDateTime,
}

impl PendingRequestsList {
    pub fn new(channel_id: ChannelId, cx: &mut App) -> Self;

    pub fn render(&mut self, window: &mut Window, cx: &mut App) -> AnyElement {
        // Tab header with badge: "Pending Requests (3)"
        // List of requests with: avatar, name, timestamp, reason (if any)
        // Each item has an "open detail" action
    }

    fn load_requests(&mut self, cx: &mut Context<Self>) -> Task<Result<()>> {
        let channel_id = self.channel_id;
        let client = // ... get client
        cx.spawn(async move |this, cx| {
            let response = client
                .request(proto::GetPendingJoinRequests {
                    channel_id: channel_id.0,
                })
                .await?;

            this.update(cx, |this, _| {
                this.requests = response.requests.into_iter().map(...).collect();
                this.loading = false;
            }).ok();
            Ok(())
        })
    }
}
```

### 3.8 Client-side: Request Detail Panel

A clickable detail view (inline expand or slide-over) showing:

- Requester's profile avatar, name, and username
- Request timestamp (formatted as relative time, e.g., "2 hours ago")
- Reason text (or "No reason provided" placeholder)
- **Approve** and **Deny** buttons (with optional denial reason field on deny)

```rust
pub struct RequestDetailPanel {
    request: PendingRequestViewModel,
    show_denial_input: bool,
    denial_reason: SharedString,
}

impl RequestDetailPanel {
    pub fn approve(&mut self, cx: &mut Context<Self>) {
        let channel_id = self.channel_id;
        let user_id = self.request.user.id;
        let client = // ... get client
        cx.spawn(async move |this, cx| {
            client
                .request(proto::RespondToJoinRequest {
                    channel_id: channel_id.0,
                    requesting_user_id: user_id.to_proto(),
                    approve: true,
                    denial_reason: None,
                })
                .await?;

            this.update(cx, |this, cx| {
                // Remove from list
                cx.notify();
            }).ok();
            Ok(())
        }).detach();
    }

    pub fn deny(&mut self, cx: &mut Context<Self>) {
        let channel_id = self.channel_id;
        let user_id = self.request.user.id;
        let reason = self.denial_reason.clone();
        let client = // ... get client
        cx.spawn(async move |this, cx| {
            client
                .request(proto::RespondToJoinRequest {
                    channel_id: channel_id.0,
                    requesting_user_id: user_id.to_proto(),
                    approve: false,
                    denial_reason: if reason.is_empty() { None } else { Some(reason.to_string()) },
                })
                .await?;

            this.update(cx, |this, cx| {
                // Remove from list
                cx.notify();
            }).ok();
            Ok(())
        }).detach();
    }
}
```

### 3.9 Notification Integration

New variants on the `rpc::Notification` enum:

```rust
// In crates/rpc/src/notification.rs
#[derive(Debug, Clone, PartialEq, Eq, VariantNames, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Notification {
    // ...existing variants...
    ContactRequest { sender_id: u64 },
    ContactRequestAccepted { responder_id: u64 },
    ChannelInvitation { channel_id: u64, channel_name: String, inviter_id: u64 },

    // New variants for join requests
    JoinRequest {
        #[serde(rename = "entity_id")]
        channel_id: u64,
        channel_name: String,
        requesting_user_id: u64,
        requesting_user_name: String,
        reason: Option<String>,
    },
    JoinRequestApproved {
        #[serde(rename = "entity_id")]
        channel_id: u64,
        channel_name: String,
    },
    JoinRequestDenied {
        #[serde(rename = "entity_id")]
        channel_id: u64,
        channel_name: String,
        reason: Option<String>,
    },
}
```

**Notification content rendering** (in the UI notification center):

| Notification Kind | Title | Body | Action |
|---|---|---|---|
| `JoinRequest` | "Join Request" | "{username} wants to join #{channel}" | Opens member management for that channel |
| `JoinRequestApproved` | "Request Approved" | "You've been added to #{channel}" | Click navigates to the channel |
| `JoinRequestDenied` | "Request Denied" | "Your request to join #{channel} was denied" + reason (if provided) | Dismiss |

**Notification Store subscription** — the existing `add_notifications` handler in `notification_store.rs` processes user_ids from the new variants:

```rust
// In NotificationStore::add_notifications
for entry in &notifications {
    match entry.notification {
        Notification::JoinRequest { requesting_user_id, .. } => {
            user_ids.push(requesting_user_id);
        }
        // JoinRequestApproved and JoinRequestDenied don't introduce new user IDs
        // that aren't already known (channel_id is entity_id)
        _ => {}
    }
}
```

### 3.10 Channel Store Updates

The `ChannelStore` gains tracking for pending request counts:

```rust
// In channel_store.rs
pub struct ChannelStore {
    // ...existing fields...
    pending_join_request_counts: HashMap<ChannelId, u32>,  // only for admin'd channels
}

impl ChannelStore {
    /// Returns the number of pending join requests for a channel the user admins.
    pub fn pending_request_count(&self, channel_id: ChannelId) -> u32 {
        self.pending_join_request_counts.get(&channel_id).copied().unwrap_or(0)
    }

    fn handle_update_channels(&mut self, message: proto::UpdateChannels, cx: &mut Context<Self>) {
        // ...existing handling...

        // Sync pending request counts
        for count in message.pending_request_counts {
            if count.count == 0 {
                self.pending_join_request_counts.remove(&ChannelId(count.channel_id));
            } else {
                self.pending_join_request_counts.insert(ChannelId(count.channel_id), count.count);
            }
        }
    }
}
```

## 4. Data Models

### 4.1 Server-side (database)

```
channel_join_requests
├── id            BIGINT PRIMARY KEY (generated)
├── channel_id    BIGINT NOT NULL (FK → channels.id, ON DELETE CASCADE)
├── user_id       BIGINT NOT NULL (FK → users.id, ON DELETE CASCADE)
├── reason        TEXT NULL
├── created_at    TIMESTAMP NOT NULL DEFAULT NOW()
└── UNIQUE (channel_id, user_id)
```

### 4.2 Client-side (Rust structs)

```rust
/// Client-side representation of a pending join request for display.
pub struct PendingJoinRequest {
    pub user_id: u64,
    pub user: Arc<User>,            // resolved via UserStore
    pub reason: Option<SharedString>,
    pub created_at: OffsetDateTime,
}

/// State maintained by the ChannelStore for badge display.
pub struct PendingRequestCount {
    pub channel_id: ChannelId,
    pub count: u32,
}
```

### 4.3 Proto messages summary

| Proto Message | Direction | Purpose |
|---|---|---|
| `RequestJoinChannel` | Client→Server | User requests to join a private channel |
| `RequestJoinChannelResponse` | Server→Client | Acknowledges the request was recorded |
| `RespondToJoinRequest` | Admin→Server | Admin approves/denies a join request |
| `RespondToJoinRequestResponse` | Server→Admin | Acknowledges the response was processed |
| `GetPendingJoinRequests` | Admin→Server | Fetches pending requests for a channel |
| `GetPendingJoinRequestsResponse` | Server→Admin | List of pending requests with user IDs |
| `JoinRequestAdded` | Server→All Admins | Push: new join request arrived |
| `JoinRequestResponded` | Server→Requester | Push: request was approved or denied |
| `PendingRequestCount` | Server→All Admins | Synced via `UpdateChannels` for badge state |

## 5. Correctness Properties

### Property 5.1: Duplicate request prevention

_For any_ `(channel_id, user_id)` pair where a pending `channel_join_requests` row already exists, calling `RequestJoinChannel` SHALL return an error with code `AlreadyJoinRequested`.

**Validates: Requirement 10.1 (AC 4)**

### Property 5.2: Admin-only response authority

_For any_ `RespondToJoinRequest` call where the caller is NOT a channel admin (role is not `ChannelRole::Admin`) for the given `channel_id`, the server SHALL return a `Forbidden` error and SHALL NOT modify any state.

**Validates: Requirement 10.2 (AC 1)**

### Property 5.3: Membership consistency on approval

_For any_ approved join request, the server SHALL atomically delete the `channel_join_requests` row and insert a `channel_members` row (with `accepted = true`, `role = Member`) in a single database transaction.

**Validates: Requirement 10.2 (AC 3)**

### Property 5.4: Notification delivery on outcome

_For any_ approved or denied join request, the requesting user SHALL receive exactly one `Notification` of kind `JoinRequestApproved` or `JoinRequestDenied` respectively. The notification SHALL be persisted in the `notifications` table and SHALL be pushed via WebSocket to the user's connected clients.

**Validates: Requirement 10.3 (AC 1, AC 2)**

### Property 5.5: Notification click navigates to channel

_For any_ `JoinRequestApproved` notification clicked by the user, the system SHALL open the channel view for the `channel_id` referenced in the notification.

**Validates: Requirement 10.3 (AC 3)**

### Property 5.6: Badge accuracy

_For any_ channel admin viewing the member management UI, the badge count on the "Pending Requests" tab SHALL equal the number of rows in `channel_join_requests` for that channel where `created_at > now - TTL`.

**Validates: Requirement 10.4 (AC 1)**

### Property 5.7: Request auto-expiry

_For any_ `channel_join_requests` row where `created_at + TTL < now`, the background expiry job SHALL delete the row and create a `JoinRequestDenied` notification for the requesting user with reason "Your join request has expired."

**Validates: Requirement 10.4 (AC 3)**

### Property 5.8: Public channel bypass

_For any_ `RequestJoinChannel` call where the target channel has `visibility = Public`, the server SHALL return an error explaining that public channels can be joined directly, and SHALL NOT create a join request.

**Validates: Requirement 10.1 (implicit — only private channels need requests)**

### Property 5.9: Already-member prevention

_For any_ `RequestJoinChannel` call where the requesting user is already an accepted member of the target channel, the server SHALL return an error and SHALL NOT create a join request.

**Validates: Requirement 10.1 (AC 1)**

## 6. Error Handling

| Error | Handling |
|---|---|
| User already a member of the channel | Return error from `RequestJoinChannel`; show toast "You are already a member of this channel" |
| Duplicate pending request | Return `AlreadyJoinRequested` error; show toast "You have already requested to join this channel" |
| Channel is public | Return error; show toast "This channel is public — you can join directly" instead of showing the request panel |
| Request no longer exists on respond | Return error to admin; refresh the pending requests list (another admin already handled it) |
| Not an admin trying to respond | Return `Forbidden` error; this should not happen in normal UI flow (admin-only UI) |
| Network failure on request | Retry with exponential backoff (3 attempts). If all fail, show error state in `RequestToJoinPanel` with a "Try Again" button |
| Network failure on admin response | Retry with exponential backoff. Show toast failure: "Failed to respond to join request" |
| Channel deleted while request is pending | Cascade delete handles cleanup; admin sees empty list; requester gets no notification (channel no longer exists) |
| Requesting user deleted while request is pending | Cascade delete handles cleanup |
| Reason text too long | Server enforces max 500 characters; client shows character counter and truncates at limit |

## 7. Testing Strategy

### Unit tests

- **JoinRequestStore**: `request_join`, `approve_join_request`, `deny_join_request`, `get_pending_requests`, `expire_old_requests` — verify correct SQL, unique constraint enforcement, and cascade behavior.
- **Duplicate prevention**: Call `request_join` twice for the same `(channel_id, user_id)` and verify the second call returns an error.
- **Approval creates member**: Approve a request and verify a `channel_members` row was created with the correct role and `accepted = true`.

### Integration tests

- **Full request flow**: Client A (non-member) calls `RequestJoinChannel` → verify `JoinRequestAdded` is pushed to admin Client B → admin Client B calls `GetPendingJoinRequests` → verify request appears → admin Client B calls `RespondToJoinRequest(approve=true)` → verify Client A receives `JoinRequestApproved` → verify Client A can now join the channel.
- **Denial flow**: Same as above but with `approve=false` → verify Client A receives `JoinRequestDenied` → verify Client A still cannot join.
- **Expiry flow**: Create a request with a past timestamp → run expiry job → verify notification is created and request is deleted.
- **Admin-only authority**: Non-admin user calls `RespondToJoinRequest` → verify `Forbidden` error.

### UI tests (GPUI)

- **RequestToJoinPanel rendering**: Verify the panel shows the "Request to Join" button and reason field in `Idle` state; verify it shows the confirmation message in `Sent` state; verify it shows the "already requested" message.
- **PendingRequestsList rendering**: Verify the list renders request entries with user info, reason, and timestamps; verify badge count reflects `pending_request_counts`.
- **RequestDetailPanel**: Verify Approve and Deny buttons dispatch the correct proto RPC.
- **Notification rendering**: Verify `JoinRequestApproved` notification renders with a clickable link that navigates to the channel.

### Concurrency tests

- **Race: two admins respond simultaneously**: Both approve the same request; the first succeeds, the second gets "request no longer exists" — verify no duplicate member creation.
- **Race: user requests while admin responds**: Ensure transactional isolation — either the request or the response wins, never resulting in inconsistent state.
- **Race: expiry job runs while admin responds**: Row-level locking or `SELECT FOR UPDATE` prevents double-processing.

### Edge case tests

- Request with very long reason (500+ chars) truncated by server.
- Request for a deleted channel — cascade verify.
- User requests, then is directly invited by admin — `role.is_some()` check prevents request.
- Notification with missing user (user deleted after request) — graceful handling in notification display.
