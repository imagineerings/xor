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

/// Create, read, or append paragraphs to a DOCX document.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DocxToolInput {
    /// Operation to run: "create", "read", or "append_paragraph".
    pub operation: String,
    /// DOCX file path.
    pub path: PathBuf,
    /// Paragraph text for create or append_paragraph.
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocxToolOutput {
    pub success: bool,
    pub path: PathBuf,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl From<DocxToolOutput> for LanguageModelToolResultContent {
    fn from(output: DocxToolOutput) -> Self {
        serde_json::to_string(&output)
            .unwrap_or_else(|error| format!("failed to serialize DOCX tool output: {error}"))
            .into()
    }
}

pub struct DocxTool;

impl AgentTool for DocxTool {
    type Input = DocxToolInput;
    type Output = DocxToolOutput;

    const NAME: &'static str = "docx_tool";

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
                format!("DOCX {} {}", input.operation, MarkdownInlineCode(&path)).into()
            }
            Err(_) => "Work with DOCX".into(),
        }
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| DocxToolOutput {
                success: false,
                path: PathBuf::new(),
                message: format!("Failed to read DOCX tool input: {error}"),
                text: None,
            })?;
            authorize_path(Self::NAME, &input.path, "Use DOCX tool", &event_stream, cx)
                .await
                .map_err(|error| DocxToolOutput {
                    success: false,
                    path: input.path.clone(),
                    message: error.to_string(),
                    text: None,
                })?;
            cx.background_spawn(async move { run_docx_tool(input) })
                .await
                .map_err(|error| DocxToolOutput {
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

fn run_docx_tool(input: DocxToolInput) -> Result<DocxToolOutput> {
    match input.operation.as_str() {
        "create" => {
            let text = input.text.as_deref().context("DOCX create requires text")?;
            write_bytes(&input.path, &build_docx(&[text]))?;
            Ok(DocxToolOutput {
                success: true,
                path: input.path,
                message: "Created DOCX document".to_string(),
                text: None,
            })
        }
        "read" => {
            let bytes = fs::read(&input.path)
                .with_context(|| format!("failed to read DOCX {}", input.path.display()))?;
            let text = read_docx_text(&bytes)?;
            Ok(DocxToolOutput {
                success: true,
                path: input.path,
                message: "Read DOCX document".to_string(),
                text: Some(text),
            })
        }
        "append_paragraph" => {
            let bytes = fs::read(&input.path)
                .with_context(|| format!("failed to read DOCX {}", input.path.display()))?;
            let mut paragraphs = read_docx_paragraphs(&bytes)?;
            let text = input
                .text
                .as_deref()
                .context("DOCX append_paragraph requires text")?;
            paragraphs.push(text.to_string());
            let paragraph_refs = paragraphs.iter().map(String::as_str).collect::<Vec<_>>();
            write_bytes(&input.path, &build_docx(&paragraph_refs))?;
            Ok(DocxToolOutput {
                success: true,
                path: input.path,
                message: "Appended paragraph to DOCX document".to_string(),
                text: None,
            })
        }
        operation => bail!("unsupported DOCX operation: {operation}"),
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

fn build_docx(paragraphs: &[&str]) -> Vec<u8> {
    let body = paragraphs
        .iter()
        .map(|paragraph| {
            format!(
                "<w:p><w:r><w:t>{}</w:t></w:r></w:p>",
                quick_xml::escape::escape(*paragraph)
            )
        })
        .collect::<String>();
    let document = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body>{body}<w:sectPr/></w:body></w:document>"#
    );
    zip_store(&[
        (
            "[Content_Types].xml",
            br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#.as_ref(),
        ),
        (
            "_rels/.rels",
            br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#.as_ref(),
        ),
        ("word/document.xml", document.as_bytes()),
    ])
}

fn read_docx_text(bytes: &[u8]) -> Result<String> {
    Ok(read_docx_paragraphs(bytes)?.join("\n"))
}

fn read_docx_paragraphs(bytes: &[u8]) -> Result<Vec<String>> {
    let document = zip_entry(bytes, "word/document.xml")?;
    let document = String::from_utf8(document).context("word/document.xml is not UTF-8")?;
    let mut paragraphs = Vec::new();
    let mut cursor = document.as_str();
    while let Some(start) = cursor.find("<w:t>") {
        cursor = &cursor[start + "<w:t>".len()..];
        let Some(end) = cursor.find("</w:t>") else {
            bail!("malformed DOCX text run");
        };
        paragraphs.push(unescape_xml(&cursor[..end]));
        cursor = &cursor[end + "</w:t>".len()..];
    }
    if paragraphs.is_empty() {
        bail!("DOCX document contains no readable text runs");
    }
    Ok(paragraphs)
}

fn unescape_xml(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn zip_store(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut output = Vec::new();
    let mut central_directory = Vec::new();
    for (name, data) in entries {
        let local_offset = output.len() as u32;
        let crc = crc32(data);
        write_local_file_header(&mut output, name, crc, data.len() as u32);
        output.extend_from_slice(data);
        write_central_directory_header(
            &mut central_directory,
            name,
            crc,
            data.len() as u32,
            local_offset,
        );
    }
    let central_offset = output.len() as u32;
    output.extend_from_slice(&central_directory);
    write_end_of_central_directory(
        &mut output,
        entries.len() as u16,
        central_directory.len() as u32,
        central_offset,
    );
    output
}

fn zip_entry(bytes: &[u8], name: &str) -> Result<Vec<u8>> {
    let mut index = 0;
    while index + 30 <= bytes.len() {
        if bytes[index..index + 4] != [0x50, 0x4b, 0x03, 0x04] {
            break;
        }
        let method = read_u16(bytes, index + 8)?;
        let compressed_size = read_u32(bytes, index + 18)? as usize;
        let uncompressed_size = read_u32(bytes, index + 22)? as usize;
        let name_len = read_u16(bytes, index + 26)? as usize;
        let extra_len = read_u16(bytes, index + 28)? as usize;
        let data_start = index + 30 + name_len + extra_len;
        let data_end = data_start + compressed_size;
        if data_end > bytes.len() {
            bail!("ZIP entry extends past end of file");
        }
        let entry_name = std::str::from_utf8(&bytes[index + 30..index + 30 + name_len])
            .context("ZIP entry name is not UTF-8")?;
        if entry_name == name {
            if method != 0 {
                bail!("DOCX reader only supports stored ZIP entries");
            }
            let data = bytes[data_start..data_end].to_vec();
            if data.len() != uncompressed_size {
                bail!("ZIP entry size mismatch");
            }
            return Ok(data);
        }
        index = data_end;
    }
    bail!("DOCX is missing {name}")
}

fn write_local_file_header(output: &mut Vec<u8>, name: &str, crc: u32, size: u32) {
    output.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
    output.extend_from_slice(&20u16.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&crc.to_le_bytes());
    output.extend_from_slice(&size.to_le_bytes());
    output.extend_from_slice(&size.to_le_bytes());
    output.extend_from_slice(&(name.len() as u16).to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(name.as_bytes());
}

fn write_central_directory_header(
    output: &mut Vec<u8>,
    name: &str,
    crc: u32,
    size: u32,
    local_offset: u32,
) {
    output.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
    output.extend_from_slice(&20u16.to_le_bytes());
    output.extend_from_slice(&20u16.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&crc.to_le_bytes());
    output.extend_from_slice(&size.to_le_bytes());
    output.extend_from_slice(&size.to_le_bytes());
    output.extend_from_slice(&(name.len() as u16).to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&0u32.to_le_bytes());
    output.extend_from_slice(&local_offset.to_le_bytes());
    output.extend_from_slice(name.as_bytes());
}

fn write_end_of_central_directory(
    output: &mut Vec<u8>,
    entry_count: u16,
    central_size: u32,
    central_offset: u32,
) {
    output.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
    output.extend_from_slice(&entry_count.to_le_bytes());
    output.extend_from_slice(&entry_count.to_le_bytes());
    output.extend_from_slice(&central_size.to_le_bytes());
    output.extend_from_slice(&central_offset.to_le_bytes());
    output.extend_from_slice(&0u16.to_le_bytes());
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    let slice = bytes
        .get(offset..offset + 2)
        .context("unexpected end of ZIP file")?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    let slice = bytes
        .get(offset..offset + 4)
        .context("unexpected end of ZIP file")?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn docx_round_trips_paragraphs() {
        let docx = build_docx(&["hello", "a < b & c"]);
        assert_eq!(
            read_docx_text(&docx).unwrap(),
            "hello\na < b & c".to_string()
        );
        assert!(zip_entry(&docx, "[Content_Types].xml").is_ok());
    }
}
