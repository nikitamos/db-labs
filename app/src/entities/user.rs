use schemars::JsonSchema;
use serde::Serialize;

#[derive(sqlx::FromRow, Debug, Serialize, JsonSchema)]
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
    pub fn check_password(&self, password: impl AsRef<str>) -> crate::Result<()> {
        if let Some(ref hash) = self.password_hash {
            bcrypt::verify(password.as_ref(), hash)?.ok_or(crate::Error::WrongPassword)
        } else {
            tracing::error!("attempt to check unpresent password");
            Err(crate::Error::WrongPassword)
        }
    }
    pub fn update_password(
        &mut self,
        old_password: impl AsRef<str>,
        new_password: impl AsRef<str>,
    ) -> crate::Result<&str> {
        self.check_password(old_password.as_ref())?;
        self.password_hash = Some(bcrypt::hash(new_password.as_ref(), 12)?);
        Ok(self.password_hash.as_ref().unwrap())
    }
}
