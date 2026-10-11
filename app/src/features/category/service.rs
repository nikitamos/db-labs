use super::{CreateCategoryDto, UpdateCategoryDto};
use crate::{ResultExtensions, entities::{Page, PageRequest, category::Category}};
use axum::extract::{Json, Path, Query};
use sqlx::PgPool;

type Unit = ();

#[autoendpoint::endpoints(error=crate::HttpError)]
pub trait CategoryService {
    #[endpoint(ok=Category)]
    async fn get_by_id(&self, #[extract(Path)] id: i32) -> crate::Result<Category>;

    #[endpoint(ok=Page::<Category>)]
    async fn get_list(
        &self,
        #[extract(Query)] pagination: PageRequest,
    ) -> crate::Result<Page<Category>>;

    #[endpoint(ok=Category)]
    async fn create_category(
        &self,
        #[extract(Json)] body: CreateCategoryDto,
    ) -> crate::Result<Category>;

    #[endpoint(ok=Category)]
    async fn update_category(
        &self,
        #[extract(Path)] id: i32,
        #[extract(Json)] body: UpdateCategoryDto,
    ) -> crate::Result<Category>;

    #[endpoint(ok=Unit)]
    async fn delete_category(&self, #[extract(Path)] id: i32) -> crate::Result<()>;
}

pub struct SqlxCategoryService {
    pool: PgPool,
}

impl SqlxCategoryService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl CategoryService for SqlxCategoryService {
    async fn get_by_id(&self, id: i32) -> crate::Result<Category> {
        Ok(sqlx::query_as!(
            Category,
            r#"
            SELECT id, name, badge_color, urgency
            FROM categories
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn get_list(&self, pagination: PageRequest) -> crate::Result<Page<Category>> {
        let items = sqlx::query_as!(
            Category,
            r#"
            SELECT id, name, badge_color, urgency
            FROM categories
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
            Ok(Page::empty(pagination.limit)) // TODO: fix pagination
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
            INSERT INTO categories (name, badge_color, urgency)
            VALUES ($1, $2, $3)
            RETURNING id, name, badge_color, urgency
            "#,
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
            UPDATE categories
            SET name        = COALESCE($1, name),
                badge_color = COALESCE($2, badge_color),
                urgency     = COALESCE($3, urgency)
            WHERE id = $4
            RETURNING id, name, badge_color, urgency
            "#,
            body.name,
            body.badge_color,
            body.urgency,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete_category(&self, id: i32) -> crate::Result<()> {
        sqlx::query!(r#"DELETE FROM categories WHERE id = $1"#, id)
            .execute(&self.pool)
            .await.report_noop_delete("category")?;
        Ok(())
    }
}