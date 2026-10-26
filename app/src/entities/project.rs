use schemars::JsonSchema;
use serde::Serialize;

use crate::entities::{Page, Relation, user::User};

#[derive(Debug, Serialize, JsonSchema)]
pub struct Project {
    pub(crate) id: Option<i32>,
    pub(crate) manager: Relation<User>,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) start_time: Option<chrono::NaiveDateTime>,
    pub(crate) deadline: Option<chrono::NaiveDateTime>,
    pub(crate) members: Relation<Page<User>>,
    pub(crate) tasks: Relation<Page<()>>,
}

