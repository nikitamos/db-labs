use aide::axum::routing::{delete_with as delete, get_with as get, post_with as post};
use autoendpoint::endpoints;
use axum::extract::{FromRef, Json, Path, Query};
use sqlx::PgPool;

use crate::{
    entities::{Page, PageRequest, task::Task, user::User},
    extract::WithRejection,
    features::{
        auth::{UserId, jwt::JwtAuthState},
        user::UpdateUserDto,
    },
};

type Unit = ();

#[endpoints(error=crate::HttpError, router=aide::axum::ApiRouter, method=get, tag="users")]
#[async_trait::async_trait]
pub trait UserService: Clone + Send + Sync + 'static {
    #[endpoint(path="/identity/{identity}",ok=Json::<User>)]
    async fn get_by_username_or_email(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] identity: String,
    ) -> crate::Result<User>;

    #[endpoint(path="/{id}",ok=Json::<User>, method=post)]
    async fn update(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] id: i32,
        #[extract(WithRejection<Json>)] dto: UpdateUserDto,
    ) -> crate::Result<User>;

    #[endpoint(path="/{id}",ok=Unit,method=delete)]
    async fn delete(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] id: i32,
    ) -> crate::Result<()>;

    #[endpoint(path = "/",ok=Json::<Page::<User>>)]
    async fn fetch_list(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Query)] page: PageRequest,
    ) -> crate::Result<Page<User>>;

    #[endpoint(path="/{id}",ok=Json::<User>)]
    async fn get_by_id(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] id: i32,
    ) -> crate::Result<User>;

    #[endpoint(path="/{id}/tasks",ok=Json::<Page<Task>>)]
    async fn get_tasks(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] id: i32,
        #[extract(Query)] pagination: PageRequest
    ) -> crate::Result<Page<Task>>;
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
    async fn get_by_username_or_email(
        &self,
        actor_id: i32,
        identity: String,
    ) -> crate::Result<User> {
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
            actor_id,
            identity
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update(&self, actor_id: i32, id: i32, dto: UpdateUserDto) -> crate::Result<User> {
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
            actor_id,
            dto.first_name,
            dto.second_name,
            dto.bio,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn get_by_id(&self, actor_id: i32, id: i32) -> crate::Result<User> {
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
            actor_id,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete(&self, actor_id: i32, id: i32) -> crate::Result<()> {
        let affected =
            sqlx::query_scalar!(r#"SELECT delete_user($1, $2) AS "affected!""#, actor_id, id)
                .fetch_one(&self.pool)
                .await?;
        if affected == 0 {
            tracing::warn!("attempt to remove non-existent user");
        }
        Ok(())
    }
    async fn fetch_list(&self, actor_id: i32, page: PageRequest) -> crate::Result<Page<User>> {
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
            actor_id
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

    async fn get_tasks(&self, actor_id: i32, id: i32, pagination: PageRequest) -> crate::Result<Page<Task>> {
        let mut items = sqlx::query_as!(
            Task,
            r#"
            SELECT
                id,
                name            AS "name!",
                description,
                deadline        AS "deadline!",
                status          AS "status!",
                assignee,
                estimated_time  AS "estimated_time!"
            FROM read_tasks($1)
            WHERE assignee = $2 AND id > $3
            ORDER BY id
            LIMIT $4
            "#,
            actor_id,
            id,
            pagination.offset,
            pagination.limit
        )
        .fetch_all(&self.pool)
        .await?;

        items.retain(|task| task.id.unwrap_or_default() > pagination.offset);
        items.truncate(pagination.limit.max(0) as usize);

        if items.is_empty() {
            Ok(Page::empty(pagination.limit))
        } else {
            let last = items.last().and_then(|x| x.id).unwrap_or_default();
            Ok(Page::next(last, items, pagination))
        }
    }
}
