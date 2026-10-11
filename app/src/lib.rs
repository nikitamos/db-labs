use axum::{
    Json, extract::FromRef, http::{Response, StatusCode}, response::IntoResponse,
};
use serde::Serialize;
use sqlx::{
    Pool, Postgres,
    migrate::{MigrateError, Migrator},
    postgres::PgQueryResult,
};
use tracing::error;

use crate::{config::AppConfig, features::user::repository::TestUserRepo};

pub mod config;
pub mod entities;
pub mod features {
    pub mod user;
    pub mod project;
}

#[derive(Clone, FromRef)]
pub struct AppState {
    pub db: Pool<Postgres>,
    pub user_repo: TestUserRepo
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
        Self { db: pool, user_repo: TestUserRepo }
    }

    pub async fn migrate_db(&self) -> Result<(), MigrateError> {
        MIGRATOR
            .run(&self.db)
            .await
            .inspect_err(|e| error!("Failed to apply migrations: {e}"))
    }
    pub async fn drop_db_content(&self) -> crate::Result<PgQueryResult> {
        Ok(sqlx::raw_sql(
            r#"DROP SCHEMA public CASCADE;
           CREATE SCHEMA public;
        "#,
        )
        .execute(&self.db)
        .await?)
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
            Error::Sqlx(_) => Self::new(StatusCode::INTERNAL_SERVER_ERROR, "database error"),
        }
    }
}

fn serialize_status<S>(status: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u16(status.as_u16())
}
