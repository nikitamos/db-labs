use serde::{Deserialize, Serialize};
use sqlx::{Executor, PgPool};

use crate::entities::{Page, PageRequest};

#[derive(sqlx::FromRow, Debug, Deserialize, Serialize)]
pub struct User {
    pub(crate) id: Option<i32>,
    pub(crate) username: String,
    #[serde(skip)]
    pub(crate) password_hash: Option<String>,
    pub(crate) email: String,
    pub(crate) first_name: String,
    pub(crate) second_name: String,
    pub(crate) bio: Option<String>,
}

impl User {
    pub fn check_password(&self, password: String) -> crate::Result<()> {
        if let Some(ref hash) = self.password_hash {
            bcrypt::verify(password, hash)?.ok_or(crate::Error::WrongPassword)
        } else {
            tracing::error!("attempt to check unpresent password");
            Err(crate::Error::WrongPassword)
        }
    }
}
