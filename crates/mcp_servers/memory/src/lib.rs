use anyhow::{Context as _, Result, anyhow, bail, ensure};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlez::{bindable::Column, connection::Connection, statement::Statement};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};
use uuid::Uuid;

const STORE_FACT: &str = "store_fact";
const RETRIEVE_MEMORIES: &str = "retrieve_memories";
const SEARCH_MEMORIES: &str = "search_memories";
const DEFAULT_LIMIT: usize = 20;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryItem {
    pub id: String,
    pub key: Option<String>,
    pub value: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
    pub importance: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StoreFactRequest {
    pub value: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default = "default_importance")]
    pub importance: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RetrieveMemoriesRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchMemoriesRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

pub trait MemoryStore: Send + Sync {
    fn store(&self, request: StoreFactRequest) -> Result<MemoryItem>;
    fn retrieve(&self, query: &str, limit: usize) -> Result<Vec<MemoryItem>>;
    fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryItem>>;
    fn delete(&self, key: &str) -> Result<()>;
}

pub struct SqliteMemoryStore {
    path: PathBuf,
    connection: Mutex<Connection>,
}

impl SqliteMemoryStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("creating memory store directory {}", parent.display()))?;
        }

        let connection = Connection::open_file(&path.to_string_lossy());
        ensure!(
            connection.persistent(),
            "failed to open persistent SQLite memory store at {}",
            path.display()
        );
        let store = Self {
            path,
            connection: Mutex::new(connection),
        };
        store.migrate()?;
        Ok(store)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn migrate(&self) -> Result<()> {
        let connection = self.lock_connection()?;
        let mut statement = Statement::prepare(
            &connection,
            "
            CREATE TABLE IF NOT EXISTS memories (
                id TEXT PRIMARY KEY,
                key TEXT UNIQUE,
                value TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL,
                importance REAL NOT NULL
            ) STRICT;
            ",
        )?;
        statement.exec()?;

        let mut statement = Statement::prepare(
            &connection,
            "CREATE INDEX IF NOT EXISTS idx_memories_value ON memories(value);",
        )?;
        statement.exec()
    }

    fn lock_connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|_| anyhow!("memory store connection lock poisoned"))
    }
}

impl MemoryStore for SqliteMemoryStore {
    fn store(&self, request: StoreFactRequest) -> Result<MemoryItem> {
        let value = request.value.trim().to_string();
        if value.is_empty() {
            bail!("memory value cannot be empty");
        }

        if !request.importance.is_finite() || request.importance < 0.0 {
            bail!("memory importance must be a finite non-negative number");
        }

        let item = MemoryItem {
            id: Uuid::new_v4().to_string(),
            key: request.key,
            value,
            metadata: normalize_metadata(request.metadata)?,
            created_at: Utc::now(),
            importance: request.importance,
        };
        let metadata = serde_json::to_string(&item.metadata)?;
        let created_at = item.created_at.to_rfc3339();

        let connection = self.lock_connection()?;
        let mut statement = Statement::prepare(
            &connection,
            "
            INSERT INTO memories (id, key, value, metadata, created_at, importance)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                metadata = excluded.metadata,
                created_at = excluded.created_at,
                importance = excluded.importance
            ",
        )?;
        statement.bind(&item.id, 1)?;
        statement.bind(&item.key, 2)?;
        statement.bind(&item.value, 3)?;
        statement.bind(&metadata, 4)?;
        statement.bind(&created_at, 5)?;
        statement.bind(&item.importance, 6)?;
        statement.exec()?;
        Ok(item)
    }

    fn retrieve(&self, query: &str, limit: usize) -> Result<Vec<MemoryItem>> {
        self.query_memories(query, limit, QueryMode::Relevant)
    }

    fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryItem>> {
        self.query_memories(query, limit, QueryMode::Search)
    }

    fn delete(&self, key: &str) -> Result<()> {
        let connection = self.lock_connection()?;
        let mut statement = Statement::prepare(&connection, "DELETE FROM memories WHERE key = ?1")?;
        statement.bind(&key, 1)?;
        statement.exec()
    }
}

impl SqliteMemoryStore {
    fn query_memories(
        &self,
        query: &str,
        limit: usize,
        mode: QueryMode,
    ) -> Result<Vec<MemoryItem>> {
        let query = query.trim();
        if query.is_empty() {
            bail!("memory query cannot be empty");
        }

        let limit = clamp_limit(limit);
        let pattern = format!("%{}%", escape_like_pattern(query));
        let connection = self.lock_connection()?;
        let mut statement = Statement::prepare(&connection, mode.sql())?;
        statement.bind(&pattern, 1)?;
        statement.bind(&(limit as i64), 2)?;
        statement.map(memory_item_from_row)
    }
}

enum QueryMode {
    Relevant,
    Search,
}

impl QueryMode {
    fn sql(&self) -> &'static str {
        match self {
            QueryMode::Relevant => {
                "
                SELECT id, key, value, metadata, created_at, importance
                FROM memories
                WHERE key LIKE ?1 ESCAPE '\\' OR value LIKE ?1 ESCAPE '\\' OR metadata LIKE ?1 ESCAPE '\\'
                ORDER BY importance DESC, created_at DESC
                LIMIT ?2
                "
            }
            QueryMode::Search => {
                "
                SELECT id, key, value, metadata, created_at, importance
                FROM memories
                WHERE key LIKE ?1 ESCAPE '\\' OR value LIKE ?1 ESCAPE '\\' OR metadata LIKE ?1 ESCAPE '\\'
                ORDER BY created_at DESC
                LIMIT ?2
                "
            }
        }
    }
}

pub struct MemoryServer<S = SqliteMemoryStore> {
    store: S,
}

impl<S> MemoryServer<S> {
    pub fn new(store: S) -> Self {
        Self { store }
    }

    pub fn store(&self) -> &S {
        &self.store
    }
}

impl<S: MemoryStore> MemoryServer<S> {
    pub fn capabilities(&self) -> Value {
        json!({
            "tools": {
                "listChanged": false
            }
        })
    }

    pub fn tools(&self) -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor {
                name: STORE_FACT.to_string(),
                description: "Store a long-term memory fact locally.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "key": { "type": "string" },
                        "value": { "type": "string" },
                        "metadata": { "type": "object" },
                        "importance": { "type": "number", "minimum": 0 }
                    },
                    "required": ["value"]
                }),
            },
            ToolDescriptor {
                name: RETRIEVE_MEMORIES.to_string(),
                description: "Retrieve relevant long-term memories for a query.".to_string(),
                input_schema: query_schema(),
            },
            ToolDescriptor {
                name: SEARCH_MEMORIES.to_string(),
                description: "Search stored long-term memories by key, value, or metadata."
                    .to_string(),
                input_schema: query_schema(),
            },
        ]
    }

    pub fn handle_tool_call(&self, name: &str, arguments: Value) -> Result<Value> {
        match name {
            STORE_FACT => {
                let request =
                    serde_json::from_value(arguments).context("parsing store_fact arguments")?;
                let memory = self.store.store(request)?;
                Ok(json!({ "memory": memory }))
            }
            RETRIEVE_MEMORIES => {
                let request: RetrieveMemoriesRequest = serde_json::from_value(arguments)
                    .context("parsing retrieve_memories arguments")?;
                let memories = self.store.retrieve(&request.query, request.limit)?;
                Ok(json!({ "memories": memories }))
            }
            SEARCH_MEMORIES => {
                let request: SearchMemoriesRequest = serde_json::from_value(arguments)
                    .context("parsing search_memories arguments")?;
                let memories = self.store.search(&request.query, request.limit)?;
                Ok(json!({ "memories": memories }))
            }
            _ => bail!("unknown memory tool `{name}`"),
        }
    }
}

fn query_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": { "type": "string" },
            "limit": { "type": "integer", "minimum": 1, "maximum": 100 }
        },
        "required": ["query"]
    })
}

fn memory_item_from_row(statement: &mut Statement) -> Result<MemoryItem> {
    let (id, next_index): (String, i32) = Column::column(statement, 0)?;
    let (key, next_index): (Option<String>, i32) = Column::column(statement, next_index)?;
    let (value, next_index): (String, i32) = Column::column(statement, next_index)?;
    let (metadata, next_index): (String, i32) = Column::column(statement, next_index)?;
    let (created_at, next_index): (String, i32) = Column::column(statement, next_index)?;
    let (importance, _next_index): (f32, i32) = Column::column(statement, next_index)?;

    Ok(MemoryItem {
        id,
        key,
        value,
        metadata: serde_json::from_str(&metadata).context("parsing memory metadata")?,
        created_at: DateTime::parse_from_rfc3339(&created_at)
            .context("parsing memory creation timestamp")?
            .with_timezone(&Utc),
        importance,
    })
}

fn normalize_metadata(metadata: Value) -> Result<Value> {
    match metadata {
        Value::Null => Ok(json!({})),
        Value::Object(_) => Ok(metadata),
        _ => bail!("memory metadata must be an object"),
    }
}

fn escape_like_pattern(query: &str) -> String {
    query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

fn clamp_limit(limit: usize) -> usize {
    limit.clamp(1, 100)
}

fn default_limit() -> usize {
    DEFAULT_LIMIT
}

fn default_importance() -> f32 {
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_store() -> (TempDir, SqliteMemoryStore) {
        let temp_dir = TempDir::new().expect("create temp dir");
        let store = SqliteMemoryStore::open(temp_dir.path().join("memory.sqlite"))
            .expect("open memory store");
        (temp_dir, store)
    }

    #[test]
    fn stores_and_retrieves_fact_by_query() {
        let (_temp_dir, store) = test_store();
        let memory = store
            .store(StoreFactRequest {
                key: Some("preferred_editor".to_string()),
                value: "The user prefers modal editing.".to_string(),
                metadata: json!({ "source": "test" }),
                importance: 2.0,
            })
            .expect("store fact");

        let memories = store.retrieve("modal", 10).expect("retrieve memory");

        assert_eq!(memories, vec![memory]);
    }

    #[test]
    fn search_orders_by_recency() {
        let (_temp_dir, store) = test_store();
        store
            .store(StoreFactRequest {
                key: Some("alpha".to_string()),
                value: "Searchable project detail".to_string(),
                metadata: json!({}),
                importance: 100.0,
            })
            .expect("store first fact");
        std::thread::sleep(std::time::Duration::from_millis(2));
        let second = store
            .store(StoreFactRequest {
                key: Some("beta".to_string()),
                value: "Another searchable project detail".to_string(),
                metadata: json!({}),
                importance: 1.0,
            })
            .expect("store second fact");

        let memories = store.search("project", 1).expect("search memories");

        assert_eq!(memories, vec![second]);
    }

    #[test]
    fn persists_memories_across_store_reopen() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let database_path = temp_dir.path().join("memory.sqlite");
        let first_store = SqliteMemoryStore::open(&database_path).expect("open first memory store");
        first_store
            .store(StoreFactRequest {
                key: Some("timezone".to_string()),
                value: "The user works in Europe/London.".to_string(),
                metadata: json!({ "kind": "profile" }),
                importance: 3.0,
            })
            .expect("store fact");
        drop(first_store);

        let reopened_store =
            SqliteMemoryStore::open(&database_path).expect("open reopened memory store");
        let memories = reopened_store
            .retrieve("Europe/London", 10)
            .expect("retrieve persisted memory");

        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].key.as_deref(), Some("timezone"));
        assert_eq!(memories[0].metadata, json!({ "kind": "profile" }));
    }

    #[test]
    fn updates_fact_when_key_already_exists() {
        let (_temp_dir, store) = test_store();
        store
            .store(StoreFactRequest {
                key: Some("preference".to_string()),
                value: "Old value".to_string(),
                metadata: json!({}),
                importance: 1.0,
            })
            .expect("store first fact");
        store
            .store(StoreFactRequest {
                key: Some("preference".to_string()),
                value: "New value".to_string(),
                metadata: json!({ "updated": true }),
                importance: 5.0,
            })
            .expect("update fact");

        let memories = store.retrieve("value", 10).expect("retrieve memories");

        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].value, "New value");
        assert_eq!(memories[0].metadata, json!({ "updated": true }));
    }

    #[test]
    fn handles_tool_calls() {
        let (_temp_dir, store) = test_store();
        let server = MemoryServer::new(store);

        server
            .handle_tool_call(
                STORE_FACT,
                json!({
                    "key": "project",
                    "value": "The project uses Rust.",
                    "metadata": { "source": "unit-test" }
                }),
            )
            .expect("store through server");
        let response = server
            .handle_tool_call(RETRIEVE_MEMORIES, json!({ "query": "Rust" }))
            .expect("retrieve through server");

        assert_eq!(response["memories"][0]["key"], "project");
    }
}
