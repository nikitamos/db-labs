use std::convert::Infallible;

use chrono::{DateTime, NaiveDateTime, Utc};

use crate::entities::{Relation, user::User};

pub struct Task {
    pub(crate) id: Option<i32>,
    pub(crate) name: String,
    pub(crate) description: Option<String>,
    pub(crate) deadline: NaiveDateTime,
    pub(crate) status: Relation<()>,
    pub(crate) assignee: Relation<User>,
    pub(crate) estimated_time: i32,
}
