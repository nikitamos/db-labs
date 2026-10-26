use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod service;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateTaskDto {
    pub project_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub deadline: chrono::DateTime<chrono::Utc>,
    pub status_id: i32,
    pub assignee_id: i32,
    pub estimated_time: i32,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateTaskDto {
    pub name: Option<String>,
    pub description: Option<String>,
    pub deadline: Option<chrono::DateTime<chrono::Utc>>,
    pub status_id: Option<i32>,
    pub assignee_id: Option<i32>,
    pub estimated_time: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct DependencyDto {
    pub dependent: i32,
    pub dependency: i32,
}
