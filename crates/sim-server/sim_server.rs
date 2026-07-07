pub mod auth;
pub mod configuration;
pub mod error;
pub mod event_bus;
pub mod openapi;
pub mod routes;
pub mod state;
pub mod tunnel;

use std::net::SocketAddr;

use anyhow::Result;
use axum::{Router, routing::get};
use tower_http::cors::CorsLayer;

pub use configuration::{AuthConfig, CorsConfig, ServerConfig, TlsConfig};
pub use error::{ApiError, ApiResult, ErrorResponse};
pub use event_bus::{SessionEvent, SessionEventBus};
pub use state::{AppState, ServerStatus};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(routes::system::health))
        .route("/status", get(routes::system::status))
        .route("/telemetry", get(routes::system::telemetry))
        .route(
            "/setup",
            get(routes::setup::get_setup).post(routes::setup::post_setup),
        )
        .route("/agent/status", get(routes::agent::status))
        .route(
            "/agent/message",
            axum::routing::post(routes::agent::message),
        )
        .route("/agent/stream", axum::routing::post(routes::agent::stream))
        .route(
            "/sessions",
            get(routes::sessions::list).post(routes::sessions::create),
        )
        .route(
            "/sessions/:id",
            get(routes::sessions::get).delete(routes::sessions::delete),
        )
        .route("/sessions/:id/events", get(routes::session_events::stream))
        .route("/recipes", get(routes::recipes::list))
        .route("/recipes/:name", get(routes::recipes::get_recipe))
        .route(
            "/recipes/:name/run",
            axum::routing::post(routes::recipes::run),
        )
        .route(
            "/config",
            get(routes::config::get_config).put(routes::config::put_config),
        )
        .route("/config/:key", get(routes::config::get_key))
        .route(
            "/schedules",
            get(routes::schedules::list).post(routes::schedules::create),
        )
        .route(
            "/schedules/:id",
            axum::routing::delete(routes::schedules::delete),
        )
        .route(
            "/dictation",
            axum::routing::post(routes::dictation::transcribe),
        )
        .route(
            "/gateways",
            get(routes::gateways::list).post(routes::gateways::create),
        )
        .route(
            "/gateways/:id",
            axum::routing::delete(routes::gateways::delete),
        )
        .route("/openapi.json", get(openapi::openapi_json))
        .route("/docs", get(openapi::docs))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

pub async fn serve(config: ServerConfig, state: AppState) -> Result<()> {
    let address = SocketAddr::new(config.host.parse()?, config.port);
    axum::Server::bind(&address)
        .serve(router(state).into_make_service())
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode, header},
    };
    use recipe::{BuiltinRecipeSource, RecipeEngine};
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn health_route_returns_ok() {
        let response = router(AppState::for_tests())
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn protected_route_requires_auth() {
        let state = AppState::new(
            ServerConfig {
                auth: AuthConfig::ApiKey {
                    key: "secret".to_string(),
                },
                ..Default::default()
            },
            RecipeEngine::new().with_source(BuiltinRecipeSource::sim_defaults()),
        );

        let response = router(state)
            .oneshot(
                Request::builder()
                    .uri("/agent/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn session_route_creates_session() {
        let response = router(AppState::for_tests())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/sessions")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"title":"Test session"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn recipe_run_route_executes_builtin_recipe() {
        let response = router(AppState::for_tests())
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/recipes/release-risk-check/run")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"variables":{"version":"1.0.0"}}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn openapi_route_is_available() {
        let response = router(AppState::for_tests())
            .oneshot(
                Request::builder()
                    .uri("/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
