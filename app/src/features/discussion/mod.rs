use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
pub mod service;


pub fn routes() {
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateDiscussionDto {
    pub task_id: i32,
    pub author_id: i32,
    pub title: String,
    pub body: String,
    pub parent_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateDiscussionDto {
    pub title: Option<String>,
    pub body: Option<String>,
}