use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use recipe::{ExecutionContext, Recipe, RecipeManifest, RecipeOutput};
use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiResult, AppState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunRecipeRequest {
    #[serde(default)]
    pub variables: HashMap<String, String>,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<RecipeManifest>>> {
    super::require_auth(&headers, &state)?;
    Ok(Json(state.recipes.discover_all()?))
}

pub async fn get_recipe(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
) -> ApiResult<Json<Recipe>> {
    super::require_auth(&headers, &state)?;
    state
        .recipes
        .load(&name)
        .map(Json)
        .map_err(|_| ApiError::NotFound(name))
}

pub async fn run(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(name): Path<String>,
    Json(request): Json<RunRecipeRequest>,
) -> ApiResult<Json<RecipeOutput>> {
    super::require_auth(&headers, &state)?;
    let recipe = state
        .recipes
        .load(&name)
        .map_err(|_| ApiError::NotFound(name.clone()))?;
    let mut context = ExecutionContext {
        variables: request.variables,
        ..Default::default()
    };
    Ok(Json(state.recipes.execute(&recipe, &mut context)?))
}
