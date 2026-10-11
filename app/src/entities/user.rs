use serde::{Deserialize, Serialize};
use sqlx::{Executor, PgPool};

use crate::entities::{Page, PageRequest};

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize)]
pub struct User {
    id: Option<i32>,
    username: String,
    #[serde(skip)]
    password_hash: Option<String>,
    email: String,
    first_name: String,
    second_name: String,
    bio: Option<String>,
}

// #[endpoints]
// TODO: Hard-code list of correct signatures and
// automatic endpoints for:

// create       - post /{name}
// fetch_by_id  - get /{name}/{id}
// fetch_list   - get /{name}?page=XX&offset=YY
// delete_by_id - delete /{name}/{id}; post /{name}/{id}/delete
// save_changes - put /{name}/{id};

impl User {
    pub async fn create(
        self,
        db: impl Executor<'_, Database = sqlx::Postgres>,
    ) -> crate::Result<User> {
        if self.id.is_some() {
            // return error
        }
        Ok(sqlx::query_as!(
            User,
            r#"
             INSERT INTO users (username, password_hash, email, first_name, second_name, bio)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING *;
            "#,
            self.username,
            self.password_hash,
            self.email,
            self.first_name,
            self.second_name,
            self.bio
        )
        .fetch_one(db)
        .await?)
    }
    pub async fn fetch_by_id(id: i32, db: &PgPool) -> crate::Result<Self> {
        Ok(sqlx::query_as!(
            User,
            r#"
              SELECT * FROM users
              WHERE id = $1
            "#,
            id
        )
        .fetch_one(db)
        .await?)
    }
    pub async fn fetch_by_username(username: impl AsRef<str>, db: &PgPool) -> crate::Result<Self> {
        Ok(sqlx::query_as!(
            User,
            r#"
                    SELECT * FROM users
                    WHERE username = $1
                "#,
            username.as_ref()
        )
        .fetch_one(db)
        .await?)
    }
    pub async fn delete(self, db: &PgPool) -> crate::Result<bool> {
        if let Some(id) = self.id {
            Self::delete_by_id(id, db).await
        } else {
            Ok(false)
        }
    }
    pub async fn delete_by_id(id: i32, db: &PgPool) -> crate::Result<bool> {
        Ok(sqlx::query(r#"DELETE FROM users WHERE id=$1"#)
            .bind(id)
            .execute(db)
            .await
            .map(|a| a.rows_affected() != 0)?)
    }
    pub async fn save_changes(&self, db: &PgPool) -> crate::Result<()> {
        sqlx::query!(
            r#"
              UPDATE users
              SET username = $1,
                  password_hash=$2,
                  email=$3,
                  first_name=$4,
                  second_name=$5,
                  bio=$6
              WHERE id =$7;
            "#,
            self.username,
            self.password_hash,
            self.email,
            self.first_name,
            self.second_name,
            self.bio,
            self.id
        )
        .execute(db)
        .await?;
        Ok(())
    }
    pub async fn fetch_list(db: &PgPool, page: PageRequest) -> crate::Result<Page<Self>> {
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
        .fetch_all(db)
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
    // pub async fn get_roles()
}
