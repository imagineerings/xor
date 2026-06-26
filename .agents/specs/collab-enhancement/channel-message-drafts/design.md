# Design: Message Drafts

## 1. Overview

Channel message composition in Baymax currently loses unsent content if the user navigates away, switches channels, or closes the app. This design adds automatic local draft persistence for channel messages, with channel sidebar indicators showing which channels have unsaved drafts.

**Key decisions:**

- **Client-side only**: Drafts are stored locally in the `KeyValueStore` (SQLite KVP — already part of Baymax). No server involvement.
- **Auto-save with debounce**: Save draft 500ms after the user stops typing, debounced.
- **Channel-scoped**: Each channel has at most one draft (the most recent unsent message).
- **Draft indicator**: An italicized channel name or pencil icon in the collab panel's channel list.

## 2. Architecture

```mermaid
flowchart LR
    subgraph Client
        A[Compose Area] -->|content changes| B[Debounce 500ms]
        B -->|save| C[KeyValueStore]
        
        D[Channel Switcher] -->|check drafts| C
        D -->|draft exists| E[Draft Indicator]
        
        F[Send Message] -->|clear| C
        G[Discard Draft] -->|clear| C
        H[Channel Load] -->|restore| C
        H -->|load draft| A
    end
    
    subgraph Storage
        C[KVP: channel_drafts/{channel_id}]
    end
```

### Components

| Component | Responsibility |
|---|---|
| `DraftStore` | Client-side singleton managing draft persistence |
| `ComposeArea` | Message input field; integrates with DraftStore |
| `CollabPanel` | Channel list; shows draft indicators |
| `KeyValueStore` | Existing Baymax persistence layer (SQLite key-value pairs) |

## 3. Components and Interfaces

### 3.1 DraftStore

```rust
pub struct DraftStore {
    kvp: Entity<KeyValueStore>,
    drafts: HashMap<ChannelId, Draft>,
    active_draft_channel: Option<ChannelId>,
}

#[derive(Serialize, Deserialize)]
pub struct Draft {
    pub body: String,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl DraftStore {
    pub fn global(cx: &mut App) -> Entity<Self>;

    /// Auto-save: called by compose area on content change.
    /// Debounced externally (500ms timer).
    pub fn save_draft(&mut self, channel_id: ChannelId, body: &str, cx: &mut Context<Self>);

    /// Load draft for a channel (called when entering a channel).
    pub fn load_draft(&mut self, channel_id: ChannelId, cx: &mut Context<Self>) -> Option<String>;

    /// Clear draft after sending or discarding.
    pub fn clear_draft(&mut self, channel_id: ChannelId, cx: &mut Context<Self>);

    /// Check if a channel has a draft (for indicators).
    pub fn has_draft(&self, channel_id: ChannelId) -> bool;

    /// Get all channels with drafts.
    pub fn channels_with_drafts(&self) -> Vec<ChannelId>;
}
```

**Storage key format**: `channel_draft_{channel_id}` in `KeyValueStore`.

### 3.2 DraftStore — persistence details

```rust
impl DraftStore {
    fn persist_key(channel_id: ChannelId) -> String {
        format!("channel_draft.{}", channel_id.0)
    }

    async fn write_to_kvp(&self, channel_id: ChannelId, body: &str, cx: &AsyncApp) -> Result<()> {
        let key = Self::persist_key(channel_id);
        let value = serde_json::to_string(&Draft {
            body: body.to_string(),
            updated_at: chrono::Utc::now(),
        })?;
        self.kvp.update(cx, |kvp, cx| {
            kvp.write_kvp(key, value)
        }).await?
    }

    async fn read_from_kvp(&self, channel_id: ChannelId, cx: &AsyncApp) -> Result<Option<Draft>> {
        let key = Self::persist_key(channel_id);
        let value = self.kvp.read_with(cx, |kvp, _| kvp.read_kvp(&key))?;
        value.map(|v| serde_json::from_str::<Draft>(&v)).transpose()
    }
}
```

### 3.3 ComposeArea integration

```rust
// Pseudocode for draft integration in the compose area

impl ComposeArea {
    fn on_channel_switched(&mut self, channel_id: ChannelId, cx: &mut Context<Self>) {
        // Save current draft
        if let Some(prev_channel) = self.active_channel {
            self.draft_store.update(cx, |store, cx| {
                store.save_draft(prev_channel, &self.editor.text(cx), cx);
            });
        }
        
        // Load new channel's draft
        self.active_channel = Some(channel_id);
        if let Some(draft) = self.draft_store.update(cx, |store, cx| {
            store.load_draft(channel_id, cx)
        }) {
            self.editor.set_text(draft, cx);
        }
    }

    fn on_send_message(&mut self, channel_id: ChannelId, cx: &mut Context<Self>) {
        self.draft_store.update(cx, |store, cx| {
            store.clear_draft(channel_id, cx);
        });
    }
}
```

### 3.4 CollabPanel Draft Indicator

The `CollabPanel` observes the `DraftStore` and renders an italicized channel name or a pencil icon on channels that have drafts.

```rust
// In CollabPanel's render logic for ListEntry::Channel
impl Render for CollabPanel {
    fn render_channel_entry(&mut self, channel: &Channel, ...) -> AnyElement {
        let has_draft = self.draft_store.read(cx).has_draft(channel.id);
        
        h_flex()
            .child(channel_icon)
            .child(
                Label::new(channel.name.clone())
                    .when(has_draft, |label| label.italic())
            )
            .when(has_draft, |el| el.child(
                Icon::new(IconName::FileEdit).small()
            ))
            .into_any()
    }
}
```

## 4. Data Models

### 4.1 Local storage (KeyValueStore)

```json
// Key: "channel_draft.{channel_id}"
// Value: 
{
    "body": "Hey team, here's the update...",
    "updated_at": "2025-06-25T14:30:00Z"
}
```

### 4.2 In-memory state (DraftStore)

```rust
pub struct DraftStore {
    kvp: Entity<KeyValueStore>,
    drafts: HashMap<ChannelId, Draft>, // In-memory cache of loaded drafts
    active_draft_channel: Option<ChannelId>, // Channel currently being composed in
}
```

## 5. Correctness Properties

### Property 5.1: Draft persistence on navigation

_For any_ channel with a non-empty compose area, when the user navigates away from that channel, the draft content SHALL be persisted to `KeyValueStore` within 1 second.

**Validates: Requirement 7.1**

### Property 5.2: Draft restoration

_For any_ channel that has a saved draft in `KeyValueStore`, when the user opens that channel, the compose area SHALL pre-fill with the saved draft content.

**Validates: Requirement 7.1**

### Property 5.3: Draft clearing on send

_For any_ channel where the user successfully sends a message, the draft for that channel SHALL be cleared from both in-memory cache and `KeyValueStore`.

**Validates: Requirement 7.1**

### Property 5.4: Draft indicator consistency

_For any_ channel, `has_draft(channel_id)` SHALL return true if and only if `KeyValueStore` contains a non-empty draft for that channel.

**Validates: Requirement 7.2**

### Property 5.5: Storage limits

_For any_ number of drafts exceeding `MAX_DRAFTS` (default 50), the oldest draft by `updated_at` SHALL be evicted on the next write.

**Validates: Requirement 7.4**

## 6. Error Handling

| Error | Handling |
|---|---|
| KVP write failure | Log warning; draft stays in in-memory cache only; retry on next auto-save |
| KVP read failure | Return `None`; compose area starts empty; log error |
| Corrupt draft JSON | Discard corrupt draft; log error; compose area starts empty |
| Concurrent channel switching during save | Use per-channel serialization via channel-scoped task; queue saves |
| Exceeded draft limit | Evict oldest draft(s) by `updated_at` before writing new one |

## 7. Testing Strategy

- **Unit tests**: DraftStore save/load/clear/has_draft with mock KVP
- **Persistence tests**: Verify drafts survive app restart (write KVP → simulate restart → read back)
- **Integration tests**: 
  - Type in channel A → switch to channel B → switch back to A → verify draft restored
  - Send message → verify draft cleared
  - Verify draft indicator appears/disappears correctly
- **Concurrency tests**: Rapid channel switching while typing; verify no data loss
