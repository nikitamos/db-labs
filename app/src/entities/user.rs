use sqlx::PgPool;

use crate::entities::{Page, PageRequest};

#[derive(sqlx::FromRow, Debug)]
pub struct User {
    id: Option<i32>,
    username: String,
    password_hash: String,
    email: String,
    first_name: String,
    second_name: String,
    bio: Option<String>,
}

impl User {
    pub async fn create(&mut self, db: &PgPool) -> sqlx::Result<()> {
        if self.id.is_some() {
            // return error
        }
        *self = sqlx::query_as!(
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
        .await?;
        Ok(())
    }
    pub async fn fetch_by_id(id: i32, db: &PgPool) -> sqlx::Result<Self> {
        sqlx::query_as!(
            User,
            r#"
              SELECT * FROM users
              WHERE id = $1
            "#,
            id
        )
        .fetch_one(db)
        .await
    }
    pub async fn delete(self, db: &PgPool) -> sqlx::Result<bool> {
        sqlx::query(r#"DELETE FROM users WHERE id=$1"#)
            .bind(self.id)
            .execute(db)
            .await
            .map(|a| a.rows_affected() != 0)
    }
    pub async fn save_changes(&self, db: &PgPool) -> sqlx::Result<()> {
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
    pub async fn fetch_list(db: &PgPool, page: PageRequest) -> sqlx::Result<Page<Self>> {
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
}
