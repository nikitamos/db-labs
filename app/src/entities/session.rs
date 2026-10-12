use crate::entities::{Relation, user::User};

pub struct Session {
    pub(crate) id: i32,
    pub(crate) user: Relation<User>,
    pub(crate) auth_token: String
}