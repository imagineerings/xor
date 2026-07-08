use mcp_memory::{MemoryServer, MemoryStore as _, SqliteMemoryStore, StoreFactRequest};
use serde_json::json;
use tempfile::TempDir;

#[test]
fn persists_and_retrieves_memories_across_reopen() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let database_path = temp_dir.path().join("memory.sqlite");
    let store = SqliteMemoryStore::open(&database_path).expect("open memory store");

    store
        .store(StoreFactRequest {
            key: Some("timezone".to_string()),
            value: "The user works in Europe/London.".to_string(),
            metadata: json!({ "source": "integration" }),
            importance: 3.0,
        })
        .expect("store fact");
    drop(store);

    let reopened_store = SqliteMemoryStore::open(&database_path).expect("reopen memory store");
    let memories = reopened_store
        .retrieve("Europe/London", 10)
        .expect("retrieve persisted fact");

    assert_eq!(memories.len(), 1);
    assert_eq!(memories[0].key.as_deref(), Some("timezone"));
    assert_eq!(memories[0].metadata, json!({ "source": "integration" }));
}

#[test]
fn handles_store_and_search_tool_calls() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let store =
        SqliteMemoryStore::open(temp_dir.path().join("memory.sqlite")).expect("open memory store");
    let server = MemoryServer::new(store);

    server
        .handle_tool_call(
            "store_fact",
            json!({
                "key": "project-language",
                "value": "The MCP server tests are written in Rust.",
                "metadata": { "scope": "test" },
                "importance": 2.0
            }),
        )
        .expect("store fact");

    let response = server
        .handle_tool_call("search_memories", json!({ "query": "Rust", "limit": 5 }))
        .expect("search memories");

    assert_eq!(response["memories"][0]["key"], "project-language");
    assert_eq!(response["memories"][0]["metadata"]["scope"], "test");
}
