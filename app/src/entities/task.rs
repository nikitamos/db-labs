
use chrono::{NaiveDateTime};
use schemars::JsonSchema;
use serde::Serialize;

use crate::entities::{Relation, status::Status, user::User};

#[derive(Debug, Serialize, JsonSchema)]
pub struct Task {
    pub(crate) id: Option<i32>,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) deadline: NaiveDateTime,
    pub(crate) status: Relation<Status>,
    pub(crate) assignee: Relation<User>,
    pub(crate) estimated_time: i32,
}
