use super::CreateProjectDto;
use super::UpdateProjectDto;
use crate::entities::Page;
use crate::entities::PageRequest;
use crate::entities::project::Project;
use crate::entities::task::Task;
use axum::extract::{Json, Path, Query};
use chrono::DateTime;
use sqlx::PgExecutor;
use sqlx::PgPool;
use sqlx::Postgres;
use sqlx::query;

#[autoendpoint::endpoints(error=crate::HttpError)]
pub trait ProjectService {
    #[endpoint(ok=Project)]
    async fn get_by_id(&self, #[extract(Path)] id: i32) -> crate::Result<Project>;

    #[endpoint(ok=Page::<Task>)]
    async fn project_tasks(
        &self,
        #[extract(Path)] id: i32,
        #[extract(Query)] pagination: PageRequest,
    ) -> crate::Result<Page<Task>>;

    #[endpoint(ok=Page::<Project>)]
    async fn get_list(
        &self,
        #[extract(Query)] pagination: PageRequest,
    ) -> crate::Result<Page<Project>>;

    #[endpoint(ok=Project)]
    async fn create_project(
        &self,
        #[extract(Json)] body: CreateProjectDto,
    ) -> crate::Result<Project>;

    #[endpoint(ok=Project)]
    async fn update_project(
        &self,
        #[extract(Path)] id: i32,
        #[extract(Json)] body: UpdateProjectDto,
    ) -> crate::Result<Project>;
}

pub struct SqlxProjectService {
    pool: PgPool,
}

impl ProjectService for SqlxProjectService {
    async fn get_by_id(&self, id: i32) -> crate::Result<Project> {
        Ok(Self::fetch_full(&self.pool, id).await?)
    }

    // FIXME: move to correct service
    async fn project_tasks(&self, id: i32, pagination: PageRequest) -> crate::Result<Page<Task>> {
        let items = sqlx::query_as!(
            Task,
            r#"
                SELECT 
                    id as id,
                    name as name,
                    description as description,
                    deadline as deadline,
                    status_id as status,
                    assignee_id as assignee,
                    estimated_time
                FROM tasks
                WHERE (id > $1) AND (project_id = $2)
                ORDER BY id
                LIMIT $3
            "#,
            pagination.offset,
            id,
            pagination.limit
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(Page::next(
            items.first().and_then(|f| f.id).unwrap(),
            items,
            pagination,
        ))
    }

    async fn get_list(&self, pagination: PageRequest) -> crate::Result<Page<Project>> {
        let items = sqlx::query_as!(
            Project,
            r#"
            SELECT
                manager_id AS manager,
                id, name, description, start_time, deadline,
                0::int as members, 0::int as tasks
            FROM projects
            ORDER BY id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(Page::next(
            items.first().and_then(|f| f.id).unwrap(),
            items,
            pagination,
        ))
    }

    async fn create_project(
        &self,
        CreateProjectDto {
            manager_id,
            name,
            description,
            start_time,
            deadline,
        }: CreateProjectDto,
    ) -> crate::Result<Project> {
        let mut tx = self.pool.begin().await?;
        // TODO: trigger that adds manager to members
        let project_id = sqlx::query_scalar!(
            r#"
                INSERT INTO projects(
                    manager_id,
                    name,
                    description,
                    start_time,
                    deadline
                )
                VALUES ($1, $2, $3, $4, $5)
                RETURNING
                    id
            "#,
            manager_id,
            name,
            description,
            start_time.naive_utc(),
            deadline.naive_utc()
        )
        .fetch_one(&mut *tx)
        .await?;
        sqlx::query!(
            r#"
                INSERT INTO project_members(user_id, project_id, project_role)
                VALUES ($1, $2, 'manager')
            "#,
            manager_id,
            project_id
        )
        .execute(&mut *tx)
        .await?;
        let project = Self::fetch_full(&mut *tx, project_id).await?;
        tx.commit().await?;
        Ok(project)
    }

    async fn update_project(
        &self,
        id: i32,
        UpdateProjectDto {
            manager_id,
            name,
            description,
            start_time,
            deadline,
        }: UpdateProjectDto,
    ) -> crate::Result<Project> {
        let mut tx = self.pool.begin().await?;
        let old_mgr = sqlx::query_scalar!(
            r#"
            UPDATE projects
            SET manager_id  = COALESCE($1, manager_id),
                name        = COALESCE($2, name),
                description = COALESCE($3, description),
                start_time  = COALESCE($4, start_time),
                deadline    = COALESCE($5, deadline)
            WHERE id = $6
            RETURNING
                OLD.manager_id
            "#,
            manager_id,
            name,
            description,
            start_time.as_ref().map(DateTime::naive_utc),
            deadline.as_ref().map(DateTime::naive_utc),
            id
        )
        .fetch_one(&mut *tx)
        .await?;

        if let Some(new_mgr) = manager_id {
            sqlx::query!(
                r#"DELETE FROM project_members
                   WHERE user_id=$1 AND project_id=$2"#,
                old_mgr,
                id
            )
            .execute(&mut *tx)
            .await?;
            sqlx::query!(
                r#"INSERT INTO project_members(user_id, project_id)
                   VALUES ($1, $2)"#,
                new_mgr,
                id
            )
            .execute(&mut *tx)
            .await?;
        }
        let project = Self::fetch_no_nav(&mut *tx, id).await?;
        tx.commit().await?;
        Ok(project)
    }
}

impl SqlxProjectService {
    fn fetch_full<'a>(
        exec: impl PgExecutor<'a> + 'a,
        id: i32,
    ) -> impl Future<Output = Result<Project, sqlx::Error>> + 'a {
        // TODO: join
        sqlx::query_as!(
            Project,
            r#"
            SELECT
                manager_id AS manager,
                id, name, description, start_time, deadline,
                0::int as members, 0::int as tasks
            FROM projects
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(exec)
    }
    fn fetch_no_nav<'a>(
        exec: impl PgExecutor<'a> + 'a,
        id: i32,
    ) -> impl Future<Output = Result<Project, sqlx::Error>> + 'a {
        // TODO: join
        sqlx::query_as!(
            Project,
            r#"
            SELECT
                manager_id AS manager,
                id, name, description, start_time, deadline,
                0::int as members, 0::int as tasks
            FROM projects
            WHERE id = $1
            "#,
            id
        )
        .fetch_one(exec)
    }
}
