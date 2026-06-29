use axum::{
    Json,
    extract::State,
    http::HeaderMap,
    response::sse::{Event, Sse},
};
use futures::stream;
use serde::{Deserialize, Serialize};

use crate::{ApiResult, AppState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRequest {
    pub message: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentResponse {
    pub response: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentStatus {
    pub status: String,
}

pub async fn message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AgentRequest>,
) -> ApiResult<Json<AgentResponse>> {
    super::require_auth(&headers, &state)?;
    Ok(Json(AgentResponse {
        response: format!("Baymax received: {}", request.message),
        session_id: request.session_id,
    }))
}

pub async fn stream(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AgentRequest>,
) -> ApiResult<Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>>> {
    super::require_auth(&headers, &state)?;
    let event = Event::default()
        .event("message")
        .data(format!("Baymax received: {}", request.message));
    Ok(Sse::new(stream::once(async move { Ok(event) })))
}

pub async fn status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<AgentStatus>> {
    super::require_auth(&headers, &state)?;
    Ok(Json(AgentStatus {
        status: "idle".to_string(),
    }))
}
