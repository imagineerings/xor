use anyhow::{Context as _, Result};
use chrono::{DateTime, Local, Utc};
use rpc::proto;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScheduledMessageId(pub u64);

impl ScheduledMessageId {
    pub fn from_proto(id: u64) -> Self {
        Self(id)
    }

    pub fn to_proto(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduledMessage {
    pub id: ScheduledMessageId,
    pub channel_id: u64,
    pub sender_id: u64,
    pub body: String,
    pub scheduled_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub nonce: Option<u128>,
    pub mentions: Vec<proto::ChatMention>,
    pub display_time: DateTime<Local>,
}

impl TryFrom<proto::ScheduledMessage> for ScheduledMessage {
    type Error = anyhow::Error;

    fn try_from(message: proto::ScheduledMessage) -> Result<Self> {
        let scheduled_at = datetime_from_millis(message.scheduled_at, "scheduled message time")?;
        let created_at =
            datetime_from_millis(message.created_at, "scheduled message created time")?;
        Ok(Self {
            id: ScheduledMessageId::from_proto(message.id),
            channel_id: message.channel_id,
            sender_id: message.sender_id,
            body: message.body,
            scheduled_at,
            created_at,
            nonce: message.nonce.map(u128::from),
            mentions: message.mentions,
            display_time: scheduled_at.with_timezone(&Local),
        })
    }
}

impl From<ScheduledMessage> for proto::ScheduledMessage {
    fn from(message: ScheduledMessage) -> Self {
        Self {
            id: message.id.to_proto(),
            channel_id: message.channel_id,
            body: message.body,
            sender_id: message.sender_id,
            scheduled_at: datetime_to_millis(message.scheduled_at),
            created_at: datetime_to_millis(message.created_at),
            nonce: message.nonce.map(Into::into),
            mentions: message.mentions,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChannelMessage {
    pub id: u64,
    pub body: String,
    pub timestamp: DateTime<Utc>,
    pub sender_id: u64,
    pub nonce: Option<u128>,
    pub mentions: Vec<proto::ChatMention>,
    pub reply_to_message_id: Option<u64>,
    pub edited_at: Option<DateTime<Utc>>,
    pub reaction_summaries: Vec<proto::ReactionSummary>,
    pub scheduled_at: Option<DateTime<Utc>>,
}

impl TryFrom<proto::ChannelMessage> for ChannelMessage {
    type Error = anyhow::Error;

    fn try_from(message: proto::ChannelMessage) -> Result<Self> {
        Ok(Self {
            id: message.id,
            body: message.body,
            timestamp: datetime_from_seconds(message.timestamp, "channel message timestamp")?,
            sender_id: message.sender_id,
            nonce: message.nonce.map(u128::from),
            mentions: message.mentions,
            reply_to_message_id: message.reply_to_message_id,
            edited_at: message
                .edited_at
                .map(|timestamp| datetime_from_seconds(timestamp, "channel message edit time"))
                .transpose()?,
            reaction_summaries: message.reaction_summaries,
            scheduled_at: message
                .scheduled_at
                .map(|timestamp| datetime_from_millis(timestamp, "channel message schedule time"))
                .transpose()?,
        })
    }
}

fn datetime_from_seconds(timestamp: u64, label: &str) -> Result<DateTime<Utc>> {
    let timestamp = timestamp
        .try_into()
        .with_context(|| format!("{label} is out of range"))?;
    DateTime::<Utc>::from_timestamp(timestamp, 0).with_context(|| format!("{label} is invalid"))
}

fn datetime_from_millis(timestamp: u64, label: &str) -> Result<DateTime<Utc>> {
    let timestamp = timestamp
        .try_into()
        .with_context(|| format!("{label} is out of range"))?;
    DateTime::<Utc>::from_timestamp_millis(timestamp).with_context(|| format!("{label} is invalid"))
}

fn datetime_to_millis(timestamp: DateTime<Utc>) -> u64 {
    timestamp.timestamp_millis() as u64
}
