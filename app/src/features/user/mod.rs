use schemars::JsonSchema;
use serde::Deserialize;

pub mod service;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateUserDto {
    pub first_name: Option<String>,
    pub second_name: Option<String>,
    pub bio: Option<String>,
}
