use axum::{Json, extract::State, http::HeaderMap};
use serde::{Deserialize, Serialize};

use crate::{ApiResult, AppState, ServerStatus};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
}

pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

pub async fn status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<ServerStatus>> {
    super::require_auth(&headers, &state)?;
    let data = state
        .data
        .lock()
        .map_err(|_| crate::ApiError::Internal("server state lock poisoned".into()))?;
    Ok(Json(ServerStatus {
        status: "ready".to_string(),
        active_sessions: data.sessions.len(),
    }))
}

pub async fn telemetry(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<serde_json::Value>> {
    super::require_auth(&headers, &state)?;
    let data = state
        .data
        .lock()
        .map_err(|_| crate::ApiError::Internal("server state lock poisoned".into()))?;
    Ok(Json(serde_json::json!({
        "sessions": data.sessions.len(),
        "schedules": data.schedules.len(),
        "gateways": data.gateways.len(),
    })))
}
