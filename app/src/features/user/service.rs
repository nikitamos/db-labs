use aide::axum::routing::{delete_with as delete, post_with as post, get_with as get};
use autoendpoint::endpoints;
use axum::extract::{Json, Path, Query};
use sqlx::PgPool;

use crate::{
    entities::{Page, PageRequest, user::User}, extract::WithRejection, features::user::UpdateUserDto,
};

type Unit = ();

#[endpoints(error=crate::HttpError, router=aide::axum::ApiRouter, method=get, tag="users")]
#[async_trait::async_trait]
pub trait UserService: Clone + Send + Sync + 'static {
    #[endpoint(path="/identity/{identity}",ok=Json::<User>)]
    async fn get_by_username_or_email(
        &self,
        #[extract(Path)] identity: String,
    ) -> crate::Result<User>;

    #[endpoint(path="/{id}",ok=Json::<User>, method=post)]
    async fn update(
        &self,
        #[extract(Path)] id: i32,
        #[extract(WithRejection<Json>)] dto: UpdateUserDto,
    ) -> crate::Result<User>;

    #[endpoint(path="/{id}",ok=Unit,method=delete)]
    async fn delete(&self, #[extract(Path)] id: i32) -> crate::Result<()>;

    #[endpoint(path = "/",ok=Json::<Page::<User>>)]
    async fn fetch_list(&self, #[extract(Query)] page: PageRequest) -> crate::Result<Page<User>>;

    #[endpoint(path="/{id}",ok=Json::<User>)]
    async fn get_by_id(&self, #[extract(Path)] id: i32) -> crate::Result<User>;
}

#[derive(Debug, Clone)]
pub struct SqlxUserService {
    pool: PgPool,
}

impl SqlxUserService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserService for SqlxUserService {
    async fn get_by_username_or_email(&self, identity: String) -> crate::Result<User> {
        Ok(sqlx::query_as!(
            User,
            r#"
                SELECT
                    id,
                    username         AS "username!",
                    password_hash,
                    email            AS "email!",
                    first_name       AS "first_name!",
                    second_name      AS "second_name!",
                    bio
                FROM read_user_by_identity($1, $2)
                "#,
            crate::ACTOR_ID,
            identity
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update(&self, id: i32, dto: UpdateUserDto) -> crate::Result<User> {
        Ok(sqlx::query_as!(
            User,
            r#"
                SELECT
                    id,
                    username         AS "username!",
                    password_hash,
                    email            AS "email!",
                    first_name       AS "first_name!",
                    second_name      AS "second_name!",
                    bio
                FROM modify_user_profile($1, $2, $3, $4, $5)
                "#,
            crate::ACTOR_ID,
            dto.first_name,
            dto.second_name,
            dto.bio,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn get_by_id(&self, id: i32) -> crate::Result<User> {
        Ok(sqlx::query_as!(
            User,
            r#"
                SELECT
                    id,
                    username         AS "username!",
                    password_hash,
                    email            AS "email!",
                    first_name       AS "first_name!",
                    second_name      AS "second_name!",
                    bio
                FROM read_user($1, $2)
                "#,
            crate::ACTOR_ID,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete(&self, id: i32) -> crate::Result<()> {
        let affected = sqlx::query_scalar!(
            r#"SELECT delete_user($1, $2) AS "affected!""#,
            crate::ACTOR_ID,
            id
        )
        .fetch_one(&self.pool)
        .await?;
        if affected == 0 {
            tracing::warn!("attempt to remove non-existent user");
        }
        Ok(())
    }
    async fn fetch_list(&self, page: PageRequest) -> crate::Result<Page<User>> {
        let mut items = sqlx::query_as!(
            User,
            r#"
              SELECT
                  id,
                  username         AS "username!",
                  password_hash,
                  email            AS "email!",
                  first_name       AS "first_name!",
                  second_name      AS "second_name!",
                  bio
              FROM read_users($1)
            "#,
            crate::ACTOR_ID
        )
        .fetch_all(&self.pool)
        .await?;

        items.retain(|user| user.id.unwrap_or_default() >= page.offset);
        items.truncate(page.limit.max(0) as usize);

        if items.is_empty() {
            Ok(Page::empty(page.limit))
        } else {
            let offset = items.last().and_then(|user| user.id).unwrap_or_default();
            Ok(Page::next(offset, items, page))
        }
    }
}
