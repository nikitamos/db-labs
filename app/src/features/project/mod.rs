use crate::entities::ProjectRole;
use chrono::Utc;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateProjectDto {
    pub manager_id: i32,
    pub name: String,
    pub description: String,
    pub start_time: chrono::DateTime<Utc>,
    pub deadline: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct UpdateProjectDto {
    pub manager_id: Option<i32>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub start_time: Option<chrono::DateTime<Utc>>,
    pub deadline: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AddMemberDro {
    pub member_id: i32,
    pub role: ProjectRole
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RemoveMemberDto {
    pub project_id: i32,
    pub user_id: i32
}

pub mod service;
