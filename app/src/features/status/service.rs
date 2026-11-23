use super::{CreateStatusDto, UpdateStatusDto};
use crate::{entities::{Page, PageRequest, status::Status}, extract::WithRejection, features::auth::UserId};
use aide::axum::routing::{delete_with as delete, post_with as post, get_with as get};
use axum::extract::{Json, Path, Query};
use sqlx::PgPool;

type Unit = ();

#[autoendpoint::endpoints(error=crate::HttpError, router=aide::axum::ApiRouter, method=get, tag="status")]
#[async_trait::async_trait]
pub trait StatusService: Clone + Send + Sync + 'static {
    #[endpoint(ok=Json::<Status>,path="/{id}")]
    async fn get_by_id(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] id: i32,
    ) -> crate::Result<Status>;

    #[endpoint(ok=Json::<Page::<Status>>,path="/")]
    async fn get_list(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Query)] pagination: PageRequest,
    ) -> crate::Result<Page<Status>>;

    #[endpoint(ok=Json::<Status>,path="/",method=post)]
    async fn create_status(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(WithRejection<Json>)] body: CreateStatusDto,
    ) -> crate::Result<Status>;

    #[endpoint(ok=Json::<Status>,path="/{id}",method=post)]
    async fn update_status(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] id: i32,
        #[extract(WithRejection<Json>)] body: UpdateStatusDto,
    ) -> crate::Result<Status>;

    #[endpoint(ok=Unit,path="/{id}",method=delete)]
    async fn delete_status(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] id: i32,
    ) -> crate::Result<()>;
}

#[derive(Debug, Clone)]
pub struct SqlxStatusService {
    pool: PgPool,
}

impl SqlxStatusService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl StatusService for SqlxStatusService {
    async fn get_by_id(&self, actor_id: i32, id: i32) -> crate::Result<Status> {
        Ok(sqlx::query_as!(
            Status,
            r#"
            SELECT
                id,
                name     AS "name!",
                color    AS "color!",
                ready    AS "ready!",
                pending  AS "pending!"
            FROM read_status($1, $2)
            "#,
            actor_id,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn get_list(&self, actor_id: i32, pagination: PageRequest) -> crate::Result<Page<Status>> {
        let mut items = sqlx::query_as!(
            Status,
            r#"
            SELECT
                id,
                name     AS "name!",
                color    AS "color!",
                ready    AS "ready!",
                pending  AS "pending!"
            FROM read_statuses($1)
            "#,
            actor_id
        )
        .fetch_all(&self.pool)
        .await?;

        items.retain(|status| status.id.unwrap_or_default() > pagination.offset);
        items.truncate(pagination.limit.max(0) as usize);

        if items.is_empty() {
            Ok(Page::empty(pagination.limit))
        } else {
            let last = items.last().and_then(|x| x.id).unwrap_or_default();
            Ok(Page::next(last, items, pagination))
        }
    }

    async fn create_status(&self, actor_id: i32, body: CreateStatusDto) -> crate::Result<Status> {
        Ok(sqlx::query_as!(
            Status,
            r#"
            SELECT
                id,
                name     AS "name!",
                color    AS "color!",
                ready    AS "ready!",
                pending  AS "pending!"
            FROM write_status($1, $2, $3, $4, $5)
            "#,
            actor_id,
            body.name,
            body.color,
            body.ready,
            body.pending
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update_status(&self, actor_id: i32, id: i32, body: UpdateStatusDto) -> crate::Result<Status> {
        Ok(sqlx::query_as!(
            Status,
            r#"
            SELECT
                id,
                name     AS "name!",
                color    AS "color!",
                ready    AS "ready!",
                pending  AS "pending!"
            FROM modify_status($1, $2, $3, $4, $5, $6)
            "#,
            actor_id,
            body.name,
            body.color,
            body.ready,
            body.pending,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete_status(&self, actor_id: i32, id: i32) -> crate::Result<()> {
        let affected = sqlx::query_scalar!(
            r#"SELECT delete_status($1, $2) AS "affected!""#,
            actor_id,
            id
        )
        .fetch_one(&self.pool)
        .await?;
        if affected == 0 {
            tracing::warn!("attempt to remove non-existent status");
        }
        Ok(())
    }
}
