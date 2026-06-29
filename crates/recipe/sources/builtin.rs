use anyhow::{Context as _, Result, anyhow};

use crate::{Recipe, RecipeManifest, RecipeSource, RecipeSourceType, RecipeValidator};

#[derive(Debug, Clone)]
pub struct BuiltinRecipe {
    pub name: &'static str,
    pub content: &'static str,
}

#[derive(Debug, Clone)]
pub struct BuiltinRecipeSource {
    recipes: Vec<BuiltinRecipe>,
    priority: u8,
}

impl BuiltinRecipeSource {
    pub fn new(recipes: impl Into<Vec<BuiltinRecipe>>) -> Self {
        Self {
            recipes: recipes.into(),
            priority: 10,
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn baymax_defaults() -> Self {
        Self::new(vec![BuiltinRecipe {
            name: "release-risk-check",
            content: include_str!("../builtin_recipes/release_risk_check.yaml"),
        }])
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }
}

impl RecipeSource for BuiltinRecipeSource {
    fn discover(&self) -> Result<Vec<RecipeManifest>> {
        let mut manifests = Vec::new();
        for builtin in &self.recipes {
            let recipe = RecipeValidator::validate_yaml(builtin.content)
                .with_context(|| format!("failed to validate built-in recipe {}", builtin.name))?;
            manifests.push(recipe.manifest(RecipeSourceType::Builtin));
        }
        Ok(manifests)
    }

    fn load(&self, name: &str) -> Result<Recipe> {
        for builtin in &self.recipes {
            let recipe = RecipeValidator::validate_yaml(builtin.content)
                .with_context(|| format!("failed to validate built-in recipe {}", builtin.name))?;
            if builtin.name.eq_ignore_ascii_case(name) || recipe.title.eq_ignore_ascii_case(name) {
                return Ok(recipe);
            }
        }

        Err(anyhow!("built-in recipe `{name}` was not found"))
    }

    fn priority(&self) -> u8 {
        self.priority
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASE_RECIPE: &str = r#"
title: Release Risk
description: Check release risk
prompt: Check release
"#;

    #[test]
    fn discovers_builtin_recipes() {
        let source = BuiltinRecipeSource::new(vec![BuiltinRecipe {
            name: "release-risk",
            content: RELEASE_RECIPE,
        }]);

        let manifests = source.discover().unwrap();

        assert_eq!(manifests[0].name, "Release Risk");
    }

    #[test]
    fn loads_by_builtin_name() {
        let source = BuiltinRecipeSource::new(vec![BuiltinRecipe {
            name: "release-risk",
            content: RELEASE_RECIPE,
        }]);

        let recipe = source.load("release-risk").unwrap();

        assert_eq!(recipe.title, "Release Risk");
    }

    #[test]
    fn default_builtins_include_release_risk_check() {
        let source = BuiltinRecipeSource::baymax_defaults();

        let recipe = source.load("release-risk-check").unwrap();

        assert_eq!(recipe.title, "Release Risk Check");
    }
}
