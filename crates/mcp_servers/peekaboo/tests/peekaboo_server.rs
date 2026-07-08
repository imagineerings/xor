use anyhow::Result;
use mcp_peekaboo::{
    DisplayInfo, ImageFormat, Rect, ScreenCapture, ScreenCaptureResult, server::PeekabooServer,
};
use serde_json::{Value, json};

struct TestCapture;

impl ScreenCapture for TestCapture {
    fn capture_fullscreen(&self, display: Option<usize>) -> Result<ScreenCaptureResult> {
        Ok(ScreenCaptureResult {
            data: b"\x89PNG\r\n\x1a\nfullscreen".to_vec(),
            format: ImageFormat::Png,
            region: None,
            display,
        })
    }

    fn capture_region(&self, region: Rect, display: Option<usize>) -> Result<ScreenCaptureResult> {
        Ok(ScreenCaptureResult {
            data: b"\x89PNG\r\n\x1a\nregion".to_vec(),
            format: ImageFormat::Png,
            region: Some(region),
            display,
        })
    }

    fn displays(&self) -> Result<Vec<DisplayInfo>> {
        Ok(vec![DisplayInfo {
            id: 2,
            name: "Integration display".to_string(),
            primary: true,
        }])
    }
}

#[test]
fn lists_displays_and_supported_formats() {
    let server = PeekabooServer::new(TestCapture);

    let response = server
        .handle_tool_call("list_displays", json!({}))
        .expect("list displays");

    assert_eq!(response["displays"][0]["id"], 2);
    assert_eq!(response["formats"][0], "png");
}

#[test]
fn captures_fullscreen_and_region() {
    let server = PeekabooServer::new(TestCapture);

    let fullscreen = server
        .handle_tool_call("capture_screen", json!({ "display": 2 }))
        .expect("capture fullscreen");
    assert_eq!(fullscreen["capture"]["display"], 2);
    assert_eq!(fullscreen["capture"]["region"], Value::Null);

    let region = server
        .handle_tool_call(
            "capture_region",
            json!({
                "display": 2,
                "region": { "x": 1, "y": 2, "width": 3, "height": 4 }
            }),
        )
        .expect("capture region");

    assert_eq!(region["capture"]["region"]["height"], 4);
    assert_eq!(region["capture"]["format"], "png");
}
