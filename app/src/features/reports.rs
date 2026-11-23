//! Read-only analytical endpoints backed by `app/sql/complex`.
//!
//! These handlers use runtime SQL instead of `sqlx::query!` macros so adding a
//! report does not require regenerating the offline query cache.

use aide::axum::{ApiRouter, routing::{get, get_with}};
use axum::{
    extract::{Query, State}, 
    Json, Router,
};
use chrono::NaiveDateTime;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::{AppState, HttpError, features::auth::UserId};

pub fn routes() -> ApiRouter<AppState> {
    ApiRouter::new()
        .api_route("/deadline-rank", get_with(deadline_rank, |x| x.tag("report")))
        .api_route("/deadlines-missed", get_with(deadlines_missed, |x| x.tag("report")))
        .api_route("/deadlines-next", get_with(deadlines_next, |x| x.tag("report")))
        .api_route("/least-busy", get_with(least_busy, |x| x.tag("report")))
        .api_route("/task-status-table", get_with(task_status_table, |x| x.tag("report")))
        .api_route("/tasks-per-urgency", get_with(tasks_per_urgency, |x| x.tag("report")))
        .api_route("/search/projects", get_with(search_projects, |x| x.tag("report")))
        .api_route("/search/tasks", get_with(search_tasks, |x| x.tag("report")))
        .api_route("/search/users", get_with(search_users, |x| x.tag("report")))
        // Workaround for the existing `read_statuses` function, whose declared
        // `integer` return type does not match the `Color` domain it selects.
        .api_route("/statuses", get_with(status_list, |x| x.tag("report")))
}

fn timestamp(value: NaiveDateTime) -> String {
    value.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn opt_timestamp(value: Option<NaiveDateTime>) -> Option<String> {
    value.map(timestamp)
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DeadlineRankQuery {
    project_id: i32,
}

/// Tasks of a project ordered by deadline within each assignee.
async fn deadline_rank(
    State(app): State<AppState>,
    _user: UserId,
    Query(query): Query<DeadlineRankQuery>,
) -> Result<Json<Value>, HttpError> {
    let inner = include_str!("../../sql/complex/deadline_rank.sql")
        .trim()
        .trim_end_matches(';');
    let sql = format!(
        r#"
        SELECT
            task_id,
            assignee_id,
            deadline_rank,
            deadline_gap::text AS deadline_gap,
            spare_time::text AS spare_time
        FROM ({inner}) AS ranked
        ORDER BY assignee_id NULLS LAST, deadline_rank
        "#
    );

    let rows = sqlx::query_as::<_, (i32, Option<i32>, i64, Option<String>, Option<String>)>(
        sqlx::AssertSqlSafe(sql.as_str()),
    )
    .bind(query.project_id)
        .fetch_all(&app.db)
        .await?;

    let items = rows
        .into_iter()
        .map(
            |(task_id, assignee_id, deadline_rank, deadline_gap, spare_time)| {
                json!({
                    "task_id": task_id,
                    "assignee_id": assignee_id,
                    "deadline_rank": deadline_rank,
                    "deadline_gap": deadline_gap,
                    "spare_time": spare_time,
                })
            },
        )
        .collect::<Vec<_>>();

    Ok(Json(Value::Array(items)))
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DeadlinesMissedQuery {
    user_id: i32,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Overdue, not-ready tasks of one employee.
async fn deadlines_missed(
    State(app): State<AppState>,
    _user: UserId,
    Query(query): Query<DeadlinesMissedQuery>,
) -> Result<Json<Value>, HttpError> {
    let rows = sqlx::query_as::<
        _,
        (
            i32,
            String,
            String,
            NaiveDateTime,
            i32,
            i32,
            Option<i32>,
            i32,
        ),
    >(include_str!("../../sql/complex/deadlines_missed.sql"))
    .bind(query.user_id)
    .bind(query.limit.unwrap_or(50))
    .bind(query.offset.unwrap_or(0))
    .fetch_all(&app.db)
    .await?;

    let items = rows
        .into_iter()
        .map(
            |(
                id,
                name,
                description,
                deadline,
                status_id,
                project_id,
                assignee_id,
                estimated_time,
            )| {
                json!({
                    "id": id,
                    "name": name,
                    "description": description,
                    "deadline": timestamp(deadline),
                    "status_id": status_id,
                    "project_id": project_id,
                    "assignee_id": assignee_id,
                    "estimated_time": estimated_time,
                })
            },
        )
        .collect::<Vec<_>>();

    Ok(Json(Value::Array(items)))
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct DeadlinesNextQuery {
    user_id: i32,
    limit: Option<i64>,
}

/// Upcoming, not-ready tasks of one employee.
async fn deadlines_next(
    State(app): State<AppState>,
    _user: UserId,
    Query(query): Query<DeadlinesNextQuery>,
) -> Result<Json<Value>, HttpError> {
    let rows = sqlx::query_as::<
        _,
        (
            i32,
            String,
            String,
            NaiveDateTime,
            i32,
            i32,
            Option<i32>,
            i32,
        ),
    >(include_str!("../../sql/complex/deadlines_next.sql"))
    .bind(query.user_id)
    .bind(query.limit.unwrap_or(20))
    .fetch_all(&app.db)
    .await?;

    let items = rows
        .into_iter()
        .map(
            |(
                id,
                name,
                description,
                deadline,
                status,
                project,
                assignee,
                estimated_time,
            )| {
                json!({
                    "id": id,
                    "name": name,
                    "description": description,
                    "deadline": timestamp(deadline),
                    "status": status,
                    "project": project,
                    "assignee": assignee,
                    "estimated_time": estimated_time,
                })
            },
        )
        .collect::<Vec<_>>();

    Ok(Json(Value::Array(items)))
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct LeastBusyQuery {
    project_id: Option<i32>,
    max_tasks: Option<i64>,
}

/// Employees whose task count is below `max_tasks`.
async fn least_busy(
    State(app): State<AppState>,
    _user: UserId,
    Query(query): Query<LeastBusyQuery>,
) -> Result<Json<Value>, HttpError> {
    let rows = sqlx::query_as::<_, (Option<i32>, i64)>(include_str!(
        "../../sql/complex/employee_least_busy.sql"
    ))
    .bind(query.project_id)
    .bind(query.max_tasks.unwrap_or(i64::MAX))
    .fetch_all(&app.db)
    .await?;

    let items = rows
        .into_iter()
        .map(|(assignee_id, task_count)| {
            json!({
                "assignee_id": assignee_id,
                "task_count": task_count,
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(Value::Array(items)))
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TaskStatusTableQuery {
    project_id: Option<i32>,
    user_id: Option<i32>,
}

/// Tasks belonging to a project or user, joined with their statuses.
///
/// The source query uses `SELECT *` over a FULL OUTER JOIN, which produces
/// duplicate column names. This wrapper uses explicit aliases so the result can
/// be serialized safely.
async fn task_status_table(
    State(app): State<AppState>,
    _user: UserId,
    Query(query): Query<TaskStatusTableQuery>,
) -> Result<Json<Value>, HttpError> {
    let rows = sqlx::query_as::<
        _,
        (
            Option<i32>,
            Option<String>,
            Option<i32>,
            Option<bool>,
            Option<bool>,
            Option<i32>,
            Option<String>,
            Option<String>,
            Option<NaiveDateTime>,
            Option<i32>,
            Option<i32>,
            Option<i32>,
            Option<i32>,
        ),
    >(
        r#"
        SELECT
            s.id            AS status_id,
            s.name          AS status_name,
            s.color::int    AS status_color,
            s.ready         AS status_ready,
            s.pending       AS status_pending,
            t.id            AS task_id,
            t.name          AS task_name,
            t.description   AS task_description,
            t.deadline      AS task_deadline,
            t.status_id     AS task_status_id,
            t.project_id    AS task_project_id,
            t.assignee_id   AS task_assignee_id,
            t.estimated_time AS task_estimated_time
        FROM statuses s
        FULL OUTER JOIN tasks t ON t.status_id = s.id
        WHERE ($1::int IS NULL OR t.project_id = $1)
          AND ($2::int IS NULL OR t.assignee_id = $2)
        ORDER BY s.id NULLS LAST, t.id NULLS LAST
        "#,
    )
    .bind(query.project_id)
    .bind(query.user_id)
    .fetch_all(&app.db)
    .await?;

    let items = rows
        .into_iter()
        .map(
            |(
                status_id,
                status_name,
                status_color,
                status_ready,
                status_pending,
                task_id,
                task_name,
                task_description,
                task_deadline,
                task_status_id,
                task_project_id,
                task_assignee_id,
                task_estimated_time,
            )| {
                json!({
                    "status_id": status_id,
                    "status_name": status_name,
                    "status_color": status_color,
                    "status_ready": status_ready,
                    "status_pending": status_pending,
                    "task_id": task_id,
                    "task_name": task_name,
                    "task_description": task_description,
                    "task_deadline": opt_timestamp(task_deadline),
                    "task_status_id": task_status_id,
                    "task_project_id": task_project_id,
                    "task_assignee_id": task_assignee_id,
                    "task_estimated_time": task_estimated_time,
                })
            },
        )
        .collect::<Vec<_>>();

    Ok(Json(Value::Array(items)))
}

async fn status_list(
    State(app): State<AppState>,
    _user: UserId,
) -> Result<Json<Value>, HttpError> {
    let rows = sqlx::query_as::<_, (i32, String, i32, bool, bool)>(
        r#"
        SELECT id, name, color::int, ready, pending
        FROM statuses
        ORDER BY id
        "#,
    )
    .fetch_all(&app.db)
    .await?;

    let items = rows
        .into_iter()
        .map(|(id, name, color, ready, pending)| {
            json!({
                "id": id,
                "name": name,
                "color": color,
                "ready": ready,
                "pending": pending,
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(json!({
        "items": items,
        "next_page": { "limit": 0, "offset": 0 },
    })))
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct TasksPerUrgencyQuery {
    user_id: i32,
}

/// Task counts per urgency level for one user.
async fn tasks_per_urgency(
    State(app): State<AppState>,
    _user: UserId,
    Query(query): Query<TasksPerUrgencyQuery>,
) -> Result<Json<Value>, HttpError> {
    let rows = sqlx::query_as::<_, (i64, Option<i32>)>(include_str!(
        "../../sql/complex/tasks_per_urgency.sql"
    ))
    .bind(query.user_id)
    .fetch_all(&app.db)
    .await?;

    let items = rows
        .into_iter()
        .map(|(task_count, urgency)| {
            json!({
                "task_count": task_count,
                "urgency": urgency,
            })
        })
        .collect::<Vec<_>>();

    Ok(Json(Value::Array(items)))
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchProjectsQuery {
    q: String,
    threshold: Option<f32>,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn search_projects(
    State(app): State<AppState>,
    _user: UserId,
    Query(query): Query<SearchProjectsQuery>,
) -> Result<Json<Value>, HttpError> {
    let rows = sqlx::query_as::<
        _,
        (
            i32,
            i32,
            String,
            Option<String>,
            Option<NaiveDateTime>,
            Option<NaiveDateTime>,
            f32,
        ),
    >(include_str!("../../sql/complex/text_search/project.sql"))
    .bind(&query.q)
    .bind(query.threshold.unwrap_or(0.0))
    .bind(query.limit.unwrap_or(50))
    .bind(query.offset.unwrap_or(0))
    .fetch_all(&app.db)
    .await?;

    let items = rows
        .into_iter()
        .map(
            |(id, manager_id, name, description, start_time, deadline, rank)| {
                json!({
                    "id": id,
                    "manager_id": manager_id,
                    "name": name,
                    "description": description,
                    "start_time": opt_timestamp(start_time),
                    "deadline": opt_timestamp(deadline),
                    "rank": rank,
                })
            },
        )
        .collect::<Vec<_>>();

    Ok(Json(Value::Array(items)))
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchTasksQuery {
    q: String,
    threshold: Option<f32>,
}

async fn search_tasks(
    State(app): State<AppState>,
    _user: UserId,
    Query(query): Query<SearchTasksQuery>,
) -> Result<Json<Value>, HttpError> {
    let rows = sqlx::query_as::<
        _,
        (i32, String, String, NaiveDateTime, i32, Option<i32>, i32, f32),
    >(include_str!("../../sql/complex/text_search/task.sql"))
    .bind(&query.q)
    .bind(query.threshold.unwrap_or(0.0))
    .fetch_all(&app.db)
    .await?;

    let items = rows
        .into_iter()
        .map(
            |(id, name, description, deadline, status, assignee, estimated_time, rank)| {
                json!({
                    "id": id,
                    "name": name,
                    "description": description,
                    "deadline": timestamp(deadline),
                    "status": status,
                    "assignee": assignee,
                    "estimated_time": estimated_time,
                    "rank": rank,
                })
            },
        )
        .collect::<Vec<_>>();

    Ok(Json(Value::Array(items)))
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
struct SearchUsersQuery {
    q: String,
    limit: Option<i64>,
    offset: Option<i64>,
}

async fn search_users(
    State(app): State<AppState>,
    _user: UserId,
    Query(query): Query<SearchUsersQuery>,
) -> Result<Json<Value>, HttpError> {
    let rows = sqlx::query_as::<
        _,
        (i32, String, String, String, String, Option<String>, f32),
    >(include_str!("../../sql/complex/text_search/user.sql"))
    .bind(&query.q)
    .bind(query.limit.unwrap_or(50))
    .bind(query.offset.unwrap_or(0))
    .fetch_all(&app.db)
    .await?;

    let items = rows
        .into_iter()
        .map(
            |(id, username, email, first_name, second_name, bio, rank)| {
                json!({
                    "id": id,
                    "username": username,
                    "email": email,
                    "first_name": first_name,
                    "second_name": second_name,
                    "bio": bio,
                    "rank": rank,
                })
            },
        )
        .collect::<Vec<_>>();

    Ok(Json(Value::Array(items)))
}
