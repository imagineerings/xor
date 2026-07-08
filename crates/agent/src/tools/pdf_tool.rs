use std::{fs, path::PathBuf, sync::Arc};

use agent_client_protocol::schema as acp;
use agent_settings::AgentSettings;
use anyhow::{Context as _, Result, bail};
use gpui::{App, AppContext as _, SharedString, Task};
use language_model::LanguageModelToolResultContent;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use settings::Settings as _;
use util::markdown::MarkdownInlineCode;

use crate::{
    AgentTool, ToolCallEventStream, ToolInput, ToolPermissionContext, ToolPermissionDecision,
    authorize_with_sensitive_settings, decide_permission_for_path,
};

/// Create, read, or append a text page to a simple PDF document.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PdfToolInput {
    /// Operation to run: "create", "read", or "append_page".
    pub operation: String,
    /// PDF file path.
    pub path: PathBuf,
    /// Text content for create or append_page.
    #[serde(default)]
    pub text: Option<String>,
    /// Optional document title for create.
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfToolOutput {
    pub success: bool,
    pub path: PathBuf,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl From<PdfToolOutput> for LanguageModelToolResultContent {
    fn from(output: PdfToolOutput) -> Self {
        serde_json::to_string(&output)
            .unwrap_or_else(|error| format!("failed to serialize PDF tool output: {error}"))
            .into()
    }
}

pub struct PdfTool;

impl AgentTool for PdfTool {
    type Input = PdfToolInput;
    type Output = PdfToolOutput;

    const NAME: &'static str = "pdf_tool";

    fn kind() -> acp::ToolKind {
        acp::ToolKind::Edit
    }

    fn initial_title(
        &self,
        input: Result<Self::Input, serde_json::Value>,
        _cx: &mut App,
    ) -> SharedString {
        match input {
            Ok(input) => {
                let path = input.path.display().to_string();
                format!("PDF {} {}", input.operation, MarkdownInlineCode(&path)).into()
            }
            Err(_) => "Work with PDF".into(),
        }
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| PdfToolOutput {
                success: false,
                path: PathBuf::new(),
                message: format!("Failed to read PDF tool input: {error}"),
                text: None,
            })?;
            authorize_path(Self::NAME, &input.path, "Use PDF tool", &event_stream, cx)
                .await
                .map_err(|error| PdfToolOutput {
                    success: false,
                    path: input.path.clone(),
                    message: error.to_string(),
                    text: None,
                })?;
            cx.background_spawn(async move { run_pdf_tool(input) })
                .await
                .map_err(|error| PdfToolOutput {
                    success: false,
                    path: PathBuf::new(),
                    message: error.to_string(),
                    text: None,
                })
        })
    }
}

fn authorize_path(
    tool_name: &str,
    path: &PathBuf,
    title: &str,
    event_stream: &ToolCallEventStream,
    cx: &mut gpui::AsyncApp,
) -> Task<Result<()>> {
    let path = path.display().to_string();
    cx.update(|cx| {
        match decide_permission_for_path(tool_name, &path, AgentSettings::get_global(cx)) {
            ToolPermissionDecision::Allow => Task::ready(Ok(())),
            ToolPermissionDecision::Deny(reason) => Task::ready(Err(anyhow::anyhow!(reason))),
            ToolPermissionDecision::Confirm => {
                let context = ToolPermissionContext::new(tool_name, vec![path]);
                authorize_with_sensitive_settings(None, context, title, event_stream, cx)
            }
        }
    })
}

fn run_pdf_tool(input: PdfToolInput) -> Result<PdfToolOutput> {
    match input.operation.as_str() {
        "create" => {
            let text = input.text.as_deref().context("PDF create requires text")?;
            let bytes = build_pdf(input.title.as_deref().unwrap_or("Document"), &[text]);
            write_bytes(&input.path, &bytes)?;
            Ok(PdfToolOutput {
                success: true,
                path: input.path,
                message: "Created PDF document".to_string(),
                text: None,
            })
        }
        "read" => {
            let bytes = fs::read(&input.path)
                .with_context(|| format!("failed to read PDF {}", input.path.display()))?;
            let text = extract_text_from_simple_pdf(&bytes)?;
            Ok(PdfToolOutput {
                success: true,
                path: input.path,
                message: "Read PDF document".to_string(),
                text: Some(text),
            })
        }
        "append_page" => {
            let existing = fs::read(&input.path)
                .with_context(|| format!("failed to read PDF {}", input.path.display()))?;
            let mut pages = extract_pages_from_simple_pdf(&existing)?;
            let text = input
                .text
                .as_deref()
                .context("PDF append_page requires text")?;
            pages.push(text.to_string());
            let page_refs = pages.iter().map(String::as_str).collect::<Vec<_>>();
            let bytes = build_pdf(input.title.as_deref().unwrap_or("Document"), &page_refs);
            write_bytes(&input.path, &bytes)?;
            Ok(PdfToolOutput {
                success: true,
                path: input.path,
                message: "Appended page to PDF document".to_string(),
                text: None,
            })
        }
        operation => bail!("unsupported PDF operation: {operation}"),
    }
}

fn write_bytes(path: &PathBuf, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))
}

fn build_pdf(title: &str, pages: &[&str]) -> Vec<u8> {
    let page_count = pages.len().max(1);
    let mut objects = Vec::new();
    objects.push("<< /Type /Catalog /Pages 2 0 R >>".to_string());
    let kids = (0..page_count)
        .map(|index| format!("{} 0 R", 3 + index * 2))
        .collect::<Vec<_>>()
        .join(" ");
    objects.push(format!(
        "<< /Type /Pages /Kids [{kids}] /Count {page_count} >>"
    ));

    for (index, page) in pages.iter().enumerate() {
        let page_object = 3 + index * 2;
        let content_object = page_object + 1;
        objects.push(format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Contents {content_object} 0 R >>"
        ));
        let stream = format!("BT /F1 12 Tf 72 720 Td ({}) Tj ET", escape_pdf_text(page));
        objects.push(format!(
            "<< /Length {} >>\nstream\n{}\nendstream",
            stream.len(),
            stream
        ));
    }

    let info_object = objects.len() + 1;
    objects.push(format!("<< /Title ({}) >>", escape_pdf_text(title)));

    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = Vec::new();
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
    }
    let xref_offset = pdf.len();
    pdf.extend_from_slice(
        format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes(),
    );
    for offset in offsets {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R /Info {info_object} 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    pdf
}

fn extract_text_from_simple_pdf(bytes: &[u8]) -> Result<String> {
    Ok(extract_pages_from_simple_pdf(bytes)?.join("\n\n"))
}

fn extract_pages_from_simple_pdf(bytes: &[u8]) -> Result<Vec<String>> {
    let source = String::from_utf8_lossy(bytes);
    if !source.starts_with("%PDF-") {
        bail!("file is not a PDF document");
    }
    let mut pages = Vec::new();
    let mut cursor = source.as_ref();
    while let Some(start) = cursor.find("BT /F1 12 Tf 72 720 Td (") {
        cursor = &cursor[start + "BT /F1 12 Tf 72 720 Td (".len()..];
        if let Some(end) = cursor.find(") Tj ET") {
            pages.push(unescape_pdf_text(&cursor[..end]));
            cursor = &cursor[end + ") Tj ET".len()..];
        } else {
            break;
        }
    }
    if pages.is_empty() {
        bail!("PDF text extraction only supports PDFs created by this tool");
    }
    Ok(pages)
}

fn escape_pdf_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)")
        .replace('\r', " ")
        .replace('\n', "\\n")
}

fn unescape_pdf_text(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => output.push('\n'),
                Some(other) => output.push(other),
                None => output.push(ch),
            }
        } else {
            output.push(ch);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_pdf_round_trips_text() {
        let pdf = build_pdf("Test", &["hello (pdf)"]);
        assert_eq!(extract_text_from_simple_pdf(&pdf).unwrap(), "hello (pdf)");
        assert!(pdf.starts_with(b"%PDF-1.4"));
    }
}
