use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};

use crate::{ApiResult, AppState, SessionEvent};

pub async fn stream(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> ApiResult<Json<Vec<SessionEvent>>> {
    super::require_auth(&headers, &state)?;
    Ok(Json(state.events.events_for(&id)))
}
