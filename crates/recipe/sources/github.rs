use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context as _, Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::{Recipe, RecipeManifest, RecipeSource, RecipeSourceType, RecipeValidator};

pub trait GitHubRecipeClient: Clone + Send + Sync + 'static {
    fn fetch_recipe(&self, request: &GitHubRecipeRequest) -> Result<String>;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GitHubRecipeRequest {
    pub owner: String,
    pub repo: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

impl GitHubRecipeRequest {
    pub fn new(owner: impl Into<String>, repo: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            owner: owner.into(),
            repo: repo.into(),
            path: path.into(),
            reference: None,
        }
    }

    pub fn with_reference(mut self, reference: impl Into<String>) -> Self {
        self.reference = Some(reference.into());
        self
    }

    pub fn raw_url(&self) -> String {
        let reference = self.reference.as_deref().unwrap_or("HEAD");
        format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            self.owner,
            self.repo,
            reference,
            self.path.trim_start_matches('/')
        )
    }

    fn source_type(&self) -> RecipeSourceType {
        RecipeSourceType::GitHub {
            owner: self.owner.clone(),
            repo: self.repo.clone(),
            path: self.path.clone(),
        }
    }

    fn file_stem_matches(&self, name: &str) -> bool {
        Path::new(&self.path)
            .file_stem()
            .and_then(|file_stem| file_stem.to_str())
            .is_some_and(|file_stem| file_stem.eq_ignore_ascii_case(name))
    }

    fn path_matches(&self, name: &str) -> bool {
        self.path.eq_ignore_ascii_case(name)
            || format!("{}/{}", self.repo, self.path).eq_ignore_ascii_case(name)
            || format!("{}/{}/{}", self.owner, self.repo, self.path).eq_ignore_ascii_case(name)
    }
}

#[derive(Clone)]
pub struct GitHubRecipeSource<C>
where
    C: GitHubRecipeClient,
{
    client: C,
    recipes: Vec<GitHubRecipeRequest>,
    cache: Arc<Mutex<HashMap<GitHubRecipeRequest, Recipe>>>,
    priority: u8,
}

impl<C> GitHubRecipeSource<C>
where
    C: GitHubRecipeClient,
{
    pub fn new(client: C, recipes: impl Into<Vec<GitHubRecipeRequest>>) -> Self {
        Self {
            client,
            recipes: recipes.into(),
            cache: Arc::new(Mutex::new(HashMap::new())),
            priority: 50,
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn requests(&self) -> &[GitHubRecipeRequest] {
        &self.recipes
    }

    fn load_request(&self, request: &GitHubRecipeRequest) -> Result<Recipe> {
        if let Some(recipe) = self
            .cache
            .lock()
            .map_err(|_| anyhow!("github recipe cache lock is poisoned"))?
            .get(request)
            .cloned()
        {
            return Ok(recipe);
        }

        let content = self
            .client
            .fetch_recipe(request)
            .with_context(|| format!("failed to fetch GitHub recipe {}", request.raw_url()))?;
        let recipe = RecipeValidator::validate_yaml(&content)
            .with_context(|| format!("failed to validate GitHub recipe {}", request.raw_url()))?;

        self.cache
            .lock()
            .map_err(|_| anyhow!("github recipe cache lock is poisoned"))?
            .insert(request.clone(), recipe.clone());
        Ok(recipe)
    }
}

impl<C> RecipeSource for GitHubRecipeSource<C>
where
    C: GitHubRecipeClient,
{
    fn discover(&self) -> Result<Vec<RecipeManifest>> {
        let mut manifests = Vec::new();
        for request in &self.recipes {
            let recipe = self.load_request(request)?;
            manifests.push(recipe.manifest(request.source_type()));
        }
        Ok(manifests)
    }

    fn load(&self, name: &str) -> Result<Recipe> {
        for request in &self.recipes {
            let recipe = self.load_request(request)?;
            if recipe.title.eq_ignore_ascii_case(name)
                || request.file_stem_matches(name)
                || request.path_matches(name)
            {
                return Ok(recipe);
            }
        }

        Err(anyhow!("GitHub recipe `{name}` was not found"))
    }

    fn priority(&self) -> u8 {
        self.priority
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    #[derive(Clone)]
    struct TestGitHubClient {
        fetch_count: Arc<AtomicUsize>,
    }

    impl GitHubRecipeClient for TestGitHubClient {
        fn fetch_recipe(&self, _request: &GitHubRecipeRequest) -> Result<String> {
            self.fetch_count.fetch_add(1, Ordering::SeqCst);
            Ok(r#"
title: Remote Recipe
description: Remote recipe
prompt: Run remote recipe
"#
            .to_string())
        }
    }

    #[test]
    fn builds_raw_github_url() {
        let request = GitHubRecipeRequest::new("simtropolis", "sim", "recipes/release.yaml")
            .with_reference("main");

        assert_eq!(
            request.raw_url(),
            "https://raw.githubusercontent.com/simtropolis/sim/main/recipes/release.yaml"
        );
    }

    #[test]
    fn discovers_and_loads_remote_recipes() {
        let source = GitHubRecipeSource::new(
            TestGitHubClient {
                fetch_count: Arc::new(AtomicUsize::new(0)),
            },
            vec![GitHubRecipeRequest::new(
                "simtropolis",
                "sim",
                "recipes/remote.yaml",
            )],
        );

        let manifests = source.discover().unwrap();
        let recipe = source.load("Remote Recipe").unwrap();

        assert_eq!(manifests[0].name, "Remote Recipe");
        assert_eq!(recipe.title, "Remote Recipe");
    }

    #[test]
    fn caches_fetched_recipe_content() {
        let fetch_count = Arc::new(AtomicUsize::new(0));
        let source = GitHubRecipeSource::new(
            TestGitHubClient {
                fetch_count: fetch_count.clone(),
            },
            vec![GitHubRecipeRequest::new(
                "simtropolis",
                "sim",
                "recipes/remote.yaml",
            )],
        );

        source.discover().unwrap();
        source.load("remote").unwrap();

        assert_eq!(fetch_count.load(Ordering::SeqCst), 1);
    }
}
