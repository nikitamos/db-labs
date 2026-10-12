use serde::{Deserialize, Serialize};

pub mod service;

#[derive(Debug, Deserialize, Serialize)]
pub struct SignupDto {
    pub email: String,
    pub username: String,
    pub password: String,
    pub first_name: String,
    pub second_name: String,
    pub bio: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SigninDto {
    pub identity: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct SessionDto {
    pub access_token: String
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordDto {
    pub old_password: String,
    pub new_password: String
}