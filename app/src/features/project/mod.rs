use crate::entities::{project::Project, task::Task};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProjectDto {
    pub manager_id: i32,
    pub name: String,
    pub description: String,
    pub start_time: chrono::DateTime<Utc>,
    pub deadline: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProjectDto {
    pub manager_id: Option<i32>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub start_time: Option<chrono::DateTime<Utc>>,
    pub deadline: Option<chrono::DateTime<Utc>>,
}

mod service;
