use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::yaml_format::{format_recipe_yaml, parse_recipe_yaml};

fn default_version() -> String {
    "1.0.0".to_string()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Recipe {
    #[serde(default = "default_version")]
    pub version: String,
    pub title: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<RecipeParameter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<RecipeStep>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<RecipeSettings>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub activities: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<RecipeAuthor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response: Option<RecipeResponse>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sub_recipes: Vec<SubRecipe>,
    #[serde(default, skip_serializing_if = "serde_json::Map::is_empty")]
    pub metadata: serde_json::Map<String, serde_json::Value>,
}

impl Recipe {
    pub fn from_yaml(content: &str) -> Result<Self> {
        parse_recipe_yaml(content)
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read recipe {}", path.display()))?;
        Self::from_yaml(&content)
    }

    pub fn to_yaml(&self) -> Result<String> {
        format_recipe_yaml(self)
    }

    pub fn manifest(&self, source: RecipeSourceType) -> RecipeManifest {
        RecipeManifest {
            name: self.title.clone(),
            description: self.description.clone(),
            version: self.version.clone(),
            source,
            tags: self.tags.clone(),
            author: self
                .author
                .as_ref()
                .and_then(|author| author.contact.clone().or_else(|| author.metadata.clone())),
            variables: self
                .parameters
                .iter()
                .map(|parameter| parameter.key.clone())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeStep {
    pub id: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(default)]
    pub error_policy: ErrorPolicy,
    #[serde(default)]
    pub wait_for_input: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorPolicy {
    #[default]
    Stop,
    Skip,
    Continue,
    Retry,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeSettings {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goose_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goose_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_turns: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeAuthor {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubRecipe {
    pub name: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub values: HashMap<String, String>,
    #[serde(default)]
    pub sequential_when_repeated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeParameter {
    pub key: String,
    pub input_type: RecipeParameterInputType,
    pub requirement: RecipeParameterRequirement,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeParameterRequirement {
    Required,
    Optional,
    UserPrompt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeParameterInputType {
    String,
    Number,
    Boolean,
    Date,
    File,
    Select,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecipeManifest {
    pub name: String,
    pub description: String,
    pub version: String,
    pub source: RecipeSourceType,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub variables: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum RecipeSourceType {
    Builtin,
    Local {
        path: PathBuf,
    },
    GitHub {
        owner: String,
        repo: String,
        path: String,
    },
    Deeplink {
        uri: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_goose_style_recipe() {
        let recipe = Recipe::from_yaml(
            r#"
version: 1.0.0
title: Release Risk Check
description: Check release risk
instructions: Check {{ target_branch }}
parameters:
  - key: target_branch
    input_type: string
    requirement: required
    description: Branch to inspect
"#,
        )
        .unwrap();

        assert_eq!(recipe.title, "Release Risk Check");
        assert_eq!(recipe.parameters[0].key, "target_branch");
    }

    #[test]
    fn parses_nested_recipe_key() {
        let recipe = Recipe::from_yaml(
            r#"
recipe:
  title: Nested
  description: Nested recipe
  prompt: Run it
"#,
        )
        .unwrap();

        assert_eq!(recipe.title, "Nested");
        assert_eq!(recipe.prompt.as_deref(), Some("Run it"));
    }
}
