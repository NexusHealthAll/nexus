use axum::{extract::State, http::StatusCode, Json};
use serde_json::{json, Value};

use crate::routes::AppState;

/// Health check endpoint
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = Value,
            example = json!({"status": "ok", "service": "nexuscare-backend"}))
    )
)]
pub async fn health_check() -> (StatusCode, Json<Value>) {
    (
        StatusCode::OK,
        Json(json!({ "status": "ok", "service": "nexuscare-backend" })),
    )
}

/// Database health check endpoint
#[utoipa::path(
    get,
    path = "/health/db",
    tag = "health",
    responses(
        (status = 200, description = "Database is connected", body = Value,
            example = json!({"status": "ok", "database": "connected"})),
        (status = 503, description = "Database connection failed", body = Value,
            example = json!({"status": "error", "database": "connection failed"}))
    )
)]
pub async fn db_health_check(State(state): State<AppState>) -> (StatusCode, Json<Value>) {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({ "status": "ok", "database": "connected" })),
        ),
        Err(e) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "status": "error", "database": e.to_string() })),
        ),
    }
}
