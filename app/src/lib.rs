use parking_lot::Mutex;
use std::{fmt::format, sync::Arc};

use axum::{
    Json,
    extract::FromRef,
    http::{Response, StatusCode},
    response::IntoResponse,
};
use serde::Serialize;
use sqlx::{
    Pool, Postgres,
    migrate::{MigrateError, Migrator},
    postgres::{PgConnectOptions, PgDatabaseError, PgQueryResult},
};
use tracing::error;

use crate::{
    config::AppConfig,
    features::{maintenance::HealthStatus, user::repository::TestUserRepo},
};

pub mod config;
pub mod entities;
pub mod features {
    pub mod category;
    pub mod discussion;
    pub mod project;
    pub mod status;
    pub mod task;
    pub mod user;
    pub mod maintenance;
}

#[derive(Clone, FromRef)]
pub struct AppState {
    pub db: Pool<Postgres>,
    pub user_repo: TestUserRepo,
    pub health: Arc<Mutex<HealthStatus>>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sql query failed: {0}")]
    Sqlx(#[from] sqlx::Error),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

static MIGRATOR: Migrator = sqlx::migrate!("./sql/migrations");
impl AppState {
    pub async fn from_config(cfg: AppConfig) -> Self {
        let pool = Pool::<Postgres>::connect(&cfg.app_db_uri).await.unwrap();
        Self {
            db: pool,
            user_repo: TestUserRepo,
            health: Arc::new(Default::default()),
        }
    }

    pub async fn migrate_db(&self) -> Result<(), MigrateError> {
        MIGRATOR.run(&self.db).await.map_err(|e| {
            error!("Failed to apply migrations: {e}");
           * self.health.lock() = HealthStatus::DbSetupFailed(format!("{e}"));
            e
        })
    }
    pub async fn drop_db_content(&self) -> Result<PgQueryResult, sqlx::Error> {
        sqlx::raw_sql(
            r#"DROP SCHEMA public CASCADE;
           CREATE SCHEMA public;
        "#,
        )
        .execute(&self.db)
        .await
        .map_err(|e| {
            *self.health.lock() = HealthStatus::DbSetupFailed(format!("{e}"));
            e
        })
    }
    pub async fn reset_db(&self) -> sqlx::Result<()> {
        self.drop_db_content().await?;
        self.migrate_db().await?;
        *self.health.lock() = HealthStatus::Healthy;
        Ok(())
    }
}

#[derive(Serialize, Debug)]
pub struct HttpError {
    #[serde(serialize_with = "serialize_status")]
    pub code: StatusCode,
    pub details: String,
}

impl HttpError {
    pub fn new(code: StatusCode, details: impl ToString) -> Self {
        Self {
            code,
            details: details.to_string(),
        }
    }
}

impl IntoResponse for HttpError {
    fn into_response(self) -> axum::response::Response {
        (self.code, Json(self)).into_response()
    }
}

impl From<Error> for HttpError {
    fn from(value: Error) -> Self {
        match value {
            Error::Sqlx(e) => Self::from_sqlx_error(e),
        }
    }
}

impl HttpError {
    pub fn from_sqlx_error(e: sqlx::Error) -> Self {
        tracing::error!(db_error = %e, "db error occurred while processing");
        match e {
            sqlx::Error::Database(database_error) => Self::new(
                StatusCode::BAD_REQUEST,
                format!(
                    "data consistency error in `{}`",
                    database_error.table().unwrap_or("unknown")
                ),
            ),
            sqlx::Error::RowNotFound => Self::new(
                StatusCode::NOT_FOUND,
                "at least one of entities implied by the request doesn't exist",
            ),
            sqlx::Error::PoolTimedOut => {
                Self::new(StatusCode::SERVICE_UNAVAILABLE, "database is too busy")
            }
            _ => Self::new(StatusCode::INTERNAL_SERVER_ERROR, "database error"),
        }
    }
}

fn serialize_status<S>(status: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u16(status.as_u16())
}
