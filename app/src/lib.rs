use parking_lot::Mutex;
#[cfg(debug_assertions)]
use std::path::Path;
use std::{fmt::Display, str::FromStr, sync::Arc};

use axum::{Json, extract::FromRef, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use sqlx::{
    Pool, Postgres,
    migrate::{MigrateError, Migrator},
    postgres::{PgConnectOptions, PgPoolOptions, PgQueryResult},
};
use tracing::{error, warn};

use crate::{
    config::AppConfig,
    features::{
        auth::service::SqlxAuthService, category::service::SqlxCategoryService,
        discussion::service::SqlxDiscussionService, maintenance::HealthStatus,
        project::service::SqlxProjectService, status::service::SqlxStatusService,
        task::service::SqlxTaskService, user::service::SqlxUserService,
    },
};

pub mod config;
pub mod entities;
pub mod features {
    pub mod auth;
    pub mod category;
    pub mod discussion;
    pub mod maintenance;
    pub mod project;
    pub mod status;
    pub mod task;
    pub mod user;
}

#[derive(Clone, FromRef)]
pub struct AppState {
    pub db: Pool<Postgres>,
    pub category: SqlxCategoryService,
    pub discussion: SqlxDiscussionService,
    pub project: SqlxProjectService,
    pub status: SqlxStatusService,
    pub task: SqlxTaskService,
    pub user: SqlxUserService,
    pub auth: SqlxAuthService,
    pub health: Arc<Mutex<HealthStatus>>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sql query failed: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("bcrypt operation failed: {0}")]
    Bcrypt(#[from] bcrypt::BcryptError),
    #[error("passwords do not match")]
    WrongPassword,
    #[error("Requested item does not exist")]
    NotFound,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;

impl AppState {
    pub async fn from_config(cfg: AppConfig) -> Self {
        let pool = PgPoolOptions::new()
            .max_connections(24)
            .before_acquire(|_, m| {
                Box::pin(async move {
                    tracing::debug!(age_ms = m.age.as_millis(), "acquire connection");
                    Ok(true)
                })
            })
            .connect_with(
                PgConnectOptions::from_str(&cfg.app_db_uri)
                    .unwrap()
                    .statement_cache_capacity(0),
            )
            .await
            .unwrap();
        let user = SqlxUserService::new(pool.clone());
        Self {
            category: SqlxCategoryService::new(pool.clone()),
            discussion: SqlxDiscussionService::new(pool.clone()),
            project: SqlxProjectService::new(pool.clone()),
            status: SqlxStatusService::new(pool.clone()),
            task: SqlxTaskService::new(pool.clone()),
            auth: SqlxAuthService::new(pool.clone(), user.clone()),
            user,
            health: Arc::new(Default::default()),
            db: pool,
        }
    }

    pub async fn migrate_db(&self) -> Result<(), MigrateError> {
        tracing::info!("Applying migrations");
        #[cfg(not(debug_assertions))]
        let mut migrator = sqlx::migrate!("./sql/migrations");
        #[cfg(debug_assertions)]
        let mut migrator = Migrator::new(Path::new("./sql/migrations")).await?;
        migrator
            .set_locking(false)
            .run(&self.db)
            .await
            .map_err(|e| {
                error!("Failed to apply migrations: {e}");
                *self.health.lock() = HealthStatus::DbSetupFailed(format!("{e}"));
                e
            })
            .inspect(|_| tracing::info!("Migration applied"))
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
        .inspect(|_| tracing::debug!("DB dropped"))
    }
    pub async fn reset_db(&self) -> sqlx::Result<()> {
        self.drop_db_content().await?;
        self.migrate_db().await?;
        *self.health.lock() = HealthStatus::Healthy;
        Ok(())
    }
    pub async fn truncate_db(&self) -> sqlx::Result<()> {
        sqlx::raw_sql(r#"
            DO $$ 
            DECLARE 
                r RECORD;
            BEGIN
                FOR r IN (
                    SELECT tablename 
                    FROM pg_tables 
                    WHERE schemaname = 'public' 
                    AND tablename != '_sqlx_migrations'
                ) LOOP
                    EXECUTE 'TRUNCATE TABLE public.' || quote_ident(r.tablename) || ' RESTART IDENTITY CASCADE;';
                END LOOP;
            END $$;
            "#
        ).execute(&self.db).await.map(|_| ())
    }
}

#[derive(Serialize, Debug, schemars::JsonSchema)]
pub struct HttpError {
    #[serde(serialize_with = "serialize_status")]
    #[schemars(with = "u16")]
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

impl aide::OperationOutput for HttpError {
    type Inner = Self;

    fn operation_response(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) -> Option<aide::openapi::Response> {
        let mut response =
            <Json<Self> as aide::OperationOutput>::operation_response(ctx, operation)?;
        response.description = "Error response".to_string();
        Some(response)
    }

    fn inferred_responses(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) -> Vec<(Option<u16>, aide::openapi::Response)> {
        Self::operation_response(ctx, operation)
            .map(|response| vec![(None, response)])
            .unwrap_or_default()
    }
}

impl From<Error> for HttpError {
    fn from(value: Error) -> Self {
        match value {
            Error::Sqlx(e) => Self::from_sqlx_error(e),
            Error::NotFound => Self::new(StatusCode::NOT_FOUND, "requested item does not exist"),
            Error::WrongPassword => Self::new(StatusCode::UNAUTHORIZED, "wrong password"),
            Error::Bcrypt(e) => Self::from_bcrypt(e),
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
    pub fn from_bcrypt(e: bcrypt::BcryptError) -> Self {
        tracing::error!(bcrypt_error=%e);
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "password issues")
    }
}

fn serialize_status<S>(status: &StatusCode, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u16(status.as_u16())
}

pub struct WithCode<const CODE: u16, R: IntoResponse>(pub R);

impl<const CODE: u16, R: IntoResponse> IntoResponse for WithCode<CODE, R> {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::from_u16(CODE).unwrap(), self.0).into_response()
    }
}
impl<const CODE: u16, R: IntoResponse> From<R> for WithCode<CODE, R> {
    fn from(value: R) -> Self {
        Self(value)
    }
}

pub trait ResultExtensions<E> {
    fn report_noop_delete(self, context: impl Display) -> Result<(), E>;
}

impl<E> ResultExtensions<E> for Result<PgQueryResult, E> {
    fn report_noop_delete(self, context: impl Display) -> Result<(), E> {
        self.map(|qr| {
            if qr.rows_affected() == 0 {
                warn!(%context, "attempt to remove non-existent item");
            }
        })
    }
}

#[macro_export]
macro_rules! with {
  ($x:ident: $($($fields:ident).* = $val: expr), *) => {
      {
        let mut y = $x;
        $(y$(.$fields)* = $val;)*
        y
      }
  };
  ($x:expr => $($($fields:ident).* = $val: expr), *) => {
      {
        let mut y = $x;
        // TODO: Reuse arm #0
        $(y$(.$fields)* = $val;)*
        y
      }
  };
}
