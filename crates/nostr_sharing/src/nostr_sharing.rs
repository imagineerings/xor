use std::{
    collections::HashMap,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context as _, Result, anyhow, bail, ensure};
use serde::{Deserialize, Serialize};
use session::import::ImportedSession;
use uuid::Uuid;

pub const SIM_SESSION_EVENT_KIND: u64 = 30_078;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NostrShareConfig {
    pub relays: Vec<String>,
    #[serde(default)]
    pub private_key: Option<String>,
}

impl NostrShareConfig {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.relays.is_empty(),
            "Nostr sharing requires at least one relay"
        );
        for relay in &self.relays {
            ensure!(!relay.trim().is_empty(), "Nostr relay URL cannot be empty");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NostrEvent {
    pub id: String,
    pub pubkey: String,
    pub created_at: u64,
    pub kind: u64,
    pub tags: Vec<Vec<String>>,
    pub content: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NostrSessionSummary {
    pub event_id: String,
    pub title: String,
    pub author: String,
    pub created_at: u64,
}

pub trait NostrRelayClient: Send + Sync {
    fn publish(&self, relay: &str, event: &NostrEvent) -> Result<String>;
    fn fetch(&self, relay: &str, event_id: &str) -> Result<Option<NostrEvent>>;
    fn discover_by_author(&self, relay: &str, author: &str) -> Result<Vec<NostrEvent>>;
}

pub struct NostrShare<C> {
    config: NostrShareConfig,
    client: C,
}

impl<C> NostrShare<C>
where
    C: NostrRelayClient,
{
    pub fn new(config: NostrShareConfig, client: C) -> Result<Self> {
        config.validate()?;
        Ok(Self { config, client })
    }

    pub fn config(&self) -> &NostrShareConfig {
        &self.config
    }

    pub fn publish_session(&self, session: &ImportedSession) -> Result<String> {
        let event = session_to_event(session, self.author_pubkey())?;
        let mut errors = Vec::new();
        for relay in &self.config.relays {
            match self.client.publish(relay, &event) {
                Ok(event_id) => return Ok(event_id),
                Err(error) => errors.push(format!("{relay}: {error}")),
            }
        }
        bail!(
            "failed to publish session to all configured Nostr relays: {}",
            errors.join("; ")
        )
    }

    pub fn import_session(&self, event_id: &str) -> Result<ImportedSession> {
        let mut errors = Vec::new();
        for relay in &self.config.relays {
            match self.client.fetch(relay, event_id) {
                Ok(Some(event)) => return event_to_session(&event),
                Ok(None) => {}
                Err(error) => errors.push(format!("{relay}: {error}")),
            }
        }
        if errors.is_empty() {
            bail!("Nostr session event `{event_id}` was not found");
        }
        bail!(
            "failed to fetch Nostr session event `{event_id}`: {}",
            errors.join("; ")
        )
    }

    pub fn discover_sessions(&self, author: &str) -> Result<Vec<NostrSessionSummary>> {
        let mut summaries = Vec::new();
        let mut errors = Vec::new();
        for relay in &self.config.relays {
            match self.client.discover_by_author(relay, author) {
                Ok(events) => {
                    for event in events {
                        if event.kind == SIM_SESSION_EVENT_KIND
                            && let Ok(session) = event_to_session(&event)
                        {
                            summaries.push(NostrSessionSummary {
                                event_id: event.id,
                                title: session.title,
                                author: event.pubkey,
                                created_at: event.created_at,
                            });
                        }
                    }
                }
                Err(error) => errors.push(format!("{relay}: {error}")),
            }
        }
        summaries.sort_by(|left, right| right.created_at.cmp(&left.created_at));
        summaries.dedup_by(|left, right| left.event_id == right.event_id);
        if summaries.is_empty() && !errors.is_empty() {
            bail!("failed to discover Nostr sessions: {}", errors.join("; "));
        }
        Ok(summaries)
    }

    fn author_pubkey(&self) -> &str {
        self.config.private_key.as_deref().unwrap_or("sim-local")
    }
}

pub fn session_to_event(session: &ImportedSession, author_pubkey: &str) -> Result<NostrEvent> {
    ensure!(
        !session.messages.is_empty(),
        "cannot publish an empty imported session"
    );
    let content = serde_json::to_string(session).context("serializing imported session")?;
    Ok(NostrEvent {
        id: Uuid::new_v4().to_string(),
        pubkey: author_pubkey.to_string(),
        created_at: unix_timestamp(),
        kind: SIM_SESSION_EVENT_KIND,
        tags: vec![
            vec!["client".to_string(), "sim".to_string()],
            vec!["title".to_string(), session.title.clone()],
        ],
        content,
    })
}

pub fn event_to_session(event: &NostrEvent) -> Result<ImportedSession> {
    ensure!(
        event.kind == SIM_SESSION_EVENT_KIND,
        "Nostr event kind {} is not a Sim session",
        event.kind
    );
    serde_json::from_str(&event.content).context("deserializing imported session from Nostr event")
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[derive(Default)]
pub struct InMemoryRelayClient {
    events: Mutex<HashMap<String, NostrEvent>>,
}

impl NostrRelayClient for InMemoryRelayClient {
    fn publish(&self, _relay: &str, event: &NostrEvent) -> Result<String> {
        let mut events = self
            .events
            .lock()
            .map_err(|_| anyhow!("Nostr relay event store lock poisoned"))?;
        events.insert(event.id.clone(), event.clone());
        Ok(event.id.clone())
    }

    fn fetch(&self, _relay: &str, event_id: &str) -> Result<Option<NostrEvent>> {
        let events = self
            .events
            .lock()
            .map_err(|_| anyhow!("Nostr relay event store lock poisoned"))?;
        Ok(events.get(event_id).cloned())
    }

    fn discover_by_author(&self, _relay: &str, author: &str) -> Result<Vec<NostrEvent>> {
        let events = self
            .events
            .lock()
            .map_err(|_| anyhow!("Nostr relay event store lock poisoned"))?;
        Ok(events
            .values()
            .filter(|event| event.pubkey == author)
            .cloned()
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use session::import::{ImportedMessage, ImportedRole};

    fn imported_session() -> ImportedSession {
        ImportedSession {
            title: "Shared session".to_string(),
            messages: vec![
                ImportedMessage {
                    role: ImportedRole::User,
                    content: "Hello".to_string(),
                    timestamp: None,
                    metadata: json!({}),
                },
                ImportedMessage {
                    role: ImportedRole::Assistant,
                    content: "Hi".to_string(),
                    timestamp: None,
                    metadata: json!({}),
                },
            ],
            metadata: json!({ "source": "test" }),
        }
    }

    #[test]
    fn event_round_trips_imported_session() {
        let session = imported_session();
        let event = session_to_event(&session, "author").expect("session to event");

        let imported = event_to_session(&event).expect("event to session");

        assert_eq!(imported, session);
        assert_eq!(event.pubkey, "author");
        assert_eq!(event.kind, SIM_SESSION_EVENT_KIND);
    }

    #[test]
    fn publishes_imports_and_discovers_sessions() {
        let share = NostrShare::new(
            NostrShareConfig {
                relays: vec!["memory://relay".to_string()],
                private_key: Some("author".to_string()),
            },
            InMemoryRelayClient::default(),
        )
        .expect("create share");
        let session = imported_session();

        let event_id = share.publish_session(&session).expect("publish session");
        let imported = share.import_session(&event_id).expect("import session");
        let summaries = share
            .discover_sessions("author")
            .expect("discover sessions");

        assert_eq!(imported, session);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].event_id, event_id);
        assert_eq!(summaries[0].title, "Shared session");
    }

    #[test]
    fn rejects_empty_relay_configuration() {
        let result = NostrShare::new(
            NostrShareConfig {
                relays: Vec::new(),
                private_key: None,
            },
            InMemoryRelayClient::default(),
        );
        let error = match result {
            Ok(_) => panic!("empty relays should fail"),
            Err(error) => error,
        };

        assert!(error.to_string().contains("at least one relay"));
    }
}
