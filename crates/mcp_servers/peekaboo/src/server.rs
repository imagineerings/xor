use std::sync::Arc;

use anyhow::{Context as _, Result, bail};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{NativeScreenCapture, Rect, ScreenCapture};

const LIST_DISPLAYS: &str = "list_displays";
const CAPTURE_SCREEN: &str = "capture_screen";
const CAPTURE_REGION: &str = "capture_region";

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Deserialize)]
struct CaptureScreenRequest {
    #[serde(default)]
    display: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct CaptureRegionRequest {
    pub region: Rect,
    #[serde(default)]
    pub display: Option<usize>,
}

pub struct PeekabooServer {
    capture: Arc<dyn ScreenCapture>,
}

impl Default for PeekabooServer {
    fn default() -> Self {
        Self::native()
    }
}

impl PeekabooServer {
    pub fn native() -> Self {
        Self::new(NativeScreenCapture)
    }

    pub fn new(capture: impl ScreenCapture + 'static) -> Self {
        Self {
            capture: Arc::new(capture),
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
                name: LIST_DISPLAYS.to_string(),
                description: "List displays available for screen capture.".to_string(),
                input_schema: empty_schema(),
            },
            ToolDescriptor {
                name: CAPTURE_SCREEN.to_string(),
                description: "Capture a full display as a PNG image.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "display": { "type": "integer", "minimum": 0 }
                    }
                }),
            },
            ToolDescriptor {
                name: CAPTURE_REGION.to_string(),
                description: "Capture a rectangular screen region as a PNG image.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "display": { "type": "integer", "minimum": 0 },
                        "region": {
                            "type": "object",
                            "properties": {
                                "x": { "type": "integer", "minimum": 0 },
                                "y": { "type": "integer", "minimum": 0 },
                                "width": { "type": "integer", "minimum": 1 },
                                "height": { "type": "integer", "minimum": 1 }
                            },
                            "required": ["x", "y", "width", "height"]
                        }
                    },
                    "required": ["region"]
                }),
            },
        ]
    }

    pub fn handle_tool_call(&self, name: &str, arguments: Value) -> Result<Value> {
        match name {
            LIST_DISPLAYS => Ok(json!({
                "displays": self.capture.displays()?,
                "formats": self.capture.supported_formats()
            })),
            CAPTURE_SCREEN => {
                let request: CaptureScreenRequest = serde_json::from_value(arguments)
                    .context("parsing capture_screen arguments")?;
                let capture = self.capture.capture_fullscreen(request.display)?;
                Ok(json!({ "capture": capture }))
            }
            CAPTURE_REGION => {
                let request: CaptureRegionRequest = serde_json::from_value(arguments)
                    .context("parsing capture_region arguments")?;
                let capture = self
                    .capture
                    .capture_region(request.region, request.display)?;
                Ok(json!({ "capture": capture }))
            }
            _ => bail!("unknown peekaboo tool `{name}`"),
        }
    }
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
    use crate::MockScreenCapture;

    #[test]
    fn lists_tools_and_displays() {
        let server = PeekabooServer::new(MockScreenCapture::default());

        let tools = server.tools();
        assert_eq!(tools.len(), 3);
        assert!(tools.iter().any(|tool| tool.name == CAPTURE_SCREEN));

        let displays = server
            .handle_tool_call(LIST_DISPLAYS, json!({}))
            .expect("list displays");
        assert_eq!(displays["displays"][0]["name"], "Mock display");
    }

    #[test]
    fn captures_fullscreen_with_mock_backend() {
        let server = PeekabooServer::new(MockScreenCapture::default());

        let capture = server
            .handle_tool_call(CAPTURE_SCREEN, json!({ "display": 0 }))
            .expect("capture screen");

        assert_eq!(capture["capture"]["format"], "png");
        assert_eq!(capture["capture"]["display"], 0);
        assert_eq!(capture["capture"]["region"], Value::Null);
    }

    #[test]
    fn captures_region_with_mock_backend() {
        let server = PeekabooServer::new(MockScreenCapture::default());

        let capture = server
            .handle_tool_call(
                CAPTURE_REGION,
                json!({
                    "region": { "x": 10, "y": 20, "width": 30, "height": 40 }
                }),
            )
            .expect("capture region");

        assert_eq!(capture["capture"]["region"]["width"], 30);
        assert_eq!(capture["capture"]["format"], "png");
    }

    #[test]
    fn rejects_empty_region() {
        let server = PeekabooServer::new(MockScreenCapture::default());

        let error = server
            .handle_tool_call(
                CAPTURE_REGION,
                json!({
                    "region": { "x": 10, "y": 20, "width": 0, "height": 40 }
                }),
            )
            .expect_err("zero-width region should fail");

        assert!(error.to_string().contains("width"));
    }
}
