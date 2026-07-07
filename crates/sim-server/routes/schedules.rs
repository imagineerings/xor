use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};

use crate::{ApiError, ApiResult, AppState, state::ScheduleDetail};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateScheduleRequest {
    pub cron: String,
    pub recipe: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<ScheduleDetail>>> {
    super::require_auth(&headers, &state)?;
    let data = state
        .data
        .lock()
        .map_err(|_| ApiError::Internal("server state lock poisoned".into()))?;
    Ok(Json(data.schedules.values().cloned().collect()))
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateScheduleRequest>,
) -> ApiResult<Json<ScheduleDetail>> {
    super::require_auth(&headers, &state)?;
    if request.cron.trim().is_empty() {
        return Err(ApiError::BadRequest("cron expression is required".into()));
    }
    let mut data = state
        .data
        .lock()
        .map_err(|_| ApiError::Internal("server state lock poisoned".into()))?;
    let id = data.next_id("schedule");
    let schedule = ScheduleDetail {
        id: id.clone(),
        cron: request.cron,
        recipe: request.recipe,
    };
    data.schedules.insert(id, schedule.clone());
    Ok(Json(schedule))
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
    if data.schedules.remove(&id).is_some() {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::NotFound(id))
    }
}
