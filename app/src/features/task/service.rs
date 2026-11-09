use super::{CreateTaskDto, UpdateTaskDto};
use crate::{entities::{Page, PageRequest, task::Task}, features::task::DependencyDto};
use aide::axum::routing::{get,delete, post};
use axum::extract::{Json, Path, Query};
use chrono::DateTime;
use sqlx::PgPool;

type Unit = ();

#[autoendpoint::endpoints(error=crate::HttpError, router=aide::axum::ApiRouter, method=get)]
#[async_trait::async_trait]
pub trait TaskService : Clone + Send + Sync + 'static{
    #[endpoint(ok=Json::<Task>,path="/{id}")]
    async fn get_by_id(&self, #[extract(Path)] id: i32) -> crate::Result<Task>;

    #[endpoint(ok=Json::<Page::<Task>>,path="/")]
    async fn get_list(
        &self,
        #[extract(Query)] pagination: PageRequest,
    ) -> crate::Result<Page<Task>>;

    #[endpoint(ok=Json::<Task>,path="/",method=post)]
    async fn create_task(&self, #[extract(Json)] body: CreateTaskDto) -> crate::Result<Task>;

    #[endpoint(ok=Json::<Task>,path="/{id}",method=post)]
    async fn update_task(
        &self,
        #[extract(Path)] id: i32,
        #[extract(Json)] body: UpdateTaskDto,
    ) -> crate::Result<Task>;

    #[endpoint(ok=Unit,path="/{id}",method=delete)]
    async fn delete_task(&self, #[extract(Path)] id: i32) -> crate::Result<Unit>;

    #[endpoint(ok=Unit, path="/{dependent}/dep/{dependency}", method=post)]
    async fn add_dependency(
        &self,
        #[extract(Path)] dep: DependencyDto,
    ) -> crate::Result<()>;

    #[endpoint(ok=Unit, path="/{dependent}/dep/{dependency}", method=delete)]
    async fn remove_dependency(
        &self,
        #[extract(Path)] dep: DependencyDto,
    ) -> crate::Result<()>;


    #[endpoint(ok=Unit,path="/{task_id}/categories/add",method=post)]
    async fn add_to_categories(
        &self,
        #[extract(Path)] task_id: i32,
        #[extract(Json)] categories: Vec<i32>,
    ) -> crate::Result<()>;

    #[endpoint(ok=Unit,path="/{task_id}/categories/delete",method=post)]
    async fn remove_from_categories(
        &self,
        #[extract(Path)] task_id: i32,
        #[extract(Json)] categories: Vec<i32>,
    ) -> crate::Result<()>;

    #[endpoint(ok=Json::<Page::<Task>>,path="/by_category/{category}")]
    async fn find_by_category(
        &self,
        #[extract(Path)] category: i32,
        #[extract(Query)] page: PageRequest,
    ) -> crate::Result<Page<Task>>;
}

#[derive(Clone, Debug)]
pub struct SqlxTaskService {
    pool: PgPool,
}

impl SqlxTaskService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl TaskService for SqlxTaskService {
    async fn get_by_id(&self, id: i32) -> crate::Result<Task> {
        Ok(sqlx::query_as!(
            Task,
            r#"
            SELECT
                id,
                name            AS "name!",
                description,
                deadline        AS "deadline!",
                status          AS "status!",
                assignee AS "assignee?",
                estimated_time  AS "estimated_time!"
            FROM read_task($1, $2)
            "#,
            crate::ACTOR_ID,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn get_list(&self, pagination: PageRequest) -> crate::Result<Page<Task>> {
        let mut items = sqlx::query_as!(
            Task,
            r#"
            SELECT
                id,
                name            AS "name!",
                description,
                deadline        AS "deadline!",
                status          AS "status!",
                assignee AS "assignee?",
                estimated_time  AS "estimated_time!"
            FROM read_tasks($1)
            "#,
            crate::ACTOR_ID
        )
        .fetch_all(&self.pool)
        .await?;

        items.retain(|task| task.id.unwrap_or_default() > pagination.offset);
        items.truncate(pagination.limit.max(0) as usize);

        if items.is_empty() {
            Ok(Page::empty(pagination.limit))
        } else {
            let last = items.last().and_then(|x| x.id).unwrap_or_default();
            Ok(Page::next(last, items, pagination))
        }
    }

    async fn create_task(&self, body: CreateTaskDto) -> crate::Result<Task> {
        Ok(sqlx::query_as!(
            Task,
            r#"
            SELECT
                id,
                name            AS "name!",
                description,
                deadline        AS "deadline!",
                status          AS "status!",
                assignee AS "assignee?",
                estimated_time  AS "estimated_time!"
            FROM write_task($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            crate::ACTOR_ID,
            body.project_id,
            body.name,
            body.description,
            body.deadline.naive_utc(),
            body.status_id,
            body.assignee_id,
            body.estimated_time
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn update_task(&self, id: i32, body: UpdateTaskDto) -> crate::Result<Task> {
        Ok(sqlx::query_as!(
            Task,
            r#"
            SELECT
                id,
                name            AS "name!",
                description,
                deadline        AS "deadline!",
                status          AS "status!",
                assignee AS "assignee?",
                estimated_time  AS "estimated_time!"
            FROM modify_task($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            crate::ACTOR_ID,
            body.name,
            body.description,
            body.deadline.as_ref().map(DateTime::naive_utc),
            body.status_id,
            body.assignee_id,
            body.estimated_time,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn delete_task(&self, id: i32) -> crate::Result<()> {
        let affected = sqlx::query_scalar!(
            r#"SELECT delete_task($1, $2) AS "affected!""#,
            crate::ACTOR_ID,
            id
        )
        .fetch_one(&self.pool)
        .await?;
        if affected == 0 {
            tracing::warn!("attempt to remove non-existent task");
        }
        Ok(())
    }

    async fn add_dependency(&self, DependencyDto { dependent, dependency }: DependencyDto) -> crate::Result<()> {
        let _ = sqlx::query_scalar!(
            r#"SELECT write_task_dependency($1, $2, $3) AS "affected!""#,
            crate::ACTOR_ID,
            dependent,
            dependency
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_dependency(&self, DependencyDto { dependent, dependency }: DependencyDto) -> crate::Result<()> {
        let _ = sqlx::query_scalar!(
            r#"SELECT delete_task_dependency($1, $2, $3) AS "affected!""#,
            crate::ACTOR_ID,
            dependent,
            dependency
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(())
    }

    async fn add_to_categories(&self, task_id: i32, categories: Vec<i32>) -> crate::Result<()> {
        if categories.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.begin().await?;
        for category_id in categories {
            let _ = sqlx::query_scalar!(
                r#"SELECT write_task_category($1, $2, $3) AS "affected!""#,
                crate::ACTOR_ID,
                task_id,
                category_id
            )
            .fetch_one(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn remove_from_categories(
        &self,
        task_id: i32,
        categories: Vec<i32>,
    ) -> crate::Result<()> {
        if categories.is_empty() {
            return Ok(());
        }
        let _ = sqlx::query_scalar!(
            r#"SELECT delete_task_categories($1, $2, $3) AS "affected!""#,
            crate::ACTOR_ID,
            task_id,
            &categories
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_by_category(
        &self,
        category: i32,
        page: PageRequest,
    ) -> crate::Result<Page<Task>> {
        let mut items = sqlx::query_as!(
            Task,
            r#"
            SELECT
                id,
                name            AS "name!",
                description,
                deadline        AS "deadline!",
                status          AS "status!",
                assignee AS "assignee?",
                estimated_time  AS "estimated_time!"
            FROM read_tasks_by_category($1, $2)
            "#,
            crate::ACTOR_ID,
            category
        )
        .fetch_all(&self.pool)
        .await?;

        items.retain(|task| task.id.unwrap_or_default() > page.offset);
        items.truncate(page.limit.max(0) as usize);

        if items.is_empty() {
            Ok(Page::empty(page.limit))
        } else {
            let last = items.last().and_then(|x| x.id).unwrap_or_default();
            Ok(Page::next(last, items, page))
        }
    }
}

// async fn test(con: &PgPool) -> sqlx::Result<()> {
//     let res = sqlx::query_as!(Task, "SELECT * FROM task_with_navigation_json")
//         .fetch_one(con)
//         .await?;
//     Ok(())
// }

