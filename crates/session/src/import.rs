pub mod formats;

use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use formats::{GooseLegacyFormat, JsonImportFormat, MarkdownImportFormat};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedSession {
    pub title: String,
    pub messages: Vec<ImportedMessage>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedMessage {
    pub role: ImportedRole,
    pub content: String,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportedRole {
    User,
    Assistant,
    System,
    Tool,
}

impl ImportedRole {
    pub fn parse(role: &str) -> Result<Self> {
        match role.trim().to_ascii_lowercase().as_str() {
            "user" | "human" => Ok(Self::User),
            "assistant" | "agent" | "ai" => Ok(Self::Assistant),
            "system" => Ok(Self::System),
            "tool" | "tool_result" => Ok(Self::Tool),
            role => bail!("unsupported imported message role `{role}`"),
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::System => "system",
            Self::Tool => "tool",
        }
    }
}

pub trait ImportFormat: Send + Sync {
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn detect(&self, content: &[u8], extension: Option<&str>) -> bool;
    fn import(&self, content: &[u8]) -> Result<ImportedSession>;
}

pub struct SessionImporter {
    formats: Vec<Box<dyn ImportFormat>>,
}

impl Default for SessionImporter {
    fn default() -> Self {
        Self::new(vec![
            Box::new(GooseLegacyFormat),
            Box::new(JsonImportFormat),
            Box::new(MarkdownImportFormat),
        ])
    }
}

impl SessionImporter {
    pub fn new(formats: Vec<Box<dyn ImportFormat>>) -> Self {
        Self { formats }
    }

    pub fn formats(&self) -> &[Box<dyn ImportFormat>] {
        &self.formats
    }

    pub fn detect_format(
        &self,
        content: &[u8],
        extension: Option<&str>,
    ) -> Option<&dyn ImportFormat> {
        let extension = extension.map(normalize_extension);
        self.formats
            .iter()
            .map(Box::as_ref)
            .find(|format| format.detect(content, extension.as_deref()))
    }

    pub fn import(&self, content: &[u8], extension: Option<&str>) -> Result<ImportedSession> {
        let format = self.detect_format(content, extension).ok_or_else(|| {
            anyhow!(
                "unrecognized session import format; supported formats: {}",
                self.supported_formats().join(", ")
            )
        })?;
        let session = format.import(content)?;
        validate_imported_session(&session)?;
        Ok(session)
    }

    pub fn supported_formats(&self) -> Vec<&'static str> {
        self.formats.iter().map(|format| format.name()).collect()
    }
}

pub fn validate_imported_session(session: &ImportedSession) -> Result<()> {
    if session.title.trim().is_empty() {
        bail!("imported session title cannot be empty");
    }
    if session.messages.is_empty() {
        bail!("imported session must contain at least one message");
    }
    for (index, message) in session.messages.iter().enumerate() {
        if message.content.trim().is_empty() {
            bail!("imported message {index} cannot be empty");
        }
    }
    Ok(())
}

fn normalize_extension(extension: &str) -> String {
    extension
        .trim()
        .trim_start_matches('.')
        .to_ascii_lowercase()
}

pub(crate) fn extension_matches(extension: Option<&str>, candidates: &[&str]) -> bool {
    extension.is_some_and(|extension| candidates.contains(&extension))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn importer_detects_markdown_by_extension() {
        let importer = SessionImporter::default();
        let format = importer
            .detect_format(b"# Chat\n\n## User\nHello", Some(".md"))
            .expect("detect markdown");

        assert_eq!(format.name(), "markdown");
    }

    #[test]
    fn importer_returns_supported_formats_for_unknown_content() {
        let importer = SessionImporter::default();
        let error = importer
            .import(b"not a session", Some("txt"))
            .expect_err("unknown format should fail");

        assert!(error.to_string().contains("goose_legacy"));
        assert!(error.to_string().contains("markdown"));
    }
}
