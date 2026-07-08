use mcp_autovisualiser::{AutoVisualiserServer, OutputFormat, RenderVisualizationRequest};
use serde_json::json;
use tempfile::TempDir;

#[test]
fn renders_template_output_files() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let output_path = temp_dir.path().join("architecture.svg");
    let server = AutoVisualiserServer::default();

    let rendered = server
        .render_visualization(RenderVisualizationRequest {
            template: "architecture".to_string(),
            title: "MCP Servers".to_string(),
            output_path: output_path.clone(),
            output_format: Some(OutputFormat::Svg),
            nodes: vec![
                mcp_autovisualiser::DiagramNode {
                    id: "memory".to_string(),
                    label: "Memory".to_string(),
                },
                mcp_autovisualiser::DiagramNode {
                    id: "peekaboo".to_string(),
                    label: "Peekaboo".to_string(),
                },
            ],
            edges: vec![mcp_autovisualiser::DiagramEdge {
                from: "memory".to_string(),
                to: "peekaboo".to_string(),
                label: Some("shares runner".to_string()),
            }],
        })
        .expect("render visualization");

    assert_eq!(rendered.output_format, OutputFormat::Svg);
    assert!(rendered.content.starts_with("<svg"));
    assert_eq!(
        std::fs::read_to_string(output_path).expect("read output"),
        rendered.content
    );
}

#[test]
fn handles_render_visualization_tool_call() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let output_path = temp_dir.path().join("flow.mmd");
    let server = AutoVisualiserServer::default();

    let response = server
        .handle_tool_call(
            "render_visualization",
            json!({
                "template": "flow_chart",
                "title": "Flow",
                "output_path": output_path,
                "nodes": [
                    { "id": "start", "label": "Start" },
                    { "id": "finish", "label": "Finish" }
                ],
                "edges": [
                    { "from": "start", "to": "finish", "label": "done" }
                ]
            }),
        )
        .expect("render visualization");

    assert_eq!(response["visualization"]["template"], "flow_chart");
    assert_eq!(response["visualization"]["output_format"], "mermaid");
}
