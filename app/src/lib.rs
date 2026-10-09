use sqlx::{
    Pool, Postgres, migrate::{MigrateError, Migrator}, postgres::PgQueryResult,
};
use tracing::{
    error,
    instrument::{WithDispatch, WithSubscriber},
};

use crate::config::AppConfig;

pub mod config;

#[derive(Clone)]
pub struct AppState {
    pub db: Pool<Postgres>,
}

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");
impl AppState {
    pub async fn from_config(cfg: AppConfig) -> Self {
        let pool = Pool::<Postgres>::connect(&cfg.app_db_uri).await.unwrap();
        Self { db: pool }
    }

    pub async fn migrate_db(&self) -> Result<(), MigrateError> {
        MIGRATOR
            .run(&self.db)
            .await
            .inspect_err(|e| error!("Failed to apply migrations: {e}"))
    }
    pub async fn drop_db_content(&self) -> sqlx::Result<PgQueryResult> {
        sqlx::raw_sql(
        r#"DROP SCHEMA public CASCADE;
           CREATE SCHEMA public;
        "#,
        ).execute(&self.db).await
    }
}
