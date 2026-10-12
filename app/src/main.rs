use app::{
    AppState, config::AppConfig, features::{
        auth::service::{AuthServiceEndpoints, SqlxAuthService}, category::service::{CategoryServiceEndpoints, SqlxCategoryService}, discussion::service::{DiscussionServiceEndpoints, SqlxDiscussionService}, project::service::{ProjectServiceEndpoints, SqlxProjectService}, status::service::{SqlxStatusService, StatusServiceEndpoints}, task::service::{SqlxTaskService, TaskServiceEndpoints}, user::service::{SqlxUserService, UserServiceEndpoints},
    },
};
use axum::{Json, Router, extract::State, http::StatusCode, routing::get};
use tower_http::trace::TraceLayer;
use tracing::instrument::WithSubscriber;

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
        .merge(app::features::maintenance::routes())
        .nest(
            "/category",
            CategoryServiceEndpoints::<SqlxCategoryService>::routes(),
        )
        .nest(
            "/discussion",
            DiscussionServiceEndpoints::<SqlxDiscussionService>::routes(),
        )
        .nest(
            "/project",
            ProjectServiceEndpoints::<SqlxProjectService>::routes(),
        )
        .nest(
            "/status",
            StatusServiceEndpoints::<SqlxStatusService>::routes(),
        )
        .nest(
            "/auth",
            AuthServiceEndpoints::<SqlxAuthService>::routes(),
        )
        .nest(
            "/user",
            UserServiceEndpoints::<SqlxUserService>::routes()
        )
        .nest("/task", TaskServiceEndpoints::<SqlxTaskService>::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    axum::serve(listener, app).await.unwrap();
}
