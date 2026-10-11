use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;

use crate::{AppState, HttpError};
#[derive(Debug, Default)]
pub enum HealthStatus {
    #[default]
    Healthy,
    DbSetupFailed(String),
}

impl IntoResponse for &HealthStatus {
    fn into_response(self) -> Response {
        match self {
            HealthStatus::Healthy => (StatusCode::OK, Json(json!({"status":"pass"}))).into_response(),
            HealthStatus::DbSetupFailed(error) => {
                let message = Json(json!({
                    "status": "error",
                    "output": format!("failed to initialize db: {error}")
                }));
                (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
            }
        }
    }
}

pub async fn health_check(State(app): State<AppState>) -> Response {
    app.health.lock().into_response()
}

pub async fn reset_db(State(app): State<AppState>) -> Result<(), HttpError> {
    app.reset_db().await.map_err(crate::Error::from)?;
    Ok(())
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_check))
        .route("/db_reset", post(reset_db))
}

// pub fn routes()
