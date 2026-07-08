use mcp_tutorial::{Tutorial, TutorialCatalog, TutorialServer, TutorialStep};
use serde_json::json;
use tempfile::TempDir;

#[test]
fn loads_markdown_tutorials_from_directory() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let tutorial_path = temp_dir.path().join("first-agent-change.md");
    std::fs::write(
        &tutorial_path,
        "# First Agent Change\n\nWelcome.\n\n## Edit\nMake a change.\n\n## Verify\nRun tests.\n",
    )
    .expect("write tutorial");

    let catalog = TutorialCatalog::load_from_dir(temp_dir.path()).expect("load catalog");
    let summaries = catalog.list();

    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].id, "first-agent-change");
    assert_eq!(summaries[0].step_count, 3);
}

#[test]
fn advances_tutorial_via_tool_calls() {
    let catalog = TutorialCatalog::from_tutorials(vec![Tutorial {
        id: "sample".to_string(),
        title: "Sample".to_string(),
        path: "sample.md".into(),
        steps: vec![
            TutorialStep {
                index: 0,
                title: "One".to_string(),
                body: "First".to_string(),
            },
            TutorialStep {
                index: 1,
                title: "Two".to_string(),
                body: "Second".to_string(),
            },
        ],
    }])
    .expect("create catalog");
    let server = TutorialServer::new(catalog);

    let started = server
        .handle_tool_call("start_tutorial", json!({ "tutorial_id": "sample" }))
        .expect("start tutorial");
    let session_id = started["progress"]["session_id"]
        .as_str()
        .expect("session id");

    let current = server
        .handle_tool_call("current_step", json!({ "session_id": session_id }))
        .expect("current step");
    assert_eq!(current["step"]["title"], "One");

    let advanced = server
        .handle_tool_call("complete_step", json!({ "session_id": session_id }))
        .expect("complete step");
    assert_eq!(advanced["step"]["title"], "Two");
}
