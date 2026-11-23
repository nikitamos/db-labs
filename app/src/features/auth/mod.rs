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

/// Authenticated user extracted from the `Authorization: Bearer <jwt>` header.
///
/// The type parameter exists so the endpoint macro can use it like the other
/// generic extractors (`Path<T>`, `Query<T>`, ...). The only supported
/// instantiation is the default `UserId<i32>`.
#[derive(Debug)]
pub struct UserId<T = i32>(pub T);