use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Status {
    pub(crate) id: Option<i32>,
    pub(crate) name: String,
    pub(crate) color: i32,
    pub(crate) ready: bool,
    pub(crate) pending: bool,
}
