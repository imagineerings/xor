use std::path::PathBuf;

/// A named source root — a directory that the agent can reference.
#[derive(Clone, Debug)]
pub struct SourceRoot {
    pub name: String,
    pub path: PathBuf,
    pub priority: u8,
}

/// A collection of source roots with path resolution and priority ordering.
#[derive(Clone, Debug, Default)]
pub struct SourceRoots {
    roots: Vec<SourceRoot>,
}

impl SourceRoots {
    pub fn new() -> Self {
        Self { roots: Vec::new() }
    }

    /// Add a source root.
    pub fn add_root(&mut self, name: impl Into<String>, path: PathBuf, priority: u8) {
        self.roots.push(SourceRoot {
            name: name.into(),
            path,
            priority,
        });
    }

    /// Get a source root by name.
    pub fn get(&self, name: &str) -> Option<&SourceRoot> {
        self.roots.iter().find(|root| root.name == name)
    }

    /// Get a mutable reference to a source root by name.
    pub fn get_mut(&mut self, name: &str) -> Option<&mut SourceRoot> {
        self.roots.iter_mut().find(|root| root.name == name)
    }

    /// Resolve a path of the form `"root_name/relative/path"` to an absolute path.
    ///
    /// Returns `None` if the root name is unknown or the resolved path doesn't
    /// exist.
    pub fn resolve(&self, path: &str) -> Option<PathBuf> {
        let (root_name, relative) = path.split_once('/')?;
        let root = self.get(root_name)?;
        let resolved = root.path.join(relative);
        Some(resolved)
    }

    /// Remove a source root by name.
    pub fn remove(&mut self, name: &str) {
        self.roots.retain(|root| root.name != name);
    }

    /// Returns all roots sorted by priority (highest first).
    pub fn roots(&self) -> Vec<&SourceRoot> {
        let mut sorted: Vec<&SourceRoot> = self.roots.iter().collect();
        sorted.sort_by(|a, b| b.priority.cmp(&a.priority));
        sorted
    }

    /// Returns all roots sorted by priority (highest first), consuming self.
    pub fn into_roots(mut self) -> Vec<SourceRoot> {
        self.roots.sort_by(|a, b| b.priority.cmp(&a.priority));
        self.roots
    }

    /// Returns the number of roots.
    pub fn len(&self) -> usize {
        self.roots.len()
    }

    /// Returns true if there are no roots.
    pub fn is_empty(&self) -> bool {
        self.roots.is_empty()
    }
}

/// A named source referencing a file or directory within a source root.
#[derive(Clone, Debug)]
pub struct Source {
    pub name: String,
    pub root_name: String,
    pub path: PathBuf,
}

impl Source {
    pub fn new(name: impl Into<String>, root_name: impl Into<String>, path: PathBuf) -> Self {
        Self {
            name: name.into(),
            root_name: root_name.into(),
            path,
        }
    }

    /// Resolve this source to an absolute path using the given source roots.
    pub fn resolve(&self, roots: &SourceRoots) -> Option<PathBuf> {
        let root = roots.get(&self.root_name)?;
        Some(root.path.join(&self.path))
    }
}

/// A collection of named sources.
#[derive(Clone, Debug, Default)]
pub struct Sources {
    sources: Vec<Source>,
}

impl Sources {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    /// Register a named source.
    pub fn add(&mut self, source: Source) {
        self.sources.push(source);
    }

    /// Look up a source by name.
    pub fn get(&self, name: &str) -> Option<&Source> {
        self.sources.iter().find(|s| s.name == name)
    }

    /// Resolve a named source to an absolute path using the given roots.
    pub fn resolve(&self, source_name: &str, roots: &SourceRoots) -> Option<PathBuf> {
        let source = self.get(source_name)?;
        source.resolve(roots)
    }

    /// List all source names.
    pub fn names(&self) -> Vec<&str> {
        self.sources.iter().map(|s| s.name.as_str()).collect()
    }

    /// Returns the number of sources.
    pub fn len(&self) -> usize {
        self.sources.len()
    }

    /// Returns true if there are no sources.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn test_roots() -> SourceRoots {
        let mut roots = SourceRoots::new();
        roots.add_root("project", PathBuf::from("/home/user/project"), 10);
        roots.add_root("config", PathBuf::from("/etc/baymax"), 5);
        roots
    }

    #[test]
    fn test_add_and_get_root() {
        let mut roots = SourceRoots::new();
        roots.add_root("docs", PathBuf::from("/docs"), 1);
        let root = roots.get("docs").unwrap();
        assert_eq!(root.name, "docs");
        assert_eq!(root.path, Path::new("/docs"));
    }

    #[test]
    fn test_get_missing_root() {
        let roots = SourceRoots::new();
        assert!(roots.get("nonexistent").is_none());
    }

    #[test]
    fn test_resolve_simple() {
        let roots = test_roots();
        let resolved = roots.resolve("project/src/main.rs").unwrap();
        assert_eq!(resolved, Path::new("/home/user/project/src/main.rs"));
    }

    #[test]
    fn test_resolve_missing_root() {
        let roots = test_roots();
        assert!(roots.resolve("unknown/file.txt").is_none());
    }

    #[test]
    fn test_resolve_no_path() {
        let roots = test_roots();
        // No slash — split_once returns None
        assert!(roots.resolve("project").is_none());
    }

    #[test]
    fn test_priority_ordering() {
        let mut roots = SourceRoots::new();
        roots.add_root("low", PathBuf::from("/low"), 1);
        roots.add_root("high", PathBuf::from("/high"), 100);
        roots.add_root("med", PathBuf::from("/med"), 50);

        let ordered = roots.roots();
        assert_eq!(ordered[0].name, "high");
        assert_eq!(ordered[1].name, "med");
        assert_eq!(ordered[2].name, "low");
    }

    #[test]
    fn test_remove_root() {
        let mut roots = test_roots();
        assert!(roots.get("project").is_some());
        roots.remove("project");
        assert!(roots.get("project").is_none());
    }

    #[test]
    fn test_source_resolve() {
        let roots = test_roots();
        let source = Source::new("main.rs", "project", PathBuf::from("src/main.rs"));
        let resolved = source.resolve(&roots).unwrap();
        assert_eq!(resolved, Path::new("/home/user/project/src/main.rs"));
    }

    #[test]
    fn test_source_missing_root() {
        let roots = SourceRoots::new();
        let source = Source::new("orphan", "nonexistent", PathBuf::from("file.txt"));
        assert!(source.resolve(&roots).is_none());
    }

    #[test]
    fn test_sources_collection() {
        let roots = test_roots();
        let mut sources = Sources::new();
        sources.add(Source::new(
            "main.rs",
            "project",
            PathBuf::from("src/main.rs"),
        ));
        sources.add(Source::new(
            "config.toml",
            "config",
            PathBuf::from("settings.toml"),
        ));

        assert_eq!(sources.len(), 2);
        assert_eq!(sources.get("main.rs").unwrap().name, "main.rs");
        assert!(sources.get("missing").is_none());

        let resolved = sources.resolve("main.rs", &roots).unwrap();
        assert_eq!(resolved, Path::new("/home/user/project/src/main.rs"));
    }

    #[test]
    fn test_empty_roots() {
        let roots = SourceRoots::new();
        assert!(roots.is_empty());
        assert_eq!(roots.len(), 0);
    }

    #[test]
    fn test_into_roots_ordered() {
        let mut roots = SourceRoots::new();
        roots.add_root("z", PathBuf::from("/z"), 1);
        roots.add_root("a", PathBuf::from("/a"), 100);
        let ordered = roots.into_roots();
        assert_eq!(ordered[0].name, "a");
        assert_eq!(ordered[1].name, "z");
    }
}
