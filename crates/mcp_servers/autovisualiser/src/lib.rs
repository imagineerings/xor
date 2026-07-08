use anyhow::{Context as _, Result, anyhow, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

const LIST_TEMPLATES: &str = "list_templates";
const RENDER_VISUALIZATION: &str = "render_visualization";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Mermaid,
    Svg,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VisualiserTemplate {
    pub name: String,
    pub description: String,
    pub output_format: OutputFormat,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagramNode {
    pub id: String,
    pub label: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DiagramEdge {
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RenderedVisualization {
    pub template: String,
    pub output_format: OutputFormat,
    pub output_path: PathBuf,
    pub content: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Deserialize)]
pub struct RenderVisualizationRequest {
    pub template: String,
    pub title: String,
    pub output_path: PathBuf,
    #[serde(default)]
    pub output_format: Option<OutputFormat>,
    #[serde(default)]
    pub nodes: Vec<DiagramNode>,
    #[serde(default)]
    pub edges: Vec<DiagramEdge>,
}

pub struct AutoVisualiserServer {
    templates: HashMap<String, VisualiserTemplate>,
}

impl Default for AutoVisualiserServer {
    fn default() -> Self {
        Self::new(default_templates())
    }
}

impl AutoVisualiserServer {
    pub fn new(templates: Vec<VisualiserTemplate>) -> Self {
        Self {
            templates: templates
                .into_iter()
                .map(|template| (template.name.clone(), template))
                .collect(),
        }
    }

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
                name: LIST_TEMPLATES.to_string(),
                description: "List available visualization templates.".to_string(),
                input_schema: empty_schema(),
            },
            ToolDescriptor {
                name: RENDER_VISUALIZATION.to_string(),
                description: "Render a visualization to a Mermaid or SVG diagram file.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "template": { "type": "string" },
                        "title": { "type": "string" },
                        "output_path": { "type": "string" },
                        "output_format": { "type": "string", "enum": ["mermaid", "svg"] },
                        "nodes": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "id": { "type": "string" },
                                    "label": { "type": "string" }
                                },
                                "required": ["id", "label"]
                            }
                        },
                        "edges": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "from": { "type": "string" },
                                    "to": { "type": "string" },
                                    "label": { "type": "string" }
                                },
                                "required": ["from", "to"]
                            }
                        }
                    },
                    "required": ["template", "title", "output_path", "nodes"]
                }),
            },
        ]
    }

    pub fn handle_tool_call(&self, name: &str, arguments: Value) -> Result<Value> {
        match name {
            LIST_TEMPLATES => Ok(json!({ "templates": self.list_templates() })),
            RENDER_VISUALIZATION => {
                let request: RenderVisualizationRequest = serde_json::from_value(arguments)
                    .context("parsing render_visualization arguments")?;
                let rendered = self.render_visualization(request)?;
                Ok(json!({ "visualization": rendered }))
            }
            _ => bail!("unknown autovisualiser tool `{name}`"),
        }
    }

    pub fn list_templates(&self) -> Vec<VisualiserTemplate> {
        let mut templates = self.templates.values().cloned().collect::<Vec<_>>();
        templates.sort_by(|left, right| left.name.cmp(&right.name));
        templates
    }

    pub fn render_visualization(
        &self,
        request: RenderVisualizationRequest,
    ) -> Result<RenderedVisualization> {
        let template = self
            .templates
            .get(&request.template)
            .ok_or_else(|| anyhow!("unknown visualization template `{}`", request.template))?;
        ensure!(
            !request.nodes.is_empty(),
            "visualization must include at least one node"
        );
        validate_edges(&request.nodes, &request.edges)?;

        let output_format = request.output_format.unwrap_or(template.output_format);
        let content = match output_format {
            OutputFormat::Mermaid => render_mermaid(&request),
            OutputFormat::Svg => render_svg(&request),
        };
        write_output(&request.output_path, &content)?;

        Ok(RenderedVisualization {
            template: template.name.clone(),
            output_format,
            output_path: request.output_path,
            content,
        })
    }
}

fn default_templates() -> Vec<VisualiserTemplate> {
    vec![
        VisualiserTemplate {
            name: "flow_chart".to_string(),
            description: "Render a directed flow chart.".to_string(),
            output_format: OutputFormat::Mermaid,
        },
        VisualiserTemplate {
            name: "architecture".to_string(),
            description: "Render a simple architecture diagram.".to_string(),
            output_format: OutputFormat::Svg,
        },
        VisualiserTemplate {
            name: "class_diagram".to_string(),
            description: "Render class or component relationships.".to_string(),
            output_format: OutputFormat::Mermaid,
        },
    ]
}

fn validate_edges(nodes: &[DiagramNode], edges: &[DiagramEdge]) -> Result<()> {
    let node_ids = nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    for edge in edges {
        ensure!(
            node_ids.contains(edge.from.as_str()),
            "edge references unknown source node `{}`",
            edge.from
        );
        ensure!(
            node_ids.contains(edge.to.as_str()),
            "edge references unknown target node `{}`",
            edge.to
        );
    }
    Ok(())
}

fn render_mermaid(request: &RenderVisualizationRequest) -> String {
    let mut output = format!("---\ntitle: {}\n---\nflowchart TD\n", request.title);
    for node in &request.nodes {
        output.push_str(&format!(
            "    {}[\"{}\"]\n",
            mermaid_id(&node.id),
            escape_mermaid_label(&node.label)
        ));
    }
    for edge in &request.edges {
        let label = edge
            .label
            .as_deref()
            .map(|label| format!("|{}|", escape_mermaid_label(label)))
            .unwrap_or_default();
        output.push_str(&format!(
            "    {} -->{} {}\n",
            mermaid_id(&edge.from),
            label,
            mermaid_id(&edge.to)
        ));
    }
    output
}

fn render_svg(request: &RenderVisualizationRequest) -> String {
    let width = 720usize;
    let row_height = 88usize;
    let height = 96 + request.nodes.len() * row_height;
    let mut output = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" viewBox="0 0 {width} {height}">"#
    );
    output.push_str(&format!(
        r##"<rect width="100%" height="100%" fill="#f8fafc"/><text x="32" y="44" font-family="sans-serif" font-size="24" font-weight="700" fill="#111827">{}</text>"##,
        escape_xml(&request.title)
    ));

    let positions = request
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.id.as_str(), (80usize, 80 + index * row_height)))
        .collect::<HashMap<_, _>>();

    for edge in &request.edges {
        if let (Some((from_x, from_y)), Some((to_x, to_y))) = (
            positions.get(edge.from.as_str()),
            positions.get(edge.to.as_str()),
        ) {
            let start_y = from_y + 28;
            let end_y = to_y + 28;
            output.push_str(&format!(
                r##"<line x1="{}" y1="{start_y}" x2="{}" y2="{end_y}" stroke="#64748b" stroke-width="2" marker-end="url(#arrow)"/>"##,
                from_x + 240,
                to_x
            ));
            if let Some(label) = &edge.label {
                output.push_str(&format!(
                    r##"<text x="360" y="{}" font-family="sans-serif" font-size="12" fill="#475569">{}</text>"##,
                    (start_y + end_y) / 2,
                    escape_xml(label)
                ));
            }
        }
    }

    output.push_str(
        r##"<defs><marker id="arrow" markerWidth="10" markerHeight="10" refX="8" refY="3" orient="auto" markerUnits="strokeWidth"><path d="M0,0 L0,6 L9,3 z" fill="#64748b"/></marker></defs>"##,
    );

    for (index, node) in request.nodes.iter().enumerate() {
        let y = 80 + index * row_height;
        output.push_str(&format!(
            r##"<rect x="80" y="{y}" width="240" height="56" rx="8" fill="#ffffff" stroke="#2563eb" stroke-width="2"/><text x="104" y="{}" font-family="sans-serif" font-size="16" fill="#111827">{}</text>"##,
            y + 34,
            escape_xml(&node.label)
        ));
    }

    output.push_str("</svg>");
    output
}

fn write_output(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("writing {}", path.display()))
}

fn mermaid_id(id: &str) -> String {
    let mut normalized = id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if normalized.is_empty() || normalized.starts_with(|character: char| character.is_ascii_digit())
    {
        normalized.insert(0, '_');
    }
    normalized
}

fn escape_mermaid_label(label: &str) -> String {
    label.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_xml(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn empty_schema() -> Value {
    json!({
        "type": "object",
        "properties": {}
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn request(output_path: PathBuf) -> RenderVisualizationRequest {
        RenderVisualizationRequest {
            template: "flow_chart".to_string(),
            title: "Agent Flow".to_string(),
            output_path,
            output_format: None,
            nodes: vec![
                DiagramNode {
                    id: "start".to_string(),
                    label: "Start".to_string(),
                },
                DiagramNode {
                    id: "finish".to_string(),
                    label: "Finish".to_string(),
                },
            ],
            edges: vec![DiagramEdge {
                from: "start".to_string(),
                to: "finish".to_string(),
                label: Some("done".to_string()),
            }],
        }
    }

    #[test]
    fn renders_mermaid_visualization_file() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let output_path = temp_dir.path().join("diagram.mmd");
        let server = AutoVisualiserServer::default();

        let rendered = server
            .render_visualization(request(output_path.clone()))
            .expect("render visualization");

        assert_eq!(rendered.output_format, OutputFormat::Mermaid);
        assert!(rendered.content.contains("flowchart TD"));
        assert_eq!(
            fs::read_to_string(output_path).expect("read output"),
            rendered.content
        );
    }

    #[test]
    fn renders_svg_visualization_file() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let output_path = temp_dir.path().join("diagram.svg");
        let server = AutoVisualiserServer::default();
        let mut request = request(output_path.clone());
        request.template = "architecture".to_string();

        let rendered = server
            .render_visualization(request)
            .expect("render visualization");

        assert_eq!(rendered.output_format, OutputFormat::Svg);
        assert!(rendered.content.starts_with("<svg"));
        assert!(
            fs::read_to_string(output_path)
                .expect("read output")
                .contains("Agent Flow")
        );
    }

    #[test]
    fn handles_tool_calls() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let output_path = temp_dir.path().join("diagram.mmd");
        let server = AutoVisualiserServer::default();

        let templates = server
            .handle_tool_call(LIST_TEMPLATES, json!({}))
            .expect("list templates");
        assert_eq!(
            templates["templates"].as_array().expect("templates").len(),
            3
        );

        let rendered = server
            .handle_tool_call(
                RENDER_VISUALIZATION,
                json!({
                    "template": "flow_chart",
                    "title": "Flow",
                    "output_path": output_path,
                    "nodes": [
                        { "id": "a", "label": "A" },
                        { "id": "b", "label": "B" }
                    ],
                    "edges": [
                        { "from": "a", "to": "b" }
                    ]
                }),
            )
            .expect("render visualization");
        assert_eq!(rendered["visualization"]["template"], "flow_chart");
    }

    #[test]
    fn rejects_edges_to_missing_nodes() {
        let temp_dir = TempDir::new().expect("create temp dir");
        let output_path = temp_dir.path().join("diagram.mmd");
        let server = AutoVisualiserServer::default();
        let mut request = request(output_path);
        request.edges[0].to = "missing".to_string();

        let error = server
            .render_visualization(request)
            .expect_err("missing edge target should fail");

        assert!(error.to_string().contains("unknown target node"));
    }
}
