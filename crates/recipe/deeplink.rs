use std::collections::HashMap;

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use thiserror::Error;
use url::Url;

use crate::{Recipe, RecipeSourceType};

const RECIPE_LINK_PREFIX: &str = "baymax://recipe";
const DATA_QUERY_KEY: &str = "data";

#[derive(Debug, Error)]
pub enum RecipeDeeplinkError {
    #[error("failed to serialize recipe for deeplink")]
    Serialize(#[from] serde_json::Error),
    #[error("recipe deeplink is missing recipe data")]
    MissingData,
    #[error("recipe deeplink URL is invalid")]
    InvalidUrl(#[from] url::ParseError),
    #[error("recipe deeplink data is invalid")]
    InvalidData,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecipeDeeplink {
    pub recipe: Recipe,
    pub variables: HashMap<String, String>,
}

impl RecipeDeeplink {
    pub fn encode(
        recipe: &Recipe,
        variables: &HashMap<String, String>,
    ) -> Result<String, RecipeDeeplinkError> {
        let encoded_recipe = encode_recipe(recipe)?;
        let mut url = Url::parse(RECIPE_LINK_PREFIX)?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs.append_pair(DATA_QUERY_KEY, &encoded_recipe);
            for (key, value) in variables {
                pairs.append_pair(key, value);
            }
        }
        Ok(url.to_string())
    }

    pub fn parse(link: &str) -> Result<Self, RecipeDeeplinkError> {
        let url = Url::parse(link)?;
        if url.scheme() != "baymax" || url.host_str() != Some("recipe") {
            return Err(RecipeDeeplinkError::MissingData);
        }

        let mut encoded_recipe = None;
        let mut variables = HashMap::new();
        for (key, value) in url.query_pairs() {
            if key == DATA_QUERY_KEY {
                encoded_recipe = Some(value.into_owned());
            } else {
                variables.insert(key.into_owned(), value.into_owned());
            }
        }

        let Some(encoded_recipe) = encoded_recipe else {
            return Err(RecipeDeeplinkError::MissingData);
        };
        let recipe = decode_recipe(&encoded_recipe)?;
        Ok(Self { recipe, variables })
    }

    pub fn source_type(link: impl Into<String>) -> RecipeSourceType {
        RecipeSourceType::Deeplink { uri: link.into() }
    }
}

fn encode_recipe(recipe: &Recipe) -> Result<String, RecipeDeeplinkError> {
    let recipe_json = serde_json::to_string(recipe)?;
    Ok(URL_SAFE_NO_PAD.encode(recipe_json.as_bytes()))
}

fn decode_recipe(encoded_recipe: &str) -> Result<Recipe, RecipeDeeplinkError> {
    let decoded_bytes = URL_SAFE_NO_PAD
        .decode(encoded_recipe)
        .map_err(|_| RecipeDeeplinkError::InvalidData)?;
    let recipe_json =
        String::from_utf8(decoded_bytes).map_err(|_| RecipeDeeplinkError::InvalidData)?;
    serde_json::from_str(&recipe_json).map_err(|_| RecipeDeeplinkError::InvalidData)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_recipe() -> Recipe {
        Recipe::from_yaml(
            r#"
title: Shared Recipe
description: Recipe from a deeplink
prompt: Hello {{ name }}
parameters:
  - key: name
    input_type: string
    requirement: required
    description: Name
"#,
        )
        .unwrap()
    }

    #[test]
    fn round_trips_recipe_deeplink() {
        let recipe = test_recipe();
        let variables = HashMap::from([("name".to_string(), "Baymax".to_string())]);
        let link = RecipeDeeplink::encode(&recipe, &variables).unwrap();

        let parsed = RecipeDeeplink::parse(&link).unwrap();

        assert_eq!(parsed.recipe.title, "Shared Recipe");
        assert_eq!(parsed.variables.get("name"), Some(&"Baymax".to_string()));
    }

    #[test]
    fn rejects_missing_data() {
        let error = RecipeDeeplink::parse("baymax://recipe").unwrap_err();

        assert!(matches!(error, RecipeDeeplinkError::MissingData));
    }
}
