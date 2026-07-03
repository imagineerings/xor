use std::path::PathBuf;

/// A named source root directory with an associated priority.
///
/// Source roots let the agent know where to find relevant code or content
/// in a project. When a relative path is resolved, roots with higher
/// priority are checked first.
#[derive(Clone, Debug)]
pub struct SourceRoot {
    /// Human-readable name (e.g. "src", "config", "docs").
    pub name: String,
    /// Absolute or project-relative path to the root directory.
    pub path: PathBuf,
    /// Resolution priority — higher values take precedence.
    pub priority: u8,
}

/// Manages a set of source roots and resolves relative paths against them.
///
/// When resolving a path, roots are searched in descending priority order.
/// If no root has the path, the highest-priority root is used as the base.
#[derive(Clone, Debug, Default)]
pub struct SourceRoots {
    roots: Vec<SourceRoot>,
}

impl SourceRoots {
    /// Create an empty set of source roots.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a source root with the default priority (0).
    pub fn add_root(&mut self, name: impl Into<String>, path: PathBuf) {
        self.roots.push(SourceRoot {
            name: name.into(),
            path,
            priority: 0,
        });
    }

    /// Add a source root with an explicit priority.
    pub fn add_root_with_priority(&mut self, name: impl Into<String>, path: PathBuf, priority: u8) {
        self.roots.push(SourceRoot {
            name: name.into(),
            path,
            priority,
        });
    }

    /// Remove a source root by name.
    pub fn remove_root(&mut self, name: &str) {
        self.roots.retain(|r| r.name != name);
    }

    /// Resolve a relative path against source roots.
    ///
    /// Returns the first existing path found among the roots. If no root
    /// has the relative path, returns the path joined against the highest-
    /// priority root (or `None` if there are no roots).
    pub fn resolve(&self, relative: &str) -> Option<PathBuf> {
        // First, try to find an existing path among all roots.
        for root in &self.roots {
            let candidate = root.path.join(relative);
            if candidate.exists() {
                return Some(candidate);
            }
        }

        // Fall back to the highest-priority root.
        self.roots
            .iter()
            .max_by_key(|r| r.priority)
            .map(|root| root.path.join(relative))
    }

    /// Get a source root by name.
    pub fn get_root(&self, name: &str) -> Option<&SourceRoot> {
        self.roots.iter().find(|r| r.name == name)
    }

    /// List all registered source roots.
    pub fn list_roots(&self) -> &[SourceRoot] {
        &self.roots
    }

    /// Returns `true` if there are no source roots.
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }

    /// Returns the number of source roots.
    pub fn len(&self) -> usize {
        self.roots.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_empty_roots() {
        let roots = SourceRoots::new();
        assert!(roots.is_empty());
        assert_eq!(roots.len(), 0);
        assert!(roots.resolve("foo.txt").is_none());
    }

    #[test]
    fn test_add_and_remove() {
        let mut roots = SourceRoots::new();
        roots.add_root("src", PathBuf::from("/project/src"));
        assert_eq!(roots.len(), 1);
        roots.remove_root("src");
        assert!(roots.is_empty());
    }

    #[test]
    fn test_resolve_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("readme.md");
        fs::write(&file_path, "hello").unwrap();

        let mut roots = SourceRoots::new();
        roots.add_root("root", dir.path().to_path_buf());
        let resolved = roots.resolve("readme.md");
        assert_eq!(resolved, Some(file_path));
    }

    #[test]
    fn test_resolve_nonexistent_falls_back() {
        let mut roots = SourceRoots::new();
        roots.add_root("src", PathBuf::from("/project/src"));
        // A non-existent path should still resolve (just not exist)
        let resolved = roots.resolve("missing.rs");
        assert_eq!(resolved, Some(PathBuf::from("/project/src/missing.rs")));
    }

    #[test]
    fn test_priority_ordering() {
        let mut roots = SourceRoots::new();
        roots.add_root_with_priority("override", PathBuf::from("/override"), 10);
        roots.add_root_with_priority("base", PathBuf::from("/base"), 0);

        // The highest-priority root should be used for fallback resolution
        let resolved = roots.resolve("file.txt");
        assert_eq!(resolved, Some(PathBuf::from("/override/file.txt")));
    }

    #[test]
    fn test_get_root_by_name() {
        let mut roots = SourceRoots::new();
        roots.add_root("docs", PathBuf::from("/project/docs"));
        let root = roots.get_root("docs");
        assert!(root.is_some());
        assert_eq!(root.unwrap().name, "docs");
        assert!(roots.get_root("nonexistent").is_none());
    }
}
