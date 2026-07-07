pub mod agent;
pub mod config;
pub mod dictation;
pub mod gateways;
pub mod recipes;
pub mod schedules;
pub mod session_events;
pub mod sessions;
pub mod setup;
pub mod system;

use axum::http::HeaderMap;

use crate::{ApiResult, AppState, auth};

fn require_auth(headers: &HeaderMap, state: &AppState) -> ApiResult<()> {
    auth::require_auth(headers, &state.config.auth)
}
