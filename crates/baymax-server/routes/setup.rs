use axum::{Json, extract::State, http::HeaderMap};
use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiResult, AppState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetupStatus {
    pub required: bool,
}

pub async fn get_setup(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<SetupStatus>> {
    super::require_auth(&headers, &state)?;
    let data = state
        .data
        .lock()
        .map_err(|_| ApiError::Internal("server state lock poisoned".into()))?;
    Ok(Json(SetupStatus {
        required: !data.setup_complete,
    }))
}

pub async fn post_setup(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(config): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    super::require_auth(&headers, &state)?;
    let mut data = state
        .data
        .lock()
        .map_err(|_| ApiError::Internal("server state lock poisoned".into()))?;
    if data.setup_complete {
        return Err(ApiError::Forbidden);
    }
    data.config = config.clone();
    data.setup_complete = true;
    Ok(Json(config))
}
