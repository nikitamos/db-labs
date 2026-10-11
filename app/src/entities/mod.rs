use serde::{Deserialize, Serialize};

pub mod category;
pub mod project;
pub mod role;
pub mod status;
pub mod task;
pub mod user;
pub mod discussion;

#[derive(Debug, Serialize, Deserialize)]
pub struct PageRequest<Oft: Ord + Default = i32> {
    pub limit: i64,
    pub offset: Oft,
}

#[derive(Debug, Serialize)]
pub struct Relation<R> {
    pub id: i32,
    #[serde(rename = "referee")]
    pub value: Option<Box<R>>,
}

impl<R> From<i32> for Relation<R> {
    fn from(value: i32) -> Self {
        Relation {
            id: value,
            value: None,
        }
    }
}

impl<R> Relation<R> {
    pub fn id(&self) -> i32 {
        self.id
    }
}

impl<R> From<Option<i32>> for Relation<R> {
    fn from(value: Option<i32>) -> Self {
        Self::from(value.unwrap_or_default())
    }
}

impl<Oft: Ord + Default> PageRequest<Oft> {
    pub fn next_from(self, new_offset: Oft) -> Self {
        Self {
            limit: self.limit,
            offset: new_offset,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Page<T, Oft: Ord + Default = i32> {
    pub items: Vec<T>,
    #[serde(rename = "next_page")]
    pub next_request: PageRequest<Oft>,
}

impl<T, Oft: Ord + Default> Page<T, Oft> {
    pub fn empty(limit: i64) -> Self {
        Self {
            items: vec![],
            next_request: PageRequest {
                limit,
                offset: Default::default(),
            },
        }
    }

    pub fn next(new_offset: Oft, items: Vec<T>, request: PageRequest<Oft>) -> Self {
        Self {
            items,
            next_request: request.next_from(new_offset),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, sqlx::Type)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum ProjectRole {
    Observer,
    Manager,
    Worker,
}
