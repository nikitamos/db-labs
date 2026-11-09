use super::{CreateCategoryDto, UpdateCategoryDto};
use crate::entities::{Page, PageRequest, category::Category};
use axum::extract::{Json, Path, Query};
use sqlx::PgPool;
use aide::axum::routing::{get,post, delete};

type Unit = ();

#[autoendpoint::endpoints(error=crate::HttpError, router=aide::axum::ApiRouter, method=get)]
#[async_trait::async_trait]
pub trait CategoryService: Send + Sync + Clone + 'static {
    #[endpoint(ok=Json::<Category>,path="/{id}")]
    async fn get_by_id(&self, #[extract(Path)] id: i32) -> crate::Result<Category>;

    #[endpoint(ok=Json::<Page::<Category>>, path="/")]
    async fn get_list(
        &self,
        #[extract(Query)] pagination: PageRequest,
    ) -> crate::Result<Page<Category>>;

    #[endpoint(ok=Json::<Category>,method=post,path="/")]
    async fn create_category(
        &self,
        #[extract(Json)] body: CreateCategoryDto,
    ) -> crate::Result<Category>;

    #[endpoint(ok=Json::<Category>,method=post,path="/{id}")]
    async fn update_category(
        &self,
        #[extract(Path)] id: i32,
        #[extract(Json)] body: UpdateCategoryDto,
    ) -> crate::Result<Category>;

    #[endpoint(ok=Unit,method=delete,path="/{id}")]
    async fn delete_category(&self, #[extract(Path)] id: i32) -> crate::Result<()>;
}

#[derive(Clone)]
pub struct SqlxCategoryService {
    pool: PgPool,
}

impl SqlxCategoryService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CategoryService for SqlxCategoryService {
    async fn get_by_id(&self, id: i32) -> crate::Result<Category> {
        Ok(sqlx::query_as!(
            Category,
            r#"
            SELECT
                id,
                name         AS "name!",
                badge_color  AS "badge_color!",
                urgency      AS "urgency!"
            FROM read_category($1, $2)
            "#,
            crate::ACTOR_ID,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn get_list(&self, pagination: PageRequest) -> crate::Result<Page<Category>> {
        let mut items = sqlx::query_as!(
            Category,
            r#"
            SELECT
                id,
                name         AS "name!",
                badge_color  AS "badge_color!",
                urgency      AS "urgency!"
            FROM read_categories($1)
            "#,
            crate::ACTOR_ID
        )
        .fetch_all(&self.pool)
        .await?;

        items.retain(|category| category.id.unwrap_or_default() > pagination.offset);
        items.truncate(pagination.limit.max(0) as usize);

        if items.is_empty() {
            Ok(Page::empty(pagination.limit))
        } else {
            let last = items.last().and_then(|x| x.id).unwrap_or_default();
            Ok(Page::next(last, items, pagination))
        }
    }

    async fn create_category(&self, body: CreateCategoryDto) -> crate::Result<Category> {
        // todo!()
        Ok(sqlx::query_as!(
            Category,
            r#"
            SELECT
                id,
                name         AS "name!",
                badge_color  AS "badge_color!",
                urgency      AS "urgency!"
            FROM write_category($1, $2, $3, $4)
            "#,
            crate::ACTOR_ID,
            body.name,
            body.badge_color,
            body.urgency
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update_category(
        &self,
        id: i32,
        body: UpdateCategoryDto,
    ) -> crate::Result<Category> {
        Ok(sqlx::query_as!(
            Category,
            r#"
            SELECT
                id,
                name         AS "name!",
                badge_color  AS "badge_color!",
                urgency      AS "urgency!"
            FROM modify_category($1, $2, $3, $4, $5)
            "#,
            crate::ACTOR_ID,
            body.name,
            body.badge_color,
            body.urgency,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete_category(&self, id: i32) -> crate::Result<()> {
        let affected = sqlx::query_scalar!(
            r#"SELECT delete_category($1, $2) AS "affected!""#,
            crate::ACTOR_ID,
            id
        )
        .fetch_one(&self.pool)
        .await?;
        if affected == 0 {
            tracing::warn!("attempt to remove non-existent category");
        }
        Ok(())
    }
}