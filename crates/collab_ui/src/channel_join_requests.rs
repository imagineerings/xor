use client::{ChannelId, User};
use gpui::SharedString;
use std::sync::Arc;
use time::OffsetDateTime;

#[derive(Clone, Debug)]
pub struct PendingJoinRequest {
    pub user_id: u64,
    pub user: Arc<User>,
    pub reason: Option<SharedString>,
    pub created_at: OffsetDateTime,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingRequestCount {
    pub channel_id: ChannelId,
    pub count: u32,
}
