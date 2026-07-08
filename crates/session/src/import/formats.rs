use anyhow::{Context as _, Result};
use serde::Deserialize;
use serde_json::{Value, json};

use super::{
    ImportFormat, ImportedMessage, ImportedRole, ImportedSession, extension_matches,
    validate_imported_session,
};

pub struct JsonImportFormat;
pub struct MarkdownImportFormat;
pub struct GooseLegacyFormat;

impl ImportFormat for JsonImportFormat {
    fn name(&self) -> &'static str {
        "json"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["json"]
    }

    fn detect(&self, content: &[u8], extension: Option<&str>) -> bool {
        extension_matches(extension, self.extensions()) || looks_like_json(content)
    }

    fn import(&self, content: &[u8]) -> Result<ImportedSession> {
        let value: Value = serde_json::from_slice(content).context("parsing JSON session")?;
        let session = parse_json_session(value)?;
        validate_imported_session(&session)?;
        Ok(session)
    }
}

impl ImportFormat for MarkdownImportFormat {
    fn name(&self) -> &'static str {
        "markdown"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["md", "markdown"]
    }

    fn detect(&self, content: &[u8], extension: Option<&str>) -> bool {
        extension_matches(extension, self.extensions())
            || std::str::from_utf8(content).is_ok_and(|content| {
                content.lines().any(|line| {
                    let line = line.trim();
                    line.starts_with("## User")
                        || line.starts_with("## Assistant")
                        || line.starts_with("User:")
                        || line.starts_with("Assistant:")
                })
            })
    }

    fn import(&self, content: &[u8]) -> Result<ImportedSession> {
        let markdown = std::str::from_utf8(content).context("markdown session is not UTF-8")?;
        let session = parse_markdown_session(markdown)?;
        validate_imported_session(&session)?;
        Ok(session)
    }
}

impl ImportFormat for GooseLegacyFormat {
    fn name(&self) -> &'static str {
        "goose_legacy"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["json", "goose"]
    }

    fn detect(&self, content: &[u8], extension: Option<&str>) -> bool {
        if !extension_matches(extension, self.extensions()) && !looks_like_json(content) {
            return false;
        }
        let Ok(value) = serde_json::from_slice::<Value>(content) else {
            return false;
        };
        value
            .get("messages")
            .and_then(Value::as_array)
            .is_some_and(|messages| {
                messages.iter().any(|message| {
                    message.get("segments").is_some()
                        || message.get("tool_requests").is_some()
                        || message.get("tool_results").is_some()
                })
            })
    }

    fn import(&self, content: &[u8]) -> Result<ImportedSession> {
        let value: GooseLegacySession =
            serde_json::from_slice(content).context("parsing Goose legacy session")?;
        let title = value
            .title
            .filter(|title| !title.trim().is_empty())
            .unwrap_or_else(|| "Imported Goose Session".to_string());
        let mut messages = Vec::new();
        for message in value.messages {
            let role = ImportedRole::parse(&message.role)?;
            let content = goose_message_content(&message);
            if content.trim().is_empty() {
                continue;
            }
            messages.push(ImportedMessage {
                role,
                content,
                timestamp: message.timestamp,
                metadata: json!({
                    "source": "goose_legacy",
                    "id": message.id
                }),
            });
        }
        let session = ImportedSession {
            title,
            messages,
            metadata: json!({
                "source": "goose_legacy",
                "version": value.version
            }),
        };
        validate_imported_session(&session)?;
        Ok(session)
    }
}

#[derive(Deserialize)]
struct GooseLegacySession {
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    messages: Vec<GooseLegacyMessage>,
}

#[derive(Deserialize)]
struct GooseLegacyMessage {
    #[serde(default)]
    id: Value,
    role: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    segments: Vec<GooseLegacySegment>,
    #[serde(default)]
    timestamp: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum GooseLegacySegment {
    Text {
        text: String,
    },
    Thinking {
        text: String,
    },
    RedactedThinking {
        data: String,
    },
    #[serde(other)]
    Other,
}

fn parse_json_session(value: Value) -> Result<ImportedSession> {
    if value.is_array() {
        let messages = parse_json_messages(value)?;
        return Ok(ImportedSession {
            title: "Imported JSON Session".to_string(),
            messages,
            metadata: json!({ "source": "json" }),
        });
    }

    let title = value
        .get("title")
        .and_then(Value::as_str)
        .filter(|title| !title.trim().is_empty())
        .unwrap_or("Imported JSON Session")
        .to_string();
    let messages_value = value
        .get("messages")
        .cloned()
        .context("JSON session must include a messages array")?;
    let messages = parse_json_messages(messages_value)?;
    Ok(ImportedSession {
        title,
        messages,
        metadata: json!({ "source": "json" }),
    })
}

fn parse_json_messages(value: Value) -> Result<Vec<ImportedMessage>> {
    let messages = value
        .as_array()
        .context("JSON session messages must be an array")?;
    messages
        .iter()
        .enumerate()
        .map(|(index, message)| {
            let role = message
                .get("role")
                .and_then(Value::as_str)
                .with_context(|| format!("JSON message {index} is missing role"))?;
            let content = message
                .get("content")
                .or_else(|| message.get("text"))
                .and_then(Value::as_str)
                .with_context(|| format!("JSON message {index} is missing content"))?;
            Ok(ImportedMessage {
                role: ImportedRole::parse(role)?,
                content: content.to_string(),
                timestamp: message
                    .get("timestamp")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                metadata: json!({ "source": "json" }),
            })
        })
        .collect()
}

fn parse_markdown_session(markdown: &str) -> Result<ImportedSession> {
    let mut title = "Imported Markdown Session".to_string();
    let mut messages = Vec::new();
    let mut current_role: Option<ImportedRole> = None;
    let mut current_content = String::new();

    for line in markdown.lines() {
        let trimmed = line.trim();
        if let Some(markdown_title) = trimmed.strip_prefix("# ") {
            if !markdown_title.trim().is_empty() {
                title = markdown_title.trim().to_string();
            }
            continue;
        }

        if let Some(role) = markdown_heading_role(trimmed).or_else(|| colon_role(trimmed)) {
            push_markdown_message(&mut messages, current_role.take(), &mut current_content);
            current_role = Some(role);
            if colon_role(trimmed).is_some()
                && let Some((_, content)) = trimmed.split_once(':')
                && !content.trim().is_empty()
            {
                current_content.push_str(content.trim());
                current_content.push('\n');
            }
            continue;
        }

        if current_role.is_some() {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }
    push_markdown_message(&mut messages, current_role, &mut current_content);

    let session = ImportedSession {
        title,
        messages,
        metadata: json!({ "source": "markdown" }),
    };
    validate_imported_session(&session)?;
    Ok(session)
}

fn markdown_heading_role(line: &str) -> Option<ImportedRole> {
    let heading = line.strip_prefix("## ")?;
    ImportedRole::parse(heading.trim()).ok()
}

fn colon_role(line: &str) -> Option<ImportedRole> {
    let (role, _) = line.split_once(':')?;
    ImportedRole::parse(role.trim()).ok()
}

fn push_markdown_message(
    messages: &mut Vec<ImportedMessage>,
    role: Option<ImportedRole>,
    content: &mut String,
) {
    let Some(role) = role else {
        content.clear();
        return;
    };
    let content = std::mem::take(content);
    let content = content.trim().to_string();
    if content.is_empty() {
        return;
    }
    messages.push(ImportedMessage {
        role,
        content,
        timestamp: None,
        metadata: json!({ "source": "markdown" }),
    });
}

fn goose_message_content(message: &GooseLegacyMessage) -> String {
    let mut parts = Vec::new();
    if let Some(text) = message.text.as_deref().or(message.content.as_deref())
        && !text.trim().is_empty()
    {
        parts.push(text.trim().to_string());
    }
    for segment in &message.segments {
        match segment {
            GooseLegacySegment::Text { text } | GooseLegacySegment::Thinking { text } => {
                if !text.trim().is_empty() {
                    parts.push(text.trim().to_string());
                }
            }
            GooseLegacySegment::RedactedThinking { data } => {
                if !data.trim().is_empty() {
                    parts.push("[redacted thinking]".to_string());
                }
            }
            GooseLegacySegment::Other => {}
        }
    }
    parts.join("\n\n")
}

fn looks_like_json(content: &[u8]) -> bool {
    content
        .iter()
        .copied()
        .find(|byte| !byte.is_ascii_whitespace())
        .is_some_and(|byte| byte == b'{' || byte == b'[')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_json_session() {
        let session = JsonImportFormat
            .import(
                br#"{
                    "title": "JSON Chat",
                    "messages": [
                        { "role": "user", "content": "Hello" },
                        { "role": "assistant", "content": "Hi there" }
                    ]
                }"#,
            )
            .expect("import JSON");

        assert_eq!(session.title, "JSON Chat");
        assert_eq!(session.messages[0].role, ImportedRole::User);
        assert_eq!(session.messages[1].content, "Hi there");
    }

    #[test]
    fn imports_markdown_session() {
        let session = MarkdownImportFormat
            .import(b"# Markdown Chat\n\n## User\nHello\n\n## Assistant\nHi\n")
            .expect("import markdown");

        assert_eq!(session.title, "Markdown Chat");
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].content, "Hello");
    }

    #[test]
    fn imports_goose_legacy_segments() {
        let session = GooseLegacyFormat
            .import(
                br#"{
                    "version": "1.0",
                    "messages": [
                        {
                            "id": 1,
                            "role": "user",
                            "segments": [{ "type": "text", "text": "Legacy hello" }]
                        }
                    ]
                }"#,
            )
            .expect("import goose legacy");

        assert_eq!(session.metadata["source"], "goose_legacy");
        assert_eq!(session.messages[0].content, "Legacy hello");
    }

    #[test]
    fn rejects_empty_imports() {
        let error = JsonImportFormat
            .import(br#"{ "messages": [] }"#)
            .expect_err("empty import should fail");

        assert!(error.to_string().contains("at least one message"));
    }
}
