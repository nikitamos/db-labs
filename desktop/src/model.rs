use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct RelationId {
    pub id: i32,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct PageRequest {
    pub limit: i64,
    pub offset: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_page: PageRequest,
}

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            next_page: PageRequest::default(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SessionDto {
    pub access_token: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct User {
    pub id: Option<i32>,
    pub username: String,
    pub email: String,
    pub first_name: String,
    pub second_name: String,
    pub bio: Option<String>,
}

impl User {
    pub fn display_name(&self) -> String {
        format!("{} {}", self.first_name, self.second_name)
            .trim()
            .to_string()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Project {
    pub id: Option<i32>,
    pub manager: RelationId,
    pub name: String,
    pub description: Option<String>,
    pub start_time: Option<String>,
    pub deadline: Option<String>,
    pub members: RelationId,
    pub tasks: RelationId,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Task {
    pub id: Option<i32>,
    pub name: String,
    pub description: Option<String>,
    pub deadline: String,
    pub status: RelationId,
    pub assignee: RelationId,
    pub estimated_time: i32,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Status {
    pub id: Option<i32>,
    pub name: String,
    pub color: i32,
    pub ready: bool,
    pub pending: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Category {
    pub id: Option<i32>,
    pub name: String,
    pub badge_color: i32,
    pub urgency: i32,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct DiscussionPost {
    pub id: Option<i32>,
    pub task: RelationId,
    pub author: RelationId,
    pub title: String,
    pub body: String,
    pub parent: RelationId,
}
