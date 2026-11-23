use super::{CreateTaskDto, UpdateTaskDto};
use crate::{
    entities::{Page, PageRequest, task::Task, Relation, status::Status, user::User},
    extract::WithRejection,
    features::{auth::UserId, task::DependencyDto},
};
use aide::axum::routing::{delete_with as delete, get_with as get, post_with as post};
use axum::extract::{Json, Path, Query};
use chrono::DateTime;
use sqlx::PgPool;

type Unit = ();

#[autoendpoint::endpoints(error=crate::HttpError, router=aide::axum::ApiRouter, method=get, tag="task")]
#[async_trait::async_trait]
pub trait TaskService: Clone + Send + Sync + 'static {
    #[endpoint(ok=Json::<Task>,path="/{id}")]
    async fn get_by_id(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] id: i32,
    ) -> crate::Result<Task>;

    #[endpoint(ok=Json::<Page::<Task>>,path="/")]
    async fn get_list(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Query)] pagination: PageRequest,
    ) -> crate::Result<Page<Task>>;

    #[endpoint(ok=Json::<Task>,path="/",method=post)]
    async fn create_task(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(WithRejection<Json>)] body: CreateTaskDto,
    ) -> crate::Result<Task>;

    #[endpoint(ok=Json::<Task>,path="/{id}",method=post)]
    async fn update_task(
        &self,
        #[extract(Path)] id: i32,
        #[extract(UserId)] actor_id: i32,
        #[extract(WithRejection<Json>)] body: UpdateTaskDto,
    ) -> crate::Result<Task>;

    #[endpoint(ok=Unit,path="/{id}",method=delete)]
    async fn delete_task(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] id: i32,
    ) -> crate::Result<Unit>;

    #[endpoint(ok=Unit, path="/{dependent}/dep/{dependency}", method=post)]
    async fn add_dependency(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] dep: DependencyDto,
    ) -> crate::Result<()>;

    #[endpoint(ok=Unit, path="/{dependent}/dep/{dependency}", method=delete)]
    async fn remove_dependency(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] dep: DependencyDto,
    ) -> crate::Result<()>;

    #[endpoint(ok=Unit,path="/{task_id}/categories/add",method=post)]
    async fn add_to_categories(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] task_id: i32,
        #[extract(WithRejection<Json>)] categories: Vec<i32>,
    ) -> crate::Result<()>;

    #[endpoint(ok=Unit,path="/{task_id}/categories/delete",method=post)]
    async fn remove_from_categories(
        &self,
        #[extract(UserId)] actor_id: i32,
        #[extract(Path)] task_id: i32,
        #[extract(WithRejection<Json>)] categories: Vec<i32>,
    ) -> crate::Result<()>;

    #[endpoint(ok=Json::<Page::<Task>>,path="/by_category/{category}")]
    async fn find_by_category(
        &self,
        #[extract(UserId)] actor_id: i32,
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
    async fn get_by_id(&self, actor_id: i32, id: i32) -> crate::Result<Task> {
        Ok(sqlx::query_as!(
            Task,
            r#"
            WITH authorized AS (
                SELECT id FROM read_task($1, $2)
            )
            SELECT
                task.id,
                task.name            AS "name!",
                task.description,
                task.deadline        AS "deadline!",
                jsonb_build_object(
                    'id', (task.status->>'id')::int,
                    'referee', task.status
                ) AS "status!: sqlx::types::Json<Relation<Status>>",
                jsonb_build_object(
                    'id', (task.assignee->>'id')::int,
                    'referee', task.assignee
                ) AS "assignee!: sqlx::types::Json<Relation<User>>",
                task.estimated_time  AS "estimated_time!"
            FROM task_with_navigation_json AS task
            JOIN authorized ON authorized.id = task.id
            "#,
            actor_id,
            id
        )
        .fetch_one(&self.pool)
        .await?)
    }

    async fn get_list(&self, actor_id: i32, pagination: PageRequest) -> crate::Result<Page<Task>> {
        let mut items = sqlx::query_as!(
            Task,
            r#"
            SELECT
                id,
                name            AS "name!",
                description,
                deadline        AS "deadline!",
                status          AS "status!",
                assignee,
                estimated_time  AS "estimated_time!"
            FROM read_tasks($1)
            "#,
            actor_id
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

    async fn create_task(&self, actor_id: i32, body: CreateTaskDto) -> crate::Result<Task> {
        Ok(sqlx::query_as!(
            Task,
            r#"
            SELECT
                id,
                name            AS "name!",
                description,
                deadline        AS "deadline!",
                status          AS "status!",
                assignee,
                estimated_time  AS "estimated_time!"
            FROM write_task($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            actor_id,
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

    async fn update_task(
        &self,
        id: i32,
        actor_id: i32,
        body: UpdateTaskDto,
    ) -> crate::Result<Task> {
        Ok(sqlx::query_as!(
            Task,
            r#"
            SELECT
                id,
                name            AS "name!",
                description,
                deadline        AS "deadline!",
                status          AS "status!",
                assignee,
                estimated_time  AS "estimated_time!"
            FROM modify_task($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            actor_id,
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

    async fn delete_task(&self, actor_id: i32, id: i32) -> crate::Result<()> {
        let affected =
            sqlx::query_scalar!(r#"SELECT delete_task($1, $2) AS "affected!""#, actor_id, id)
                .fetch_one(&self.pool)
                .await?;
        if affected == 0 {
            tracing::warn!("attempt to remove non-existent task");
        }
        Ok(())
    }

    async fn add_dependency(
        &self,
        actor_id: i32,
        DependencyDto {
            dependent,
            dependency,
        }: DependencyDto,
    ) -> crate::Result<()> {
        let _ = sqlx::query_scalar!(
            r#"SELECT write_task_dependency($1, $2, $3) AS "affected!""#,
            actor_id,
            dependent,
            dependency
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_dependency(
        &self,
        actor_id: i32,
        DependencyDto {
            dependent,
            dependency,
        }: DependencyDto,
    ) -> crate::Result<()> {
        let _ = sqlx::query_scalar!(
            r#"SELECT delete_task_dependency($1, $2, $3) AS "affected!""#,
            actor_id,
            dependent,
            dependency
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(())
    }

    async fn add_to_categories(
        &self,
        actor_id: i32,
        task_id: i32,
        categories: Vec<i32>,
    ) -> crate::Result<()> {
        if categories.is_empty() {
            return Ok(());
        }
        let mut tx = self.pool.begin().await?;
        for category_id in categories {
            let _ = sqlx::query_scalar!(
                r#"SELECT write_task_category($1, $2, $3) AS "affected!""#,
                actor_id,
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
        actor_id: i32,
        task_id: i32,
        categories: Vec<i32>,
    ) -> crate::Result<()> {
        if categories.is_empty() {
            return Ok(());
        }
        let _ = sqlx::query_scalar!(
            r#"SELECT delete_task_categories($1, $2, $3) AS "affected!""#,
            actor_id,
            task_id,
            &categories
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(())
    }

    async fn find_by_category(
        &self,
        actor_id: i32,
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
                assignee,
                estimated_time  AS "estimated_time!"
            FROM read_tasks_by_category($1, $2)
            "#,
            actor_id,
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
