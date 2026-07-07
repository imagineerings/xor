use std::path::{Path, PathBuf};
use std::sync::Arc;

use fs::Fs;
use futures::StreamExt;
use util::paths::home_dir;

use super::{SIM_HINTS_FILE_NAMES, GLOBAL_HINTS_DIR_NAME, Hint, HintLoadError, HintSource};

/// Discovers and loads `.simhints` files from global
/// (`~/.config/sim/hints/`) and project locations.
pub struct HintLoader {
    fs: Arc<dyn Fs>,
}

impl HintLoader {
    pub fn new(fs: Arc<dyn Fs>) -> Self {
        Self { fs }
    }

    /// Returns the path to the global hints directory, if it exists.
    fn global_hints_dir(&self) -> Option<PathBuf> {
        let config_dir = home_dir()
            .join(".config/sim")
            .join(GLOBAL_HINTS_DIR_NAME);
        if config_dir.exists() {
            Some(config_dir)
        } else {
            None
        }
    }

    /// Loads hints from the global hints directory.
    pub async fn load_global_hints(&self) -> (Vec<Hint>, Vec<HintLoadError>) {
        let Some(dir) = self.global_hints_dir() else {
            return (Vec::new(), Vec::new());
        };

        if !self.fs.is_dir(&dir).await {
            return (Vec::new(), Vec::new());
        }

        let Ok(mut entries) = self.fs.read_dir(&dir).await else {
            return (Vec::new(), Vec::new());
        };

        let mut hints = Vec::new();
        let mut errors = Vec::new();

        while let Some(Ok(entry)) = entries.next().await {
            if self.fs.is_file(&entry).await && is_hint_file(&entry) {
                match self.fs.load(&entry).await {
                    Ok(content) => {
                        let trimmed = content.trim().to_string();
                        if !trimmed.is_empty() {
                            hints.push(Hint {
                                source: HintSource::Global { path: entry },
                                content: trimmed,
                                priority: 0,
                            });
                        }
                    }
                    Err(err) => {
                        errors.push(HintLoadError {
                            path: entry,
                            message: err.to_string(),
                        });
                    }
                }
            }
        }

        hints.sort_by_key(|hint| std::cmp::Reverse(hint.priority));
        (hints, errors)
    }

    /// Loads hints from `.simhints` files in the given worktree roots.
    pub async fn load_project_hints(
        &self,
        worktree_roots: &[(String, PathBuf)],
    ) -> (Vec<Hint>, Vec<HintLoadError>) {
        let mut hints = Vec::new();
        let mut errors = Vec::new();

        for (root_name, root_path) in worktree_roots {
            for file_name in SIM_HINTS_FILE_NAMES {
                let hint_path = root_path.join(file_name);
                if self.fs.is_file(&hint_path).await {
                    match self.fs.load(&hint_path).await {
                        Ok(content) => {
                            let trimmed = content.trim().to_string();
                            if !trimmed.is_empty() {
                                let resolved = self.resolve_imports(&trimmed, root_path).await;
                                hints.push(Hint {
                                    source: HintSource::Project {
                                        worktree_root: root_name.clone(),
                                        path: hint_path,
                                    },
                                    content: resolved,
                                    priority: 10,
                                });
                            }
                        }
                        Err(err) => {
                            errors.push(HintLoadError {
                                path: hint_path,
                                message: err.to_string(),
                            });
                        }
                    }
                }
            }
        }

        hints.sort_by_key(|hint| std::cmp::Reverse(hint.priority));
        (hints, errors)
    }

    /// Loads all hints (global + project), merging them with project hints
    /// taking priority.
    pub async fn load_all(
        &self,
        worktree_roots: &[(String, PathBuf)],
    ) -> (Vec<Hint>, Vec<HintLoadError>) {
        let mut all_errors = Vec::new();

        let (global_hints, global_errors) = self.load_global_hints().await;
        all_errors.extend(global_errors);

        let (project_hints, project_errors) = self.load_project_hints(worktree_roots).await;
        all_errors.extend(project_errors);

        // Merge — project hints (priority 10) go before global hints (priority 0).
        let merged = project_hints
            .into_iter()
            .chain(global_hints)
            .collect::<Vec<_>>();

        (merged, all_errors)
    }

    /// Resolves `@import` or file-reference directives inside hint content.
    ///
    /// Lines matching `@import "<path>"` or `@import '<path>'` are replaced
    /// with the contents of the referenced file, resolved relative to
    /// `root_path`.
    async fn resolve_imports(&self, content: &str, root_path: &Path) -> String {
        let mut result = String::new();
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(import_path) = trimmed.strip_prefix("@import ").and_then(strip_quotes) {
                let resolved = root_path.join(import_path);
                match self.fs.load(&resolved).await {
                    Ok(imported_content) => {
                        result.push_str(imported_content.trim());
                        result.push('\n');
                    }
                    Err(_) => {
                        result.push_str(&format!("<!-- Unresolved import: {import_path} -->\n"));
                    }
                }
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }
        result.trim().to_string()
    }
}

/// Strips matching quotes from a string. Returns `None` if the string
/// isn't properly quoted.
fn strip_quotes(s: &str) -> Option<&str> {
    let s = s.trim();
    if let Some(inner) = s.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        Some(inner)
    } else if let Some(inner) = s.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
        Some(inner)
    } else {
        None
    }
}

/// Returns true if the file extension indicates it could contain hint text.
fn is_hint_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
        // No extension — could be a dotfile like `.simhints` itself.
        return true;
    };
    matches!(ext, "md" | "txt" | "hbs" | "mdx" | "simhints")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_quotes_double() {
        assert_eq!(strip_quotes("\"hello\""), Some("hello"));
    }

    #[test]
    fn test_strip_quotes_single() {
        assert_eq!(strip_quotes("'hello'"), Some("hello"));
    }

    #[test]
    fn test_strip_quotes_no_match() {
        assert_eq!(strip_quotes("hello"), None);
        assert_eq!(strip_quotes("\"unclosed"), None);
    }

    #[test]
    fn test_is_hint_file_by_extension() {
        assert!(is_hint_file(Path::new("test.md")));
        assert!(is_hint_file(Path::new("test.txt")));
        assert!(is_hint_file(Path::new("test.simhints")));
        assert!(is_hint_file(Path::new(".simhints")));
        assert!(!is_hint_file(Path::new("test.rs")));
        assert!(!is_hint_file(Path::new("test.py")));
    }

    use fs::FakeFs;
    use gpui::TestAppContext;

    fn test_loader(cx: &mut TestAppContext) -> (HintLoader, Arc<FakeFs>) {
        let executor = cx.background_executor.clone();
        let fs = FakeFs::new(executor);
        let loader = HintLoader::new(fs.clone());
        (loader, fs)
    }

    #[gpui::test]
    async fn test_load_global_no_dir(cx: &mut TestAppContext) {
        // When no global hints directory exists, load_global_hints
        // returns empty results.
        let (loader, _fs) = test_loader(cx);
        let (hints, errors) = loader.load_global_hints().await;
        assert!(hints.is_empty());
        assert!(errors.is_empty());
    }

    #[gpui::test]
    async fn test_load_project_hints_empty(cx: &mut TestAppContext) {
        let (loader, _fs) = test_loader(cx);
        let (hints, errors) = loader
            .load_project_hints(&[("test".to_string(), PathBuf::from("/test"))])
            .await;
        assert!(hints.is_empty());
        assert!(errors.is_empty());
    }

    #[gpui::test]
    async fn test_load_project_hints_with_file(cx: &mut TestAppContext) {
        let (loader, fs) = test_loader(cx);
        fs.create_dir(Path::new("/test")).await.unwrap();
        fs.insert_file(
            Path::new("/test/.simhints"),
            b"use rust edition 2021".to_vec(),
        )
        .await;

        let (hints, errors) = loader
            .load_project_hints(&[("root".to_string(), PathBuf::from("/test"))])
            .await;

        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].content, "use rust edition 2021");
        assert_eq!(hints[0].priority, 10);
        match &hints[0].source {
            HintSource::Project {
                worktree_root,
                path: _,
            } => {
                assert_eq!(worktree_root, "root");
            }
            _ => panic!("expected Project source"),
        }
    }

    #[gpui::test]
    async fn test_load_project_hints_multiple_roots(cx: &mut TestAppContext) {
        let (loader, fs) = test_loader(cx);
        fs.create_dir(Path::new("/a")).await.unwrap();
        fs.create_dir(Path::new("/b")).await.unwrap();
        fs.insert_file(Path::new("/a/.simhints"), b"hint from a".to_vec())
            .await;
        fs.insert_file(Path::new("/b/.simhints"), b"hint from b".to_vec())
            .await;

        let (hints, errors) = loader
            .load_project_hints(&[
                ("a".to_string(), PathBuf::from("/a")),
                ("b".to_string(), PathBuf::from("/b")),
            ])
            .await;

        assert!(errors.is_empty());
        assert_eq!(hints.len(), 2);
    }

    #[gpui::test]
    async fn test_load_project_hints_skips_empty_content(cx: &mut TestAppContext) {
        let (loader, fs) = test_loader(cx);
        fs.create_dir(Path::new("/test")).await.unwrap();
        fs.insert_file(Path::new("/test/.simhints"), b"   ".to_vec())
            .await;

        let (hints, errors) = loader
            .load_project_hints(&[("root".to_string(), PathBuf::from("/test"))])
            .await;

        assert!(errors.is_empty());
        assert!(hints.is_empty(), "whitespace-only hints should be skipped");
    }

    #[gpui::test]
    async fn test_resolve_imports_resolves_file(cx: &mut TestAppContext) {
        let (loader, fs) = test_loader(cx);
        fs.create_dir(Path::new("/project")).await.unwrap();
        fs.insert_file(Path::new("/project/shared.rules"), b"shared-rule".to_vec())
            .await;

        let content = "line1\n@import \"shared.rules\"\nline3";
        let result = loader.resolve_imports(content, Path::new("/project")).await;

        assert!(result.contains("line1"), "non-import lines preserved");
        assert!(result.contains("shared-rule"), "imported content present");
        assert!(result.contains("line3"), "lines after import preserved");
    }

    #[gpui::test]
    async fn test_resolve_imports_missing_file_adds_comment(cx: &mut TestAppContext) {
        let (loader, _fs) = test_loader(cx);
        let content = "@import \"nonexistent.md\"";
        let result = loader.resolve_imports(content, Path::new("/project")).await;
        assert!(result.contains("Unresolved import"), "result: {result}");
    }

    #[gpui::test]
    async fn test_resolve_imports_single_quotes(cx: &mut TestAppContext) {
        let (loader, fs) = test_loader(cx);
        fs.create_dir(Path::new("/project")).await.unwrap();
        fs.insert_file(Path::new("/project/inc.md"), b"included content".to_vec())
            .await;

        let content = "@import 'inc.md'";
        let result = loader.resolve_imports(content, Path::new("/project")).await;
        assert!(result.contains("included content"), "result: {result}");
    }

    #[gpui::test]
    async fn test_load_all_merges_hints(cx: &mut TestAppContext) {
        let (loader, fs) = test_loader(cx);
        fs.create_dir(Path::new("/p1")).await.unwrap();
        fs.insert_file(Path::new("/p1/.simhints"), b"project-hint".to_vec())
            .await;

        let (hints, errors) = loader
            .load_all(&[("p1".to_string(), PathBuf::from("/p1"))])
            .await;

        assert!(errors.is_empty());
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0].content, "project-hint");
    }

    #[gpui::test]
    async fn test_load_project_hints_missing_file_no_error(cx: &mut TestAppContext) {
        let (loader, fs) = test_loader(cx);
        fs.create_dir(Path::new("/empty")).await.unwrap();

        let (hints, errors) = loader
            .load_project_hints(&[("empty".to_string(), PathBuf::from("/empty"))])
            .await;

        assert!(hints.is_empty());
        assert!(errors.is_empty(), "no error for missing file");
    }
}
