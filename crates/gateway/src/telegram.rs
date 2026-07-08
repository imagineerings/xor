use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;
use chrono::{DateTime, Utc};
use gpui::{Context, Task, WeakEntity};
use log;
use reqwest::Client;
use serde::Deserialize;

use crate::{Attachment, GatewayHandler, GatewayManager, IncomingMessage, OutgoingMessage};

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
    caption: Option<String>,
    photo: Option<Vec<TelegramPhotoSize>>,
    document: Option<TelegramDocument>,
    audio: Option<TelegramAudio>,
    voice: Option<TelegramVoice>,
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

#[derive(Deserialize)]
struct TelegramPhotoSize {
    file_id: String,
    file_size: Option<i64>,
}

#[derive(Deserialize)]
struct TelegramDocument {
    file_id: String,
    file_name: Option<String>,
    mime_type: Option<String>,
    file_size: Option<i64>,
}

#[derive(Deserialize)]
struct TelegramAudio {
    file_id: String,
    file_name: Option<String>,
    mime_type: Option<String>,
    file_size: Option<i64>,
}

#[derive(Deserialize)]
struct TelegramVoice {
    file_id: String,
    mime_type: Option<String>,
    file_size: Option<i64>,
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
                                    manager.update(cx, |mgr, cx| {
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
            if let Err(error) = tx.send(message) {
                log::error!("telegram: failed to queue outgoing message: {error}");
            }
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
        text: msg_obj
            .text
            .clone()
            .or_else(|| msg_obj.caption.clone())
            .unwrap_or_default(),
        attachments: telegram_attachments(msg_obj),
        timestamp,
    })
}

fn telegram_attachments(message: &TelegramMessageObj) -> Vec<Attachment> {
    let mut attachments = Vec::new();

    if let Some(photo) = message.photo.as_ref().and_then(|sizes| sizes.last()) {
        attachments.push(Attachment {
            kind: "photo".into(),
            mime_type: None,
            url: Some(telegram_file_url(&photo.file_id)),
            file_size: photo.file_size,
            file_name: None,
        });
    }

    if let Some(document) = &message.document {
        attachments.push(Attachment {
            kind: "document".into(),
            mime_type: document.mime_type.clone(),
            url: Some(telegram_file_url(&document.file_id)),
            file_size: document.file_size,
            file_name: document.file_name.clone(),
        });
    }

    if let Some(audio) = &message.audio {
        attachments.push(Attachment {
            kind: "audio".into(),
            mime_type: audio.mime_type.clone(),
            url: Some(telegram_file_url(&audio.file_id)),
            file_size: audio.file_size,
            file_name: audio.file_name.clone(),
        });
    }

    if let Some(voice) = &message.voice {
        attachments.push(Attachment {
            kind: "voice".into(),
            mime_type: voice.mime_type.clone(),
            url: Some(telegram_file_url(&voice.file_id)),
            file_size: voice.file_size,
            file_name: None,
        });
    }

    attachments
}

fn telegram_file_url(file_id: &str) -> String {
    format!("telegram:file/{file_id}")
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;

    #[test]
    fn telegram_update_to_incoming_uses_text_messages() {
        let update: TelegramUpdate = serde_json::from_str(
            r#"{
                "update_id": 10,
                "message": {
                    "message_id": 20,
                    "from": {"id": 30, "first_name": "Ada"},
                    "chat": {"id": 40, "type": "private"},
                    "text": "hello",
                    "date": 1700000000
                }
            }"#,
        )
        .unwrap();

        let incoming = telegram_update_to_incoming(&update).unwrap();

        assert_eq!(incoming.platform, "telegram");
        assert_eq!(incoming.platform_id, "40");
        assert_eq!(incoming.user_id, "30");
        assert_eq!(incoming.text, "hello");
        assert!(incoming.attachments.is_empty());
    }

    #[test]
    fn telegram_update_to_incoming_maps_media_attachments() {
        let update: TelegramUpdate = serde_json::from_str(
            r#"{
                "update_id": 10,
                "message": {
                    "message_id": 20,
                    "from": {"id": 30, "first_name": "Ada"},
                    "chat": {"id": 40, "type": "private"},
                    "caption": "see attached",
                    "photo": [
                        {"file_id": "small-photo", "width": 32, "height": 32, "file_size": 100},
                        {"file_id": "large-photo", "width": 1024, "height": 768, "file_size": 2000}
                    ],
                    "document": {
                        "file_id": "doc-file",
                        "file_name": "notes.md",
                        "mime_type": "text/markdown",
                        "file_size": 3000
                    },
                    "audio": {
                        "file_id": "audio-file",
                        "file_name": "clip.mp3",
                        "mime_type": "audio/mpeg",
                        "file_size": 4000
                    },
                    "voice": {
                        "file_id": "voice-file",
                        "mime_type": "audio/ogg",
                        "file_size": 5000
                    },
                    "date": 1700000000
                }
            }"#,
        )
        .unwrap();

        let incoming = telegram_update_to_incoming(&update).unwrap();

        assert_eq!(incoming.text, "see attached");
        assert_eq!(incoming.attachments.len(), 4);
        assert_eq!(incoming.attachments[0].kind, "photo");
        assert_eq!(
            incoming.attachments[0].url.as_deref(),
            Some("telegram:file/large-photo")
        );
        assert_eq!(incoming.attachments[1].kind, "document");
        assert_eq!(
            incoming.attachments[1].file_name.as_deref(),
            Some("notes.md")
        );
        assert_eq!(
            incoming.attachments[1].mime_type.as_deref(),
            Some("text/markdown")
        );
        assert_eq!(incoming.attachments[2].kind, "audio");
        assert_eq!(
            incoming.attachments[2].file_name.as_deref(),
            Some("clip.mp3")
        );
        assert_eq!(incoming.attachments[3].kind, "voice");
        assert_eq!(
            incoming.attachments[3].mime_type.as_deref(),
            Some("audio/ogg")
        );
    }

    #[test]
    fn poll_updates_posts_offset_and_parses_telegram_messages() {
        let (api_url, request_rx) = spawn_test_server(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"ok\":true,\"result\":[{\"update_id\":42,\"message\":{\"message_id\":99,\"from\":{\"id\":7,\"first_name\":\"Ada\"},\"chat\":{\"id\":11,\"type\":\"private\"},\"text\":\"hello\",\"date\":1700000000}}]}",
        );

        let updates = block_on_tokio(poll_updates(&Client::new(), &api_url, 41))
            .expect("polling mocked updates should succeed");
        let request = request_rx
            .recv()
            .expect("test server should receive one request");

        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].update_id, 42);
        assert!(request.starts_with("POST /getUpdates HTTP/1.1"));
        assert!(request.contains("\"offset\":41"));
        assert!(request.contains("\"timeout\":5"));

        let incoming = telegram_update_to_incoming(&updates[0]).unwrap();
        assert_eq!(incoming.platform_id, "11");
        assert_eq!(incoming.user_id, "7");
        assert_eq!(incoming.text, "hello");
    }

    #[test]
    fn poll_updates_reports_telegram_api_errors() {
        let (api_url, _request_rx) = spawn_test_server(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"ok\":false,\"description\":\"bad token\"}",
        );

        let error = match block_on_tokio(poll_updates(&Client::new(), &api_url, 0)) {
            Ok(_) => panic!("Telegram API errors should fail polling"),
            Err(error) => error,
        };

        assert_eq!(error.to_string(), "Telegram API error: bad token");
    }

    #[test]
    fn send_telegram_message_posts_chat_and_text() {
        let (api_url, request_rx) = spawn_test_server(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"ok\":true,\"result\":{\"message_id\":123}}",
        );
        let message = OutgoingMessage {
            platform: "telegram".into(),
            platform_id: "chat-1".into(),
            text: "hello from sim".into(),
            attachments: Vec::new(),
            reply_to: None,
        };

        block_on_tokio(send_telegram_message(&Client::new(), &api_url, &message))
            .expect("sending mocked Telegram message should succeed");
        let request = request_rx
            .recv()
            .expect("test server should receive one request");

        assert!(request.starts_with("POST /sendMessage HTTP/1.1"));
        assert!(request.contains("\"chat_id\":\"chat-1\""));
        assert!(request.contains("\"text\":\"hello from sim\""));
    }

    #[test]
    fn send_telegram_message_reports_api_errors() {
        let (api_url, _request_rx) = spawn_test_server(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{\"ok\":false,\"description\":\"rate limited\"}",
        );
        let message = OutgoingMessage {
            platform: "telegram".into(),
            platform_id: "chat-1".into(),
            text: "hello".into(),
            attachments: Vec::new(),
            reply_to: None,
        };

        let error = block_on_tokio(send_telegram_message(&Client::new(), &api_url, &message))
            .expect_err("Telegram send errors should fail");

        assert_eq!(
            error.to_string(),
            "Telegram sendMessage error: rate limited"
        );
    }

    fn spawn_test_server(response: &'static str) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let address = listener.local_addr().expect("read test server address");
        let (request_tx, request_rx) = mpsc::channel();

        thread::spawn(move || {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut buffer = [0; 4096];
            let Ok(read) = stream.read(&mut buffer) else {
                return;
            };
            let request = String::from_utf8_lossy(&buffer[..read]).into_owned();
            request_tx.send(request).expect("send captured request");
            stream
                .write_all(response.as_bytes())
                .expect("write test response");
        });

        (format!("http://{address}"), request_rx)
    }

    fn block_on_tokio<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .build()
            .expect("build tokio test runtime")
            .block_on(future)
    }
}
