use crate::{Result, entities::{Page, PageRequest, category::Category}};

pub trait TaskService {
    // TODO: crud
    fn add_dependency(&self, dependent: i32, dependency: i32) -> Result<()>;
    fn remove_dependency(&self, dependent: i32, dependency: i32) -> Result<()>;

    fn add_to_categories(&self, task_id: i32, categories: Vec<i32>) -> Result<()>;
    fn remove_from_categories(&self, task_id: i32, categories: Vec<i32>) -> Result<()>;
    fn find_by_category(&self, category: i32, page: PageRequest) -> Result<Page<Category>>;
}
