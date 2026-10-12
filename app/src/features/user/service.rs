use autoendpoint::endpoints;
use axum::extract::{self, Json, Path, Query};
use axum::routing::{delete, post};
use sqlx::PgPool;

use crate::{
    ResultExtensions,
    entities::{Page, PageRequest, user::User},
    features::user::UpdateUserDto,
};

type Unit = ();

#[endpoints(error=crate::HttpError)]
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
        #[extract(Json)] dto: UpdateUserDto,
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
                SELECT * from users
                WHERE LOWER($1) IN (LOWER(users.email), LOWER(users.username))
                "#,
            identity
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update(&self, id: i32, dto: UpdateUserDto) -> crate::Result<User> {
        Ok(sqlx::query_as!(
            User,
            r#"
                UPDATE users SET
                first_name = $1,
                second_name = $2,
                bio = $3
                WHERE id = $4
                RETURNING
                    id, username, password_hash, email, first_name, second_name, bio
                "#,
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
                SELECT * FROM users
                WHERE id = $1
                "#,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete(&self, id: i32) -> crate::Result<()> {
        Ok(sqlx::query(r#"DELETE FROM users WHERE id=$1"#)
            .bind(id)
            .execute(&self.pool)
            .await?)
        .report_noop_delete("user")
    }
    async fn fetch_list(&self, page: PageRequest) -> crate::Result<Page<User>> {
        let x = sqlx::query_as!(
            User,
            r#"
              SELECT * from users
              WHERE id >= $1
              ORDER BY id
              LIMIT $2
            "#,
            page.offset,
            page.limit
        )
        .fetch_all(&self.pool)
        .await?;

        if x.is_empty() {
            Ok(Page::empty(page.limit))
        } else {
            let offset = x.last().unwrap().id.unwrap();
            Ok(Page {
                items: x,
                next_request: page.next_from(offset),
            })
        }
    }
}
