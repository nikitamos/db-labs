
use aide::axum::ApiRouter;
use aide::openapi::{Info, OpenApi};
use aide::swagger::Swagger;
use app::{
    AppState, config::AppConfig, features::{
        auth::service::{AuthServiceEndpoints, SqlxAuthService}, category::service::{CategoryServiceEndpoints, SqlxCategoryService}, discussion::service::{DiscussionServiceEndpoints, SqlxDiscussionService}, project::service::{ProjectServiceEndpoints, SqlxProjectService}, status::service::{SqlxStatusService, StatusServiceEndpoints}, task::service::{SqlxTaskService, TaskServiceEndpoints}, user::service::{SqlxUserService, UserServiceEndpoints},
    },
};
use axum::routing::get;
use axum::{Extension, Json};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithTonicConfig;
use tonic::transport::ClientTlsConfig;
use tower_http::trace::TraceLayer;
use tracing::instrument::WithSubscriber;

use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::{
    logs::SdkLoggerProvider,
    trace::SdkTracerProvider,
};
use tracing_opentelemetry::OpenTelemetryLayer;

fn setup_tracing() {
    use tracing_subscriber::fmt;
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

    let provider = setup_otel_tracing();
    let log_provider = setup_otel_logging();
    let otel_trace = OpenTelemetryLayer::new(provider.tracer("ceaser"));

    let otel_log = OpenTelemetryTracingBridge::new(&log_provider);
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
        .with(otel_trace)
        .with(otel_log)
        .init();
}

fn setup_otel_tracing() -> SdkTracerProvider {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_tls_config(ClientTlsConfig::new().with_native_roots())
        .build()
        .unwrap();
    SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .build()
}

fn setup_otel_logging() -> SdkLoggerProvider {
    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .with_tls_config(ClientTlsConfig::new().with_enabled_roots())
        .build()
        .unwrap();
    SdkLoggerProvider::builder()
        .with_batch_exporter(exporter)
        .build()
}

async fn serve_api(Extension(api): Extension<OpenApi>) -> Json<OpenApi> {
    Json(api)
}

#[tokio::main]
async fn main() {
    let config: AppConfig = envy::from_env().unwrap();
    setup_tracing();

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port))
        .await
        .unwrap();

    let state = AppState::from_config(config).await;
    let _ = state.migrate_db().with_current_subscriber().await;

    let mut api = OpenApi {
        info: Info {
            title: "DMaDBMS API".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: Some("Project management API".to_string()),
            ..Default::default()
        },
        ..Default::default()
    };

    let app = ApiRouter::new()
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
        .route("/api.json", get(serve_api))
        .route("/docs", Swagger::new("/api.json").axum_route())
        .layer(TraceLayer::new_for_http())
        .finish_api(&mut api)
        .layer(Extension(api))
        .with_state(state);

    axum::serve(listener, app).await.unwrap();
}
