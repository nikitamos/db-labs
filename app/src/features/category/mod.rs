use serde::{Deserialize, Serialize};

pub mod service;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateCategoryDto {
    pub name: String,
    pub badge_color: i32,
    pub urgency: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateCategoryDto {
    pub name: Option<String>,
    pub badge_color: Option<i32>,
    pub urgency: Option<i32>,
}