use super::CreateProjectDto;
use super::UpdateProjectDto;
use crate::entities::Page;
use crate::entities::PageRequest;
use crate::entities::ProjectRole;
use crate::entities::project::Project;
use crate::entities::task::Task;
use crate::features::project::AddMemberDro;
use crate::features::project::RemoveMemberDto;
use aide::axum::routing::{get,delete,post};
use axum::extract::{Json, Path, Query};
use chrono::DateTime;
use sqlx::PgExecutor;
use sqlx::PgPool;

type Unit = ();

#[autoendpoint::endpoints(error=crate::HttpError, router=aide::axum::ApiRouter,method=get)]
#[async_trait::async_trait]
pub trait ProjectService: Clone + Send + Sync + 'static {
    #[endpoint(ok=Json::<Project>,path="/{id}")]
    async fn get_by_id(&self, #[extract(Path)] id: i32) -> crate::Result<Project>;

    #[endpoint(ok=Json::<Page::<Task>>,path="/{id}/tasks")]
    async fn project_tasks(
        &self,
        #[extract(Path)] id: i32,
        #[extract(Query)] pagination: PageRequest,
    ) -> crate::Result<Page<Task>>;

    #[endpoint(ok=Json::<Page::<Project>>,path="/")]
    async fn get_list(
        &self,
        #[extract(Query)] pagination: PageRequest,
    ) -> crate::Result<Page<Project>>;

    #[endpoint(ok=Json::<Project>,path="/",method=post)]
    async fn create_project(
        &self,
        #[extract(Json)] body: CreateProjectDto,
    ) -> crate::Result<Project>;

    #[endpoint(ok=Json::<Project>,path="/{id}",method=post)]
    async fn update_project(
        &self,
        #[extract(Path)] id: i32,
        #[extract(Json)] body: UpdateProjectDto,
    ) -> crate::Result<Project>;

    #[endpoint(ok=Unit,path="/{id}/member",method=post)]
    async fn add_member(
        &self,
        #[extract(Path)] id: i32,
        #[extract(Json)] member: AddMemberDro,
    ) -> crate::Result<()>;

    #[endpoint(ok=Unit,path="/{id}",method=delete)]
    async fn delete_project(&self, #[extract(Path)] id: i32) -> crate::Result<()>;

    #[endpoint(ok=Unit,path="/{project_id}/member/{user_id}",method=delete)]
    async fn remove_member(&self, #[extract(Path)] dto: RemoveMemberDto) -> crate::Result<()>;

    async fn get_role(&self, project_id: i32, user_id: i32) -> crate::Result<Option<ProjectRole>>;
}

#[derive(Clone)]
pub struct SqlxProjectService {
    pool: PgPool,
}

impl SqlxProjectService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ProjectService for SqlxProjectService {
    async fn get_by_id(&self, id: i32) -> crate::Result<Project> {
        Ok(Self::fetch_full(&self.pool, id).await?)
    }

    async fn project_tasks(&self, id: i32, pagination: PageRequest) -> crate::Result<Page<Task>> {
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
                FROM read_project_tasks($1, $2)
            "#,
            crate::ACTOR_ID,
            id
        )
        .fetch_all(&self.pool)
        .await?;

        items.retain(|task| task.id.unwrap_or_default() > pagination.offset);
        items.truncate(pagination.limit.max(0) as usize);

        if items.is_empty() {
            Ok(Page::empty(pagination.limit))
        } else {
            let last = items.last().and_then(|task| task.id).unwrap_or_default();
            Ok(Page::next(last, items, pagination))
        }
    }

    async fn get_list(&self, pagination: PageRequest) -> crate::Result<Page<Project>> {
        let mut items = sqlx::query_as!(
            Project,
            r#"
            SELECT
                manager         AS "manager!",
                id,
                name            AS "name!",
                description,
                start_time,
                deadline,
                members         AS "members!",
                tasks           AS "tasks!"
            FROM read_projects($1)
            "#,
            crate::ACTOR_ID
        )
        .fetch_all(&self.pool)
        .await?;

        items.retain(|project| project.id.unwrap_or_default() > pagination.offset);
        items.truncate(pagination.limit.max(0) as usize);

        if items.is_empty() {
            Ok(Page::empty(pagination.limit))
        } else {
            let last = items.last().and_then(|project| project.id).unwrap_or_default();
            Ok(Page::next(last, items, pagination))
        }
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
                SELECT write_project($1, $2, $3, $4, $5, $6) AS "id!"
            "#,
            crate::ACTOR_ID,
            manager_id,
            name,
            description,
            start_time.naive_utc(),
            deadline.naive_utc()
        )
        .fetch_one(&mut *tx)
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
        let _ = sqlx::query_scalar!(
            "SELECT modify_project($1, $2, $3, $4, $5, $6, $7)",
            crate::ACTOR_ID,
            manager_id,
            name,
            description,
            start_time.as_ref().map(DateTime::naive_utc),
            deadline.as_ref().map(DateTime::naive_utc),
            id
        )
        .fetch_one(&mut *tx)
        .await?;
        let project = Self::fetch_no_nav(&mut *tx, id).await?;
        tx.commit().await?;
        Ok(project)
    }
    async fn add_member(&self, id: i32, member: AddMemberDro) -> crate::Result<()> {
        let _ = sqlx::query!(
            "SELECT write_project_member($1, $2, $3, $4)",
            crate::ACTOR_ID,
            member.member_id,
            id,
            member.role as ProjectRole
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(())
    }

    async fn remove_member(&self, dto: RemoveMemberDto) -> crate::Result<()> {
        let _ = sqlx::query_scalar!(
            r#"SELECT delete_project_member($1, $2, $3) AS "affected!""#,
            crate::ACTOR_ID,
            dto.project_id,
            dto.user_id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(())
    }

    async fn get_role(&self, project_id: i32, user_id: i32) -> crate::Result<Option<ProjectRole>> {
        let role = sqlx::query_scalar!(
            r#"
            SELECT project_role AS "project_role!: ProjectRole"
            FROM read_project_member_role($1, $2, $3)
            "#,
            crate::ACTOR_ID,
            project_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(role)
    }

    async fn delete_project(&self, id: i32) -> crate::Result<()> {
        let affected = sqlx::query_scalar!(
            r#"SELECT delete_project($1, $2) AS "affected!""#,
            crate::ACTOR_ID,
            id
        )
        .fetch_one(&self.pool)
        .await?;
        if affected == 0 {
            tracing::warn!("attempt to remove non-existent project");
        }
        Ok(())
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
                manager         AS "manager!",
                id,
                name            AS "name!",
                description,
                start_time,
                deadline,
                members         AS "members!",
                tasks           AS "tasks!"
            FROM read_project($1, $2)
            "#,
            crate::ACTOR_ID,
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
                manager         AS "manager!",
                id,
                name            AS "name!",
                description,
                start_time,
                deadline,
                members         AS "members!",
                tasks           AS "tasks!"
            FROM read_project($1, $2)
            "#,
            crate::ACTOR_ID,
            id
        )
        .fetch_one(exec)
    }
}
