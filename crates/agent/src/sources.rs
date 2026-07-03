use std::path::PathBuf;

/// The kind of content a named source provides.
#[derive(Clone, Debug)]
pub enum SourceKind {
    /// A single file whose contents are used as the source.
    File(PathBuf),
    /// A directory whose listing is used as the source.
    Directory(PathBuf),
    /// Inline content provided directly.
    Inline { content: String },
}

/// A named source of information that the agent can reference.
///
/// Sources are resolved by name, providing file content, directory
/// references, or inline text.
#[derive(Clone, Debug)]
pub struct Source {
    /// Unique name for this source (e.g. "config", "readme").
    pub name: String,
    /// The kind of source and its backing data.
    pub kind: SourceKind,
}

/// Manages a set of named sources that the agent can reference.
///
/// Sources are resolved by name to provide context — file contents,
/// directory listings, or inline text snippets.
#[derive(Clone, Debug, Default)]
pub struct Sources {
    sources: Vec<Source>,
}

impl Sources {
    /// Create an empty source registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new named source.
    ///
    /// If a source with the same name already exists, it is replaced.
    pub fn add_source(&mut self, name: impl Into<String>, kind: SourceKind) {
        let name = name.into();
        // Remove any existing source with the same name.
        self.sources.retain(|s| s.name != name);
        self.sources.push(Source { name, kind });
    }

    /// Remove a named source.
    pub fn remove_source(&mut self, name: &str) {
        self.sources.retain(|s| s.name != name);
    }

    /// Get a source by name.
    pub fn get(&self, name: &str) -> Option<&Source> {
        self.sources.iter().find(|s| s.name == name)
    }

    /// Resolve a named source to its content.
    ///
    /// For file sources, reads and returns the file contents.
    /// For inline sources, returns the inline content.
    /// For directory sources, returns a formatted listing.
    /// Returns `None` if the source is not found or cannot be read.
    pub fn resolve_content(&self, name: &str) -> Option<String> {
        let source = self.sources.iter().find(|s| s.name == name)?;
        match &source.kind {
            SourceKind::File(path) => std::fs::read_to_string(path).ok(),
            SourceKind::Directory(path) => {
                let entries: Vec<String> = std::fs::read_dir(path)
                    .ok()?
                    .filter_map(|e| {
                        e.ok().map(|e| {
                            let name = e.file_name().to_string_lossy().into_owned();
                            if e.file_type().ok().map_or(false, |t| t.is_dir()) {
                                format!("{name}/")
                            } else {
                                name
                            }
                        })
                    })
                    .collect();
                Some(entries.join("\n"))
            }
            SourceKind::Inline { content } => Some(content.clone()),
        }
    }

    /// List all registered sources.
    pub fn list_sources(&self) -> &[Source] {
        &self.sources
    }

    /// Returns `true` if there are no sources registered.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Returns the number of registered sources.
    pub fn len(&self) -> usize {
        self.sources.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_empty_sources() {
        let sources = Sources::new();
        assert!(sources.is_empty());
        assert!(sources.get("anything").is_none());
    }

    #[test]
    fn test_inline_source() {
        let mut sources = Sources::new();
        sources.add_source(
            "readme",
            SourceKind::Inline {
                content: "# Project".into(),
            },
        );
        assert_eq!(sources.len(), 1);
        assert_eq!(
            sources.resolve_content("readme").as_deref(),
            Some("# Project")
        );
    }

    #[test]
    fn test_file_source() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("config.json");
        fs::write(&file_path, r#"{"key": "value"}"#).unwrap();

        let mut sources = Sources::new();
        sources.add_source("config", SourceKind::File(file_path));
        assert_eq!(
            sources.resolve_content("config").as_deref(),
            Some(r#"{"key": "value"}"#)
        );
    }

    #[test]
    fn test_directory_source() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), "a").unwrap();
        fs::write(dir.path().join("b.rs"), "b").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();

        let mut sources = Sources::new();
        sources.add_source("files", SourceKind::Directory(dir.path().to_path_buf()));
        let content = sources.resolve_content("files").unwrap();
        assert!(content.contains("a.txt"));
        assert!(content.contains("b.rs"));
        assert!(content.contains("sub/"));
    }

    #[test]
    fn test_replace_existing_source() {
        let mut sources = Sources::new();
        sources.add_source(
            "greeting",
            SourceKind::Inline {
                content: "hello".into(),
            },
        );
        sources.add_source(
            "greeting",
            SourceKind::Inline {
                content: "world".into(),
            },
        );
        assert_eq!(sources.len(), 1);
        assert_eq!(
            sources.resolve_content("greeting").as_deref(),
            Some("world")
        );
    }

    #[test]
    fn test_remove_source() {
        let mut sources = Sources::new();
        sources.add_source(
            "temp",
            SourceKind::Inline {
                content: "data".into(),
            },
        );
        assert_eq!(sources.len(), 1);
        sources.remove_source("temp");
        assert!(sources.is_empty());
    }

    #[test]
    fn test_resolve_nonexistent() {
        let sources = Sources::new();
        assert!(sources.resolve_content("nonexistent").is_none());
    }
}
