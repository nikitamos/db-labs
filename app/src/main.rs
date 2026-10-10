use std::collections::HashMap;

use app::{AppState, config::AppConfig, features::user::repository::{TestUserRepo, UserRepositoryEndpoints}};
use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use tower_http::trace::TraceLayer;

async fn health(State(_): State<AppState>) -> (StatusCode, &'static str) {
    (StatusCode::OK, "healthy")
}

async fn all_env(State(_): State<AppState>) -> Json<HashMap<String, String>> {
    let env: HashMap<String, String> = std::env::vars().collect();
    Json(env)
}

fn setup_tracing() {
    use tracing_subscriber::fmt;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|e| {
                eprintln!("error reading tracing env: {e}");
                format!(
                    "sqlx=info,tower_http=debug,{}=debug",
                    env!("CARGO_CRATE_NAME")
                )
                .into()
            }),
        )
        .with(fmt::layer().event_format(fmt::format().without_time()))
        .init();
}

#[tokio::main]
async fn main() {
    setup_tracing();

    let config: AppConfig = envy::from_env().unwrap();
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .unwrap();

    let state = AppState::from_config(config).await;
    state.migrate_db().await.unwrap();

    let app = Router::new()
        .route("/health", get(health))
        .route("/all_env", get(all_env))
        .route("/test/{id}", get(UserRepositoryEndpoints::<TestUserRepo>::xxxx))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    axum::serve(listener, app).await.unwrap();
}
