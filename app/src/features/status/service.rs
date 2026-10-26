use super::{CreateStatusDto, UpdateStatusDto};
use crate::{ResultExtensions, entities::{Page, PageRequest, status::Status}};
use aide::axum::routing::{delete, post, get};
use axum::extract::{Json, Path, Query};
use sqlx::PgPool;

type Unit = ();

#[autoendpoint::endpoints(error=crate::HttpError, router=aide::axum::ApiRouter, method=get)]
#[async_trait::async_trait]
pub trait StatusService: Clone + Send + Sync + 'static {
    #[endpoint(ok=Json::<Status>,path="/{id}")]
    async fn get_by_id(&self, #[extract(Path)] id: i32) -> crate::Result<Status>;

    #[endpoint(ok=Json::<Page::<Status>>,path="/")]
    async fn get_list(
        &self,
        #[extract(Query)] pagination: PageRequest,
    ) -> crate::Result<Page<Status>>;

    #[endpoint(ok=Json::<Status>,path="/",method=post)]
    async fn create_status(&self, #[extract(Json)] body: CreateStatusDto) -> crate::Result<Status>;

    #[endpoint(ok=Json::<Status>,path="/{id}",method=post)]
    async fn update_status(
        &self,
        #[extract(Path)] id: i32,
        #[extract(Json)] body: UpdateStatusDto,
    ) -> crate::Result<Status>;

    #[endpoint(ok=Unit,path="/{id}",method=delete)]
    async fn delete_status(&self, #[extract(Path)] id: i32) -> crate::Result<()>;
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
    async fn get_by_id(&self, id: i32) -> crate::Result<Status> {
        Ok(sqlx::query_as!(
            Status,
            r#"
            SELECT id, name, color, ready, pending
            FROM statuses
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn get_list(&self, pagination: PageRequest) -> crate::Result<Page<Status>> {
        let items = sqlx::query_as!(
            Status,
            r#"
            SELECT id, name, color, ready, pending
            FROM statuses
            WHERE id > $1
            ORDER BY id
            LIMIT $2
            "#,
            pagination.offset,
            pagination.limit
        )
        .fetch_all(&self.pool)
        .await?;
        if items.is_empty() {
            Ok(Page::empty(pagination.limit))
        } else {
            let last = items.last().and_then(|x| x.id).unwrap_or_default();
            Ok(Page::next(last, items, pagination))
        }
    }

    async fn create_status(&self, body: CreateStatusDto) -> crate::Result<Status> {
        Ok(sqlx::query_as!(
            Status,
            r#"
            INSERT INTO statuses (name, color, ready, pending)
            VALUES ($1, $2, $3, $4)
            RETURNING id, name, color, ready, pending
            "#,
            body.name,
            body.color,
            body.ready,
            body.pending
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update_status(&self, id: i32, body: UpdateStatusDto) -> crate::Result<Status> {
        Ok(sqlx::query_as!(
            Status,
            r#"
            UPDATE statuses
            SET name    = COALESCE($1, name),
                color   = COALESCE($2, color),
                ready   = COALESCE($3, ready),
                pending = COALESCE($4, pending)
            WHERE id = $5
            RETURNING id, name, color, ready, pending
            "#,
            body.name,
            body.color,
            body.ready,
            body.pending,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete_status(&self, id: i32) -> crate::Result<()> {
        Ok(sqlx::query!(r#"DELETE FROM statuses WHERE id = $1"#, id)
            .execute(&self.pool)
            .await
            .report_noop_delete("status")?)
    }
}
