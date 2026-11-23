use super::{CreateDiscussionDto, UpdateDiscussionDto};
use crate::{entities::{Page, PageRequest, discussion::DiscussionPost}, extract::WithRejection};
use aide::axum::routing::{delete_with as delete, post_with as post, get_with  as get};
use axum::extract::{Json, Path, Query};
use sqlx::PgPool;

type Unit = ();

#[autoendpoint::endpoints(error=crate::HttpError, router=aide::axum::ApiRouter, method=get, tag="discussion")]
#[async_trait::async_trait]
pub trait DiscussionService : Send + Sync + Clone + 'static{
    #[endpoint(ok=Json::<DiscussionPost>,path="/{id}")]
    async fn get_by_id(&self, #[extract(Path)] id: i32) -> crate::Result<DiscussionPost>;

    #[endpoint(ok=Json::<DiscussionPost>,path="/",method=post)]
    async fn create_post(
        &self,
        #[extract(WithRejection<Json>)] body: CreateDiscussionDto,
    ) -> crate::Result<DiscussionPost>;

    #[endpoint(ok=Json::<DiscussionPost>,path="/{id}",method=post)]
    async fn update_post(
        &self,
        #[extract(Path)] id: i32,
        #[extract(WithRejection<Json>)] body: UpdateDiscussionDto,
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
                task    AS task,
                author  AS author,
                title   AS "title!",
                body    AS "body!",
                parent  AS parent
            FROM read_discussion_post($1, $2)
            "#,
            crate::ACTOR_ID,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn create_post(&self, body: CreateDiscussionDto) -> crate::Result<DiscussionPost> {
        Ok(sqlx::query_as!(
            DiscussionPost,
            r#"
            SELECT
                id,
                task    AS task,
                author  AS author,
                title   AS "title!",
                body    AS "body!",
                parent  AS parent
            FROM write_discussion_post($1, $2, $3, $4, $5, $6)
            "#,
            crate::ACTOR_ID,
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
            SELECT
                id,
                task    AS task,
                author  AS author,
                title   AS "title!",
                body    AS "body!",
                parent  AS parent
            FROM modify_discussion_post($1, $2, $3, $4)
            "#,
            crate::ACTOR_ID,
            body.title,
            body.body,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete_post(&self, id: i32) -> crate::Result<()> {
        let affected = sqlx::query_scalar!(
            r#"SELECT delete_discussion_post($1, $2) AS "affected!""#,
            crate::ACTOR_ID,
            id
        )
        .fetch_one(&self.pool)
        .await?;
        if affected == 0 {
            tracing::warn!("attempt to remove non-existent discussion post");
        }
        Ok(())
    }

    async fn fetch_by_task(
        &self,
        task_id: i32,
        page: PageRequest,
    ) -> crate::Result<Page<DiscussionPost>> {
        let mut items = sqlx::query_as!(
            DiscussionPost,
            r#"
            SELECT
                id,
                task    AS task,
                author  AS author,
                title   AS "title!",
                body    AS "body!",
                parent  AS parent
            FROM read_discussion_posts_by_task($1, $2)
            "#,
            crate::ACTOR_ID,
            task_id
        )
        .fetch_all(&self.pool)
        .await?;

        items.retain(|post| post.id.unwrap_or_default() > page.offset);
        items.truncate(page.limit.max(0) as usize);

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
        let mut items = sqlx::query_as!(
            DiscussionPost,
            r#"
            SELECT
                id,
                task    AS task,
                author  AS author,
                title   AS "title!",
                body    AS "body!",
                parent  AS parent
            FROM read_discussion_posts_by_parent($1, $2)
            "#,
            crate::ACTOR_ID,
            parent_id
        )
        .fetch_all(&self.pool)
        .await?;

        items.retain(|post| post.id.unwrap_or_default() > page.offset);
        items.truncate(page.limit.max(0) as usize);

        if items.is_empty() {
            Ok(Page::empty(page.limit))
        } else {
            let last = items.last().and_then(|x| x.id).unwrap_or_default();
            Ok(Page::next(last, items, page))
        }
    }
}