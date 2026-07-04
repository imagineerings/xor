use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use gpui::{Context, Task, WeakEntity};
use log;
use reqwest::Client;
use serde::Deserialize;

use crate::{GatewayHandler, GatewayManager, IncomingMessage, OutgoingMessage};

/// Base URL for the Telegram Bot API.
const TELEGRAM_API_BASE: &str = "https://api.telegram.org/bot";

// ---------------------------------------------------------------------------
// Telegram API response types (minimal — only what we need)
// ---------------------------------------------------------------------------

/// Generic Telegram API response wrapper.
#[derive(Deserialize)]
struct TelegramResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
}

/// An update from `getUpdates`.
#[derive(Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    message: Option<TelegramMessageObj>,
}

/// A message object within an update.
#[derive(Deserialize)]
#[allow(dead_code)]
struct TelegramMessageObj {
    message_id: i64,
    from: Option<TelegramUser>,
    chat: TelegramChat,
    text: Option<String>,
    date: i64,
}

/// A Telegram user (sender).
#[derive(Deserialize)]
#[allow(dead_code)]
struct TelegramUser {
    id: i64,
    first_name: Option<String>,
    last_name: Option<String>,
    username: Option<String>,
}

/// A Telegram chat.
#[derive(Deserialize)]
#[allow(dead_code)]
struct TelegramChat {
    id: i64,
    #[serde(rename = "type")]
    chat_type: String,
}

// ---------------------------------------------------------------------------
// TelegramGateway
// ---------------------------------------------------------------------------

/// A gateway handler that communicates with the Telegram Bot API.
///
/// Uses long-polling (`getUpdates`) to receive messages and REST API calls
/// to send responses. Outgoing messages are queued via a channel and
/// processed by the polling loop.
pub struct TelegramGateway {
    bot_token: String,
    api_client: Client,
    polling_interval: Duration,
    outgoing_tx: Option<mpsc::Sender<OutgoingMessage>>,
}

impl TelegramGateway {
    /// Create a new Telegram gateway for the given bot token.
    pub fn new(bot_token: String) -> Self {
        Self {
            bot_token,
            api_client: Client::new(),
            polling_interval: Duration::from_secs(1),
            outgoing_tx: None,
        }
    }

    /// Set a custom polling interval (default is 1 second).
    pub fn with_polling_interval(mut self, interval: Duration) -> Self {
        self.polling_interval = interval;
        self
    }
}

impl GatewayHandler for TelegramGateway {
    fn name(&self) -> &str {
        "telegram"
    }

    fn start(&mut self, cx: &mut Context<GatewayManager>) -> Task<Result<()>> {
        let (tx, rx) = mpsc::channel();
        self.outgoing_tx = Some(tx);

        let api_url = format!("{}{}", TELEGRAM_API_BASE, self.bot_token);
        let client = self.api_client.clone();
        let interval = self.polling_interval;

        cx.spawn(async move |this: WeakEntity<GatewayManager>, cx| {
            let mut offset = 0i64;

            loop {
                // Drain any queued outgoing messages.
                while let Ok(msg) = rx.try_recv() {
                    if let Err(e) = send_telegram_message(&client, &api_url, &msg).await {
                        log::error!("telegram: failed to send message: {e}");
                    }
                }

                // Poll for incoming updates.
                match poll_updates(&client, &api_url, offset).await {
                    Ok(updates) => {
                        for update in updates {
                            if let Some(incoming) = telegram_update_to_incoming(&update) {
                                if let Some(manager) = this.upgrade() {
                                    let _ = manager.update(cx, |mgr, cx| {
                                        mgr.route_message(incoming, cx);
                                    });
                                }
                            }
                            offset = update.update_id + 1;
                        }
                    }
                    Err(e) => {
                        log::error!("telegram: polling error: {e}");
                    }
                }

                cx.background_executor().timer(interval).await;
            }
        })
    }

    fn stop(&mut self) -> Task<Result<()>> {
        // The polling task will be dropped when this handler is removed,
        // which cancels the task automatically.
        Task::ready(Ok(()))
    }

    fn send_message(&self, message: OutgoingMessage) -> Task<Result<()>> {
        if let Some(tx) = &self.outgoing_tx {
            let _ = tx.send(message);
        }
        Task::ready(Ok(()))
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Poll for updates from the Telegram Bot API.
async fn poll_updates(client: &Client, api_url: &str, offset: i64) -> Result<Vec<TelegramUpdate>> {
    let url = format!("{api_url}/getUpdates");
    let resp: TelegramResponse<Vec<TelegramUpdate>> = client
        .post(&url)
        .json(&serde_json::json!({
            "offset": offset,
            "timeout": 5,
        }))
        .send()
        .await?
        .json()
        .await?;

    if !resp.ok {
        anyhow::bail!(
            "Telegram API error: {}",
            resp.description.unwrap_or_default()
        );
    }

    Ok(resp.result.unwrap_or_default())
}

/// Convert a Telegram API update into our platform-agnostic [`IncomingMessage`].
fn telegram_update_to_incoming(update: &TelegramUpdate) -> Option<IncomingMessage> {
    let msg_obj = update.message.as_ref()?;
    let chat_id = msg_obj.chat.id.to_string();
    let user_id = msg_obj
        .from
        .as_ref()
        .map(|u| u.id.to_string())
        .unwrap_or_else(|| chat_id.clone());

    let timestamp = DateTime::from_timestamp(msg_obj.date, 0).unwrap_or_else(|| Utc::now());

    Some(IncomingMessage {
        platform: "telegram".into(),
        platform_id: chat_id,
        user_id,
        text: msg_obj.text.clone().unwrap_or_default(),
        attachments: Vec::new(),
        timestamp,
    })
}

/// Send an outgoing message via the Telegram Bot API.
async fn send_telegram_message(
    client: &Client,
    api_url: &str,
    msg: &OutgoingMessage,
) -> Result<()> {
    let url = format!("{api_url}/sendMessage");
    let params = serde_json::json!({
        "chat_id": msg.platform_id,
        "text": msg.text,
    });

    let resp: TelegramResponse<serde_json::Value> =
        client.post(&url).json(&params).send().await?.json().await?;

    if !resp.ok {
        anyhow::bail!(
            "Telegram sendMessage error: {}",
            resp.description.unwrap_or_default()
        );
    }

    Ok(())
}
