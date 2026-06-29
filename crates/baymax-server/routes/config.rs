use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};

use crate::{ApiError, ApiResult, AppState};

pub async fn get_config(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    super::require_auth(&headers, &state)?;
    let data = state
        .data
        .lock()
        .map_err(|_| ApiError::Internal("server state lock poisoned".into()))?;
    Ok(Json(data.config.clone()))
}

pub async fn put_config(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(config): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    super::require_auth(&headers, &state)?;
    let mut data = state
        .data
        .lock()
        .map_err(|_| ApiError::Internal("server state lock poisoned".into()))?;
    data.config = config.clone();
    Ok(Json(config))
}

pub async fn get_key(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(key): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    super::require_auth(&headers, &state)?;
    let data = state
        .data
        .lock()
        .map_err(|_| ApiError::Internal("server state lock poisoned".into()))?;
    data.config
        .get(&key)
        .cloned()
        .map(Json)
        .ok_or(ApiError::NotFound(key))
}
