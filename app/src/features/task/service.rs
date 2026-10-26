use super::{CreateTaskDto, UpdateTaskDto};
use crate::{ResultExtensions, entities::{Page, PageRequest, task::Task}, features::task::DependencyDto};
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
                name,
                description,
                deadline,
                status_id   AS status,
                assignee_id AS assignee,
                estimated_time
            FROM tasks
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn get_list(&self, pagination: PageRequest) -> crate::Result<Page<Task>> {
        let items = sqlx::query_as!(
            Task,
            r#"
            SELECT
                id,
                name,
                description,
                deadline,
                status_id   AS status,
                assignee_id AS assignee,
                estimated_time
            FROM tasks
            WHERE id > $1
            ORDER BY id
            LIMIT $2
            "#,
            pagination.offset,
            pagination.limit
        )
        .fetch_all(&self.pool)
        .await?;
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
            INSERT INTO tasks (
                project_id, name, description, deadline,
                status_id, assignee_id, estimated_time
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id,
                name,
                description,
                deadline,
                status_id   AS status,
                assignee_id AS assignee,
                estimated_time
            "#,
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
            UPDATE tasks
            SET name           = COALESCE($1, name),
                description    = COALESCE($2, description),
                deadline       = COALESCE($3, deadline),
                status_id      = COALESCE($4, status_id),
                assignee_id    = COALESCE($5, assignee_id),
                estimated_time = COALESCE($6, estimated_time)
            WHERE id = $7
            RETURNING
                id,
                name,
                description,
                deadline,
                status_id   AS status,
                assignee_id AS assignee,
                estimated_time
            "#,
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
        Ok(sqlx::query!(r#"DELETE FROM tasks WHERE id = $1"#, id)
            .execute(&self.pool)
            .await
            .report_noop_delete("task")?)
    }

    async fn add_dependency(&self, DependencyDto { dependent, dependency }: DependencyDto) -> crate::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO tasks_dependencies (dependent_id, dependency_id)
            VALUES ($1, $2)
            ON CONFLICT DO NOTHING
            "#,
            dependent,
            dependency
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_dependency(&self, DependencyDto { dependent, dependency }: DependencyDto) -> crate::Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM tasks_dependencies
            WHERE dependent_id = $1 AND dependency_id = $2
            "#,
            dependent,
            dependency
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn add_to_categories(&self, task_id: i32, categories: Vec<i32>) -> crate::Result<()> {
        if categories.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.begin().await?;
        for category_id in categories {
            sqlx::query!(
                r#"
                INSERT INTO task_categories (task_id, category_id)
                VALUES ($1, $2)
                ON CONFLICT DO NOTHING
                "#,
                task_id,
                category_id
            )
            .execute(&mut *tx)
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
        sqlx::query!(
            r#"
            DELETE FROM task_categories
            WHERE task_id = $1 AND category_id = ANY($2)
            "#,
            task_id,
            &categories
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_by_category(
        &self,
        category: i32,
        page: PageRequest,
    ) -> crate::Result<Page<Task>> {
        let items = sqlx::query_as!(
            Task,
            r#"
            SELECT
                t.id,
                t.name,
                t.description,
                t.deadline,
                t.status_id   AS status,
                t.assignee_id AS assignee,
                t.estimated_time
            FROM tasks t
            INNER JOIN task_categories tc ON tc.task_id = t.id
            WHERE t.id > $1 AND tc.category_id = $2
            ORDER BY t.id
            LIMIT $3
            "#,
            page.offset,
            category,
            page.limit
        )
        .fetch_all(&self.pool)
        .await?;
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

