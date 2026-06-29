use axum::{Json, extract::State, http::HeaderMap};
use serde::{Deserialize, Serialize};

use crate::{ApiResult, AppState};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictationRequest {
    pub audio_base64: String,
    pub format: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DictationResponse {
    pub text: String,
}

pub async fn transcribe(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(_request): Json<DictationRequest>,
) -> ApiResult<Json<DictationResponse>> {
    super::require_auth(&headers, &state)?;
    Ok(Json(DictationResponse {
        text: String::new(),
    }))
}
