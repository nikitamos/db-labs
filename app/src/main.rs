use std::collections::HashMap;

use app::{
    AppState,
    config::AppConfig,
    features::{
        category::service::SqlxCategoryService,
        user::repository::{TestUserRepo, UserRepositoryEndpoints},
    },
};
use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use tower_http::trace::TraceLayer;
use tracing::instrument::WithSubscriber;

async fn health(State(_): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::OK,
        serde_json::json!({
            "status": "pass"
        })
        .into(),
    )
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
    let _ = state.migrate_db().with_current_subscriber().await;

    let app = Router::new()
        .route(
            "/test/{id}",
            get(UserRepositoryEndpoints::<TestUserRepo>::xxxx),
        )
        .merge(app::features::maintenance::routes())
        .nest(
            "/category",
            app::features::category::routes::<SqlxCategoryService, _>(),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    axum::serve(listener, app).await.unwrap();
}
