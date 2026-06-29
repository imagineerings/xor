use axum::{Json, response::Html};

pub async fn openapi_json() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Baymax REST API",
            "version": "0.1.0"
        },
        "paths": {
            "/agent/message": {},
            "/agent/stream": {},
            "/agent/status": {},
            "/sessions": {},
            "/sessions/{id}": {},
            "/sessions/{id}/events": {},
            "/recipes": {},
            "/recipes/{name}": {},
            "/recipes/{name}/run": {},
            "/config": {},
            "/config/{key}": {},
            "/schedules": {},
            "/schedules/{id}": {},
            "/dictation": {},
            "/gateways": {},
            "/gateways/{id}": {},
            "/health": {},
            "/status": {},
            "/telemetry": {},
            "/setup": {}
        }
    }))
}

pub async fn docs() -> Html<&'static str> {
    Html(
        r#"<!doctype html><html><head><title>Baymax REST API</title></head><body><h1>Baymax REST API</h1><p>OpenAPI JSON is available at <a href="/openapi.json">/openapi.json</a>.</p></body></html>"#,
    )
}
