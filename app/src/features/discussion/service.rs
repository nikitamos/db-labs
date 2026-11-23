use super::{CreateDiscussionDto, UpdateDiscussionDto};
use crate::{
    entities::{Page, PageRequest, discussion::DiscussionPost, user::User, Relation},
    extract::WithRejection,
    features::auth::UserId,
};
use aide::axum::routing::{delete_with as delete, get_with as get, post_with as post};
use axum::extract::{Json, Path, Query};
use sqlx::PgPool;

type Unit = ();

#[autoendpoint::endpoints(error=crate::HttpError, router=aide::axum::ApiRouter, method=get, tag="discussion")]
#[async_trait::async_trait]
pub trait DiscussionService: Send + Sync + Clone + 'static {
    #[endpoint(ok=Json::<DiscussionPost>,path="/{id}")]
    async fn get_by_id(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] id: i32,
    ) -> crate::Result<DiscussionPost>;

    #[endpoint(ok=Json::<DiscussionPost>,path="/",method=post)]
    async fn create_post(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(WithRejection<Json>)] body: CreateDiscussionDto,
    ) -> crate::Result<DiscussionPost>;

    #[endpoint(ok=Json::<DiscussionPost>,path="/{id}",method=post)]
    async fn update_post(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] id: i32,
        #[extract(WithRejection<Json>)] body: UpdateDiscussionDto,
    ) -> crate::Result<DiscussionPost>;

    #[endpoint(ok=Unit,path="/{id}",method=delete)]
    async fn delete_post(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] id: i32,
    ) -> crate::Result<()>;

    #[endpoint(ok=Json::<Page::<DiscussionPost>>,path="/by_task/{task_id}")]
    async fn fetch_by_task(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] task_id: i32,
        #[extract(Query)] page: PageRequest,
    ) -> crate::Result<Page<DiscussionPost>>;

    #[endpoint(ok=Json::<Page::<DiscussionPost>>,path="/by_parent/{parent_id}")]
    async fn fetch_by_parent(
        &self,
        #[extract(UserId)] actor_id: i32,
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
    async fn get_by_id(&self, actor_id: i32, id: i32) -> crate::Result<DiscussionPost> {
        Ok(sqlx::query_as!(
            DiscussionPost,
            r#"
            WITH authorized AS (
                SELECT id FROM read_discussion_post($1, $2)
            )
            SELECT
                post.id,
                post.task_id    AS task,
                CASE
                    WHEN post.author IS NULL THEN
                        jsonb_build_object('id', 0, 'referee', NULL)
                    ELSE
                        jsonb_build_object(
                            'id', (post.author->>'author.id')::int,
                            'referee', (post.author - 'author.id')
                                || jsonb_build_object(
                                    'id', (post.author->>'author.id')::int
                                )
                        )
                END AS "author!: sqlx::types::Json<Relation<User>>",
                discussion.title   AS "title!",
                post.body          AS "body!",
                post.parent_id     AS parent
            FROM post_with_author AS post
            JOIN discussion_posts AS discussion ON discussion.id = post.id
            JOIN authorized ON authorized.id = post.id
            "#,
            actor_id,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn create_post(
        &self,
        actor_id: i32,
        body: CreateDiscussionDto,
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
            FROM write_discussion_post($1, $2, $3, $4, $5, $6)
            "#,
            actor_id,
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
        actor_id: i32,
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
            actor_id,
            body.title,
            body.body,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete_post(&self, actor_id: i32, id: i32) -> crate::Result<()> {
        let affected = sqlx::query_scalar!(
            r#"SELECT delete_discussion_post($1, $2) AS "affected!""#,
            actor_id,
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
        actor_id: i32,
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
            actor_id,
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
        actor_id: i32,
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
            actor_id,
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
