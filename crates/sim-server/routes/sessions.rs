use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiResult, AppState, state::SessionDetail};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<SessionDetail>>> {
    super::require_auth(&headers, &state)?;
    let data = state
        .data
        .lock()
        .map_err(|_| ApiError::Internal("server state lock poisoned".into()))?;
    Ok(Json(data.sessions.values().cloned().collect()))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateSessionRequest>,
) -> ApiResult<Json<SessionDetail>> {
    super::require_auth(&headers, &state)?;
    let mut data = state
        .data
        .lock()
        .map_err(|_| ApiError::Internal("server state lock poisoned".into()))?;
    let id = data.next_id("session");
    let session = SessionDetail {
        id: id.clone(),
        title: request
            .title
            .unwrap_or_else(|| "Untitled session".to_string()),
    };
    data.sessions.insert(id, session.clone());
    Ok(Json(session))
}

pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<SessionDetail>> {
    super::require_auth(&headers, &state)?;
    let data = state
        .data
        .lock()
        .map_err(|_| ApiError::Internal("server state lock poisoned".into()))?;
    data.sessions
        .get(&id)
        .cloned()
        .map(Json)
        .ok_or(ApiError::NotFound(id))
}

pub async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<StatusCode> {
    super::require_auth(&headers, &state)?;
    let mut data = state
        .data
        .lock()
        .map_err(|_| ApiError::Internal("server state lock poisoned".into()))?;
    if data.sessions.remove(&id).is_some() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(id))
    }
}
