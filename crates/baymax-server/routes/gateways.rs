use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiResult, AppState, state::GatewayDetail};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateGatewayRequest {
    pub kind: String,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<GatewayDetail>>> {
    super::require_auth(&headers, &state)?;
    let data = state
        .data
        .lock()
        .map_err(|_| ApiError::Internal("server state lock poisoned".into()))?;
    Ok(Json(data.gateways.values().cloned().collect()))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateGatewayRequest>,
) -> ApiResult<Json<GatewayDetail>> {
    super::require_auth(&headers, &state)?;
    if request.kind.trim().is_empty() {
        return Err(ApiError::BadRequest("gateway kind is required".into()));
    }
    let mut data = state
        .data
        .lock()
        .map_err(|_| ApiError::Internal("server state lock poisoned".into()))?;
    let id = data.next_id("gateway");
    let gateway = GatewayDetail {
        id: id.clone(),
        kind: request.kind,
    };
    data.gateways.insert(id, gateway.clone());
    Ok(Json(gateway))
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
    if data.gateways.remove(&id).is_some() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(id))
    }
}
