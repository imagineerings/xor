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

/// Create or read a simple XLSX workbook with one worksheet.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct XlsxToolInput {
    /// Operation to run: "create" or "read".
    pub operation: String,
    /// XLSX file path.
    pub path: PathBuf,
    /// Rows for create. Each inner array is one worksheet row.
    #[serde(default)]
    pub rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XlsxToolOutput {
    pub success: bool,
    pub path: PathBuf,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<Vec<String>>>,
}

impl From<XlsxToolOutput> for LanguageModelToolResultContent {
    fn from(output: XlsxToolOutput) -> Self {
        serde_json::to_string(&output)
            .unwrap_or_else(|error| format!("failed to serialize XLSX tool output: {error}"))
            .into()
    }
}

pub struct XlsxTool;

impl AgentTool for XlsxTool {
    type Input = XlsxToolInput;
    type Output = XlsxToolOutput;

    const NAME: &'static str = "xlsx_tool";

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
                format!("XLSX {} {}", input.operation, MarkdownInlineCode(&path)).into()
            }
            Err(_) => "Work with XLSX".into(),
        }
    }

    fn run(
        self: Arc<Self>,
        input: ToolInput<Self::Input>,
        event_stream: ToolCallEventStream,
        cx: &mut App,
    ) -> Task<Result<Self::Output, Self::Output>> {
        cx.spawn(async move |cx| {
            let input = input.recv().await.map_err(|error| XlsxToolOutput {
                success: false,
                path: PathBuf::new(),
                message: format!("Failed to read XLSX tool input: {error}"),
                rows: None,
            })?;
            authorize_path(Self::NAME, &input.path, "Use XLSX tool", &event_stream, cx)
                .await
                .map_err(|error| XlsxToolOutput {
                    success: false,
                    path: input.path.clone(),
                    message: error.to_string(),
                    rows: None,
                })?;
            cx.background_spawn(async move { run_xlsx_tool(input) })
                .await
                .map_err(|error| XlsxToolOutput {
                    success: false,
                    path: PathBuf::new(),
                    message: error.to_string(),
                    rows: None,
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

fn run_xlsx_tool(input: XlsxToolInput) -> Result<XlsxToolOutput> {
    match input.operation.as_str() {
        "create" => {
            write_bytes(&input.path, &build_xlsx(&input.rows))?;
            Ok(XlsxToolOutput {
                success: true,
                path: input.path,
                message: "Created XLSX workbook".to_string(),
                rows: None,
            })
        }
        "read" => {
            let bytes = fs::read(&input.path)
                .with_context(|| format!("failed to read XLSX {}", input.path.display()))?;
            let rows = read_xlsx_rows(&bytes)?;
            Ok(XlsxToolOutput {
                success: true,
                path: input.path,
                message: "Read XLSX workbook".to_string(),
                rows: Some(rows),
            })
        }
        operation => bail!("unsupported XLSX operation: {operation}"),
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

fn build_xlsx(rows: &[Vec<String>]) -> Vec<u8> {
    let sheet_data = rows
        .iter()
        .enumerate()
        .map(|(row_index, row)| {
            let row_number = row_index + 1;
            let cells = row
                .iter()
                .enumerate()
                .map(|(column_index, value)| {
                    let reference = cell_reference(column_index, row_number);
                    format!(
                        r#"<c r="{reference}" t="inlineStr"><is><t>{}</t></is></c>"#,
                        quick_xml::escape::escape(value)
                    )
                })
                .collect::<String>();
            format!(r#"<row r="{row_number}">{cells}</row>"#)
        })
        .collect::<String>();
    let worksheet = format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>{sheet_data}</sheetData></worksheet>"#
    );
    zip_store(&[
        (
            "[Content_Types].xml",
            br#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#.as_ref(),
        ),
        (
            "_rels/.rels",
            br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#.as_ref(),
        ),
        (
            "xl/workbook.xml",
            br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>"#.as_ref(),
        ),
        (
            "xl/_rels/workbook.xml.rels",
            br#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#.as_ref(),
        ),
        ("xl/worksheets/sheet1.xml", worksheet.as_bytes()),
    ])
}

fn read_xlsx_rows(bytes: &[u8]) -> Result<Vec<Vec<String>>> {
    let sheet = zip_entry(bytes, "xl/worksheets/sheet1.xml")?;
    let sheet = String::from_utf8(sheet).context("xl/worksheets/sheet1.xml is not UTF-8")?;
    let mut rows = Vec::new();
    let mut cursor = sheet.as_str();
    while let Some(row_start) = cursor.find("<row ") {
        cursor = &cursor[row_start..];
        let Some(row_open_end) = cursor.find('>') else {
            bail!("malformed XLSX row");
        };
        cursor = &cursor[row_open_end + 1..];
        let Some(row_end) = cursor.find("</row>") else {
            bail!("malformed XLSX row");
        };
        let mut row_values = Vec::new();
        let mut row_cursor = &cursor[..row_end];
        while let Some(text_start) = row_cursor.find("<t>") {
            row_cursor = &row_cursor[text_start + "<t>".len()..];
            let Some(text_end) = row_cursor.find("</t>") else {
                bail!("malformed XLSX cell text");
            };
            row_values.push(unescape_xml(&row_cursor[..text_end]));
            row_cursor = &row_cursor[text_end + "</t>".len()..];
        }
        rows.push(row_values);
        cursor = &cursor[row_end + "</row>".len()..];
    }
    Ok(rows)
}

fn cell_reference(column_index: usize, row_number: usize) -> String {
    let mut column_number = column_index + 1;
    let mut column = Vec::new();
    while column_number > 0 {
        let remainder = (column_number - 1) % 26;
        column.push((b'A' + remainder as u8) as char);
        column_number = (column_number - 1) / 26;
    }
    column.iter().rev().collect::<String>() + &row_number.to_string()
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
                bail!("XLSX reader only supports stored ZIP entries");
            }
            let data = bytes[data_start..data_end].to_vec();
            if data.len() != uncompressed_size {
                bail!("ZIP entry size mismatch");
            }
            return Ok(data);
        }
        index = data_end;
    }
    bail!("XLSX is missing {name}")
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
    fn xlsx_round_trips_rows() {
        let rows = vec![
            vec!["name".to_string(), "count".to_string()],
            vec!["a < b".to_string(), "2".to_string()],
        ];
        let xlsx = build_xlsx(&rows);
        assert_eq!(read_xlsx_rows(&xlsx).unwrap(), rows);
        assert_eq!(cell_reference(27, 3), "AB3");
    }
}
