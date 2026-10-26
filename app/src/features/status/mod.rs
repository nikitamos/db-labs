use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod service;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateStatusDto {
    pub name: String,
    pub color: i32,
    pub ready: bool,
    pub pending: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateStatusDto {
    pub name: Option<String>,
    pub color: Option<i32>,
    pub ready: Option<bool>,
    pub pending: Option<bool>,
}