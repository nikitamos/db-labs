use serde::Serialize;

use crate::entities::{Relation, task::Task, user::User};

#[derive(Debug, Serialize)]
pub struct DiscussionPost {
    pub(crate) id: Option<i32>,
    pub(crate) task: Relation<Task>,
    pub(crate) author: Relation<User>,
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) parent: Relation<Option<DiscussionPost>>
}
