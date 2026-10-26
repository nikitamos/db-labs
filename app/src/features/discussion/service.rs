use super::{CreateDiscussionDto, UpdateDiscussionDto};
use crate::{ResultExtensions, entities::{Page, PageRequest, discussion::DiscussionPost}};
use aide::axum::routing::{delete, post, get};
use axum::extract::{Json, Path, Query};
use sqlx::PgPool;

type Unit = ();

#[autoendpoint::endpoints(error=crate::HttpError, router=aide::axum::ApiRouter, method=get)]
#[async_trait::async_trait]
pub trait DiscussionService : Send + Sync + Clone + 'static{
    #[endpoint(ok=Json::<DiscussionPost>,path="/{id}")]
    async fn get_by_id(&self, #[extract(Path)] id: i32) -> crate::Result<DiscussionPost>;

    #[endpoint(ok=Json::<DiscussionPost>,path="/",method=post)]
    async fn create_post(
        &self,
        #[extract(Json)] body: CreateDiscussionDto,
    ) -> crate::Result<DiscussionPost>;

    #[endpoint(ok=Json::<DiscussionPost>,path="/{id}",method=post)]
    async fn update_post(
        &self,
        #[extract(Path)] id: i32,
        #[extract(Json)] body: UpdateDiscussionDto,
    ) -> crate::Result<DiscussionPost>;

    #[endpoint(ok=Unit,path="/{id}",method=delete)]
    async fn delete_post(&self, #[extract(Path)] id: i32) -> crate::Result<()>;

    #[endpoint(ok=Json::<Page::<DiscussionPost>>,path="/by_task/{task_id}")]
    async fn fetch_by_task(
        &self,
        #[extract(Path)] task_id: i32,
        #[extract(Query)] page: PageRequest,
    ) -> crate::Result<Page<DiscussionPost>>;

    #[endpoint(ok=Json::<Page::<DiscussionPost>>,path="/by_parent/{parent_id}")]
    async fn fetch_by_parent(
        &self,
        #[extract(Path)] parent_id: i32,
        #[extract(Query)] page: PageRequest,
    ) -> crate::Result<Page<DiscussionPost>>;
}

#[derive(Clone)]
pub struct SqlxDiscussionService {
    pool: PgPool,
}

impl SqlxDiscussionService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl DiscussionService for SqlxDiscussionService {
    async fn get_by_id(&self, id: i32) -> crate::Result<DiscussionPost> {
        Ok(sqlx::query_as!(
            DiscussionPost,
            r#"
            SELECT
                id,
                task_id    AS task,
                author_id  AS author,
                title,
                body,
                parent_id  AS parent
            FROM discussion_posts
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn create_post(&self, body: CreateDiscussionDto) -> crate::Result<DiscussionPost> {
        Ok(sqlx::query_as!(
            DiscussionPost,
            r#"
            INSERT INTO discussion_posts (task_id, author_id, title, body, parent_id)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING
                id,
                task_id    AS task,
                author_id  AS author,
                title,
                body,
                parent_id  AS parent
            "#,
            body.task_id,
            body.author_id,
            body.title,
            body.body,
            body.parent_id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update_post(
        &self,
        id: i32,
        body: UpdateDiscussionDto,
    ) -> crate::Result<DiscussionPost> {
        Ok(sqlx::query_as!(
            DiscussionPost,
            r#"
            UPDATE discussion_posts
            SET title = COALESCE($1, title),
                body  = COALESCE($2, body)
            WHERE id = $3
            RETURNING
                id,
                task_id    AS task,
                author_id  AS author,
                title,
                body,
                parent_id  AS parent
            "#,
            body.title,
            body.body,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete_post(&self, id: i32) -> crate::Result<()> {
        Ok(sqlx::query!(r#"DELETE FROM discussion_posts WHERE id = $1"#, id)
            .execute(&self.pool)
            .await
            .report_noop_delete("discussion")?)
    }

    async fn fetch_by_task(
        &self,
        task_id: i32,
        page: PageRequest,
    ) -> crate::Result<Page<DiscussionPost>> {
        let items = sqlx::query_as!(
            DiscussionPost,
            r#"
            SELECT
                id,
                task_id    AS task,
                author_id  AS author,
                title,
                body,
                parent_id  AS parent
            FROM discussion_posts
            WHERE id > $1 AND task_id = $2
            ORDER BY id
            LIMIT $3
            "#,
            page.offset,
            task_id,
            page.limit
        )
        .fetch_all(&self.pool)
        .await?;
        if items.is_empty() {
            Ok(Page::empty(page.limit))
        } else {
            let last = items.last().and_then(|x| x.id).unwrap_or_default();
            Ok(Page::next(last, items, page))
        }
    }

    async fn fetch_by_parent(
        &self,
        parent_id: i32,
        page: PageRequest,
    ) -> crate::Result<Page<DiscussionPost>> {
        let items = sqlx::query_as!(
            DiscussionPost,
            r#"
            SELECT
                id,
                task_id    AS task,
                author_id  AS author,
                title,
                body,
                parent_id  AS parent
            FROM discussion_posts
            WHERE id > $1 AND parent_id = $2
            ORDER BY id
            LIMIT $3
            "#,
            page.offset,
            parent_id,
            page.limit
        )
        .fetch_all(&self.pool)
        .await?;
        if items.is_empty() {
            Ok(Page::empty(page.limit))
        } else {
            let last = items.last().and_then(|x| x.id).unwrap_or_default();
            Ok(Page::next(last, items, page))
        }
    }
}