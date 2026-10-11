use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Category {
    pub(crate) id: Option<i32>,
    pub(crate) name: String,
    pub(crate) badge_color: i32,
    pub(crate) urgency: i32
}