use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod service;
pub mod jwt;

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SignupDto {
    pub email: String,
    pub username: String,
    pub password: String,
    pub first_name: String,
    pub second_name: String,
    pub bio: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SigninDto {
    pub identity: String,
    pub password: String,
}

#[derive(Debug, Serialize, JsonSchema)]
pub struct SessionDto {
    pub access_token: String
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChangePasswordDto {
    pub old_password: String,
    pub new_password: String
}

#[derive(Debug)]
#[non_exhaustive]
pub struct UserId {
    pub user_id: i32,
}