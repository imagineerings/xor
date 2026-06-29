use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, anyhow};

use crate::{Recipe, RecipeManifest, RecipeSource, RecipeSourceType, RecipeValidator};

const RECIPE_FILE_EXTENSIONS: &[&str] = &["yaml", "yml", "json"];

#[derive(Debug, Clone)]
pub struct LocalRecipeSource {
    directory: PathBuf,
    priority: u8,
}

impl LocalRecipeSource {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            priority: 100,
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    fn recipe_files(&self) -> Result<Vec<PathBuf>> {
        let Ok(entries) = fs::read_dir(&self.directory) else {
            return Ok(Vec::new());
        };

        let mut recipe_files = Vec::new();
        for entry in entries {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if !file_type.is_file() {
                continue;
            }

            let path = entry.path();
            let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else {
                continue;
            };
            if RECIPE_FILE_EXTENSIONS
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(extension))
            {
                recipe_files.push(path);
            }
        }
        recipe_files.sort();
        Ok(recipe_files)
    }

    fn load_path(&self, path: &Path) -> Result<Recipe> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("failed to read recipe {}", path.display()))?;
        RecipeValidator::validate_yaml(&content)
            .with_context(|| format!("failed to validate recipe {}", path.display()))
    }
}

impl RecipeSource for LocalRecipeSource {
    fn discover(&self) -> Result<Vec<RecipeManifest>> {
        let mut manifests = Vec::new();
        for path in self.recipe_files()? {
            let recipe = self.load_path(&path)?;
            manifests.push(recipe.manifest(RecipeSourceType::Local { path }));
        }
        Ok(manifests)
    }

    fn load(&self, name: &str) -> Result<Recipe> {
        for path in self.recipe_files()? {
            let recipe = self.load_path(&path)?;
            let file_stem_matches = path
                .file_stem()
                .and_then(|file_stem| file_stem.to_str())
                .is_some_and(|file_stem| file_stem.eq_ignore_ascii_case(name));
            if recipe.title.eq_ignore_ascii_case(name) || file_stem_matches {
                return Ok(recipe);
            }
        }

        Err(anyhow!(
            "recipe `{name}` was not found in {}",
            self.directory.display()
        ))
    }

    fn priority(&self) -> u8 {
        self.priority
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn discovers_yaml_recipes() {
        let temp_dir = tempdir().unwrap();
        fs::write(
            temp_dir.path().join("release.yaml"),
            r#"
title: Release
description: Release recipe
prompt: Check release
"#,
        )
        .unwrap();
        fs::write(temp_dir.path().join("notes.txt"), "ignore").unwrap();

        let source = LocalRecipeSource::new(temp_dir.path());
        let manifests = source.discover().unwrap();

        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].name, "Release");
    }

    #[test]
    fn loads_by_title_or_file_stem() {
        let temp_dir = tempdir().unwrap();
        fs::write(
            temp_dir.path().join("release-risk.yaml"),
            r#"
title: Release Risk
description: Release recipe
prompt: Check release
"#,
        )
        .unwrap();

        let source = LocalRecipeSource::new(temp_dir.path());

        assert_eq!(source.load("Release Risk").unwrap().title, "Release Risk");
        assert_eq!(source.load("release-risk").unwrap().title, "Release Risk");
    }
}
