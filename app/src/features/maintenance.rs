use aide::axum::{
    ApiRouter,
    routing::{get, post},
};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::{AppState, HttpError};
#[derive(Debug, Default)]
pub enum HealthStatus {
    #[default]
    Healthy,
    DbSetupFailed(String),
}

impl IntoResponse for &HealthStatus {
    fn into_response(self) -> Response {
        match self {
            HealthStatus::Healthy => {
                (StatusCode::OK, Json(json!({"status":"pass"}))).into_response()
            }
            HealthStatus::DbSetupFailed(error) => {
                let message = Json(json!({
                    "status": "error",
                    "output": format!("failed to initialize db: {error}")
                }));
                (StatusCode::INTERNAL_SERVER_ERROR, message).into_response()
            }
        }
    }
}

pub async fn health_check(State(app): State<AppState>) -> Response {
    app.health.lock().into_response()
}

pub async fn populate_db(State(app): State<AppState>) -> Result<(), HttpError> {
    use crate::entities::ProjectRole;
    use crate::features::auth::{SignupDto, service::AuthService};
    use crate::features::category::{CreateCategoryDto, service::CategoryService};
    use crate::features::discussion::{CreateDiscussionDto, service::DiscussionService};
    use crate::features::project::{AddMemberDro, CreateProjectDto, service::ProjectService};
    use crate::features::status::{CreateStatusDto, service::StatusService};
    use crate::features::task::{CreateTaskDto, service::TaskService};
    use chrono::{Duration, Utc};

    tracing::info!("Populating database with sample data");

    // ------------------------------------------------------------------
    // 1. Users (no dependencies) — 6 records via AuthService::signup so
    //    passwords are properly hashed.
    // ------------------------------------------------------------------
    let mut users = Vec::with_capacity(6);
    for i in 0..6 {
        let user = app
            .auth
            .signup(SignupDto {
                email: format!("user{i}@example.com"),
                username: format!("user{i}"),
                password: "password123".to_string(),
                first_name: format!("First{i}"),
                second_name: format!("Last{i}"),
                bio: Some(format!("Sample bio for user {i}")),
            })
            .await?;
        users.push(user);
    }

    // ------------------------------------------------------------------
    // 2. Categories (no dependencies) — 5 records
    // ------------------------------------------------------------------
    let mut categories = Vec::with_capacity(5);
    for i in 0..5 {
        let category = app
            .category
            .create_category(CreateCategoryDto {
                name: format!("Category {i}"),
                badge_color: 0x00FF_0000u32.wrapping_add(i as u32) as i32,
                urgency: i + 1,
            })
            .await?;
        categories.push(category);
    }

    // ------------------------------------------------------------------
    // 3. Statuses (no dependencies) — 5 records
    // ------------------------------------------------------------------
    let mut statuses = Vec::with_capacity(5);
    for i in 0..5 {
        let status = app
            .status
            .create_status(CreateStatusDto {
                name: format!("Status {i}"),
                color: 0x0000_FF00u32.wrapping_add(i as u32) as i32,
                ready: i == 4,
                pending: i == 0,
            })
            .await?;
        statuses.push(status);
    }

    // ------------------------------------------------------------------
    // 4. Projects (depends on users) — 5 records.
    //    `create_project` automatically registers the manager as a member.
    // ------------------------------------------------------------------
    let mut projects = Vec::with_capacity(5);
    for i in 0..5 {
        let project = app
            .project
            .create_project(CreateProjectDto {
                manager_id: users[i].id.unwrap(),
                name: format!("Project {i}"),
                description: format!("Sample description for project {i}"),
                start_time: Utc::now(),
                deadline: Utc::now() + Duration::days(30 + i as i64),
            })
            .await?;
        projects.push(project);
    }

    // Add a couple of extra members to each project (skip project failures
    // silently if the user is already a member — the DB has a unique index).
    for (i, project) in projects.iter().enumerate() {
        let project_id = project.id.unwrap();
        for j in 1..=2usize {
            let member = &users[(i + j) % users.len()];
            let role = if j == 1 {
                ProjectRole::Worker
            } else {
                ProjectRole::Observer
            };
            let _ = app
                .project
                .add_member(
                    project_id,
                    AddMemberDro {
                        member_id: member.id.unwrap(),
                        role,
                    },
                )
                .await;
        }
    }

    // ------------------------------------------------------------------
    // 5. Tasks (depends on projects, statuses, users) — 7 records
    // ------------------------------------------------------------------
    let mut tasks = Vec::with_capacity(7);
    for i in 0..7usize {
        let task = app
            .task
            .create_task(CreateTaskDto {
                project_id: projects[i % projects.len()].id.unwrap(),
                name: format!("Task {i}"),
                description: Some(format!("Description for task {i}")),
                deadline: Utc::now() + Duration::days(7 + i as i64),
                status_id: statuses[i % statuses.len()].id.unwrap(),
                assignee_id: users[i % users.len()].id.unwrap(),
                estimated_time: (i as i32 + 1) * 60,
            })
            .await?;
        tasks.push(task);
    }

    // ------------------------------------------------------------------
    // 6. Task ↔ Category associations (depends on tasks & categories)
    // ------------------------------------------------------------------
    for (i, task) in tasks.iter().enumerate() {
        let associated = vec![
            categories[i % categories.len()].id.unwrap(),
            categories[(i + 1) % categories.len()].id.unwrap(),
        ];
        // Ignore conflict errors — DO NOTHING protects duplicates anyway.
        let _ = app
            .task
            .add_to_categories(task.id.unwrap(), associated)
            .await;
    }

    // ------------------------------------------------------------------
    // 7. Discussion posts (depends on tasks & users) — 5 records
    // ------------------------------------------------------------------
    for i in 0..5usize {
        app.discussion
            .create_post(CreateDiscussionDto {
                task_id: tasks[i % tasks.len()].id.unwrap(),
                author_id: users[i % users.len()].id.unwrap(),
                title: format!("Discussion {i}"),
                body: format!("Body of discussion post {i}"),
                parent_id: None,
            })
            .await?;
    }

    tracing::info!(
        users = users.len(),
        categories = categories.len(),
        statuses = statuses.len(),
        projects = projects.len(),
        tasks = tasks.len(),
        "Database population complete"
    );
    Ok(())
}

pub async fn reset_db(State(app): State<AppState>) -> Result<(), HttpError> {
    app.reset_db().await.map_err(crate::Error::from)?;
    Ok(())
}
pub async fn drop_db(State(app): State<AppState>) -> Result<(), HttpError> {
    app.drop_db_content().await.map_err(crate::Error::from)?;
    Ok(())
}
async fn truncate_db(State(app): State<AppState>) -> Result<(), crate::HttpError> {
    app.truncate_db().await.map_err(crate::Error::from)?;
    Ok(())
}

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route("/health", get(health_check))
        .api_route("/db_reset", post(reset_db))
        .api_route("/db_drop", post(drop_db))
        .api_route("/db_populate", post(populate_db))
        .api_route("/db_truncate", post(truncate_db))
}

// pub fn routes()
