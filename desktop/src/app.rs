use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use chrono::{DateTime, NaiveDate, NaiveDateTime, TimeZone, Utc};
use eframe::egui::{self, Color32, RichText, Ui};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::api::{encode, ApiClient};
use crate::model::*;

const DEFAULT_API_URL: &str = "http://localhost:6200";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Projects,
    Tasks,
    Statuses,
    Categories,
    Users,
    Discussions,
    Reports,
}

impl Tab {
    const ALL: [Self; 7] = [
        Self::Projects,
        Self::Tasks,
        Self::Statuses,
        Self::Categories,
        Self::Users,
        Self::Discussions,
        Self::Reports,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Projects => "Projects",
            Self::Tasks => "Tasks",
            Self::Statuses => "Statuses",
            Self::Categories => "Categories",
            Self::Users => "Users",
            Self::Discussions => "Discussions",
            Self::Reports => "Reports",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportKind {
    DeadlineRank,
    DeadlinesMissed,
    DeadlinesNext,
    LeastBusy,
    TaskStatus,
    TasksPerUrgency,
    SearchProjects,
    SearchTasks,
    SearchUsers,
}

impl ReportKind {
    const ALL: [Self; 9] = [
        Self::DeadlineRank,
        Self::DeadlinesMissed,
        Self::DeadlinesNext,
        Self::LeastBusy,
        Self::TaskStatus,
        Self::TasksPerUrgency,
        Self::SearchProjects,
        Self::SearchTasks,
        Self::SearchUsers,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::DeadlineRank => "Deadline rank by project",
            Self::DeadlinesMissed => "Missed deadlines by user",
            Self::DeadlinesNext => "Next deadlines by user",
            Self::LeastBusy => "Least busy employees",
            Self::TaskStatus => "Task/status table",
            Self::TasksPerUrgency => "Tasks per urgency",
            Self::SearchProjects => "Full-text project search",
            Self::SearchTasks => "Full-text task search",
            Self::SearchUsers => "Fuzzy user search",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Req {
    Login,
    Signup,
    Me,
    Users,
    Projects,
    Tasks,
    Statuses,
    Categories,
    Discussions,
    ProjectTasks,
    Report(ReportKind),
    Mutation(&'static str),
}

struct ApiMessage {
    tag: Req,
    result: Result<Value, String>,
}

struct AuthForm {
    base_url: String,
    identity: String,
    password: String,
    signup: bool,
    username: String,
    email: String,
    first_name: String,
    second_name: String,
    bio: String,
}

impl Default for AuthForm {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_API_URL.to_string(),
            identity: "user0".to_string(),
            password: "password123".to_string(),
            signup: false,
            username: "newuser".to_string(),
            email: "newuser@example.com".to_string(),
            first_name: "New".to_string(),
            second_name: "User".to_string(),
            bio: String::new(),
        }
    }
}

struct ProjectForm {
    manager_id: String,
    name: String,
    description: String,
    start: String,
    deadline: String,
}

impl Default for ProjectForm {
    fn default() -> Self {
        Self {
            manager_id: String::new(),
            name: "New project".to_string(),
            description: String::new(),
            start: format_input_datetime(Utc::now()),
            deadline: format_input_datetime(Utc::now() + chrono::Duration::days(30)),
        }
    }
}

struct TaskForm {
    project_id: String,
    name: String,
    description: String,
    deadline: String,
    status_id: String,
    assignee_id: String,
    estimated_time: String,
}

impl Default for TaskForm {
    fn default() -> Self {
        Self {
            project_id: String::new(),
            name: "New task".to_string(),
            description: String::new(),
            deadline: format_input_datetime(Utc::now() + chrono::Duration::days(7)),
            status_id: String::new(),
            assignee_id: String::new(),
            estimated_time: "60".to_string(),
        }
    }
}

struct StatusForm {
    name: String,
    color: String,
    ready: bool,
    pending: bool,
}

impl Default for StatusForm {
    fn default() -> Self {
        Self {
            name: "New status".to_string(),
            color: "0".to_string(),
            ready: false,
            pending: false,
        }
    }
}

struct CategoryForm {
    name: String,
    badge_color: String,
    urgency: String,
}

impl Default for CategoryForm {
    fn default() -> Self {
        Self {
            name: "New category".to_string(),
            badge_color: "0".to_string(),
            urgency: "1".to_string(),
        }
    }
}

struct DiscussionForm {
    task_id: String,
    parent_id: String,
    title: String,
    body: String,
}

impl Default for DiscussionForm {
    fn default() -> Self {
        Self {
            task_id: String::new(),
            parent_id: String::new(),
            title: "New discussion".to_string(),
            body: String::new(),
        }
    }
}

struct MemberForm {
    project_id: String,
    member_id: String,
    role: String,
}

impl Default for MemberForm {
    fn default() -> Self {
        Self {
            project_id: String::new(),
            member_id: String::new(),
            role: "worker".to_string(),
        }
    }
}

struct ProfileForm {
    first_name: String,
    second_name: String,
    bio: String,
}

impl Default for ProfileForm {
    fn default() -> Self {
        Self {
            first_name: String::new(),
            second_name: String::new(),
            bio: String::new(),
        }
    }
}

struct ReportState {
    kind: ReportKind,
    project_id: String,
    user_id: String,
    query: String,
    threshold: String,
    limit: String,
    offset: String,
    max_tasks: String,
    result: Option<Value>,
    error: Option<String>,
}

impl Default for ReportState {
    fn default() -> Self {
        Self {
            kind: ReportKind::DeadlineRank,
            project_id: "1".to_string(),
            user_id: "1".to_string(),
            query: String::new(),
            threshold: "0.0".to_string(),
            limit: "50".to_string(),
            offset: "0".to_string(),
            max_tasks: "100".to_string(),
            result: None,
            error: None,
        }
    }
}

pub struct DesktopApp {
    api: ApiClient,
    ctx: egui::Context,
    token: Option<String>,
    auth: AuthForm,
    current_user: Option<User>,
    me_identity: String,
    tab: Tab,

    users: Vec<User>,
    projects: Vec<Project>,
    tasks: Vec<Task>,
    statuses: Vec<Status>,
    categories: Vec<Category>,
    discussions: Vec<DiscussionPost>,
    project_tasks: Vec<Task>,
    project_tasks_title: String,

    project_form: ProjectForm,
    task_form: TaskForm,
    status_form: StatusForm,
    category_form: CategoryForm,
    discussion_form: DiscussionForm,
    member_form: MemberForm,
    profile_form: ProfileForm,
    report: ReportState,

    status: String,
    error: Option<String>,
    pending: usize,
    tx: Sender<ApiMessage>,
    rx: Receiver<ApiMessage>,
}

impl DesktopApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            api: ApiClient::new(DEFAULT_API_URL),
            ctx: cc.egui_ctx.clone(),
            token: None,
            auth: AuthForm::default(),
            current_user: None,
            me_identity: String::new(),
            tab: Tab::Projects,
            users: Vec::new(),
            projects: Vec::new(),
            tasks: Vec::new(),
            statuses: Vec::new(),
            categories: Vec::new(),
            discussions: Vec::new(),
            project_tasks: Vec::new(),
            project_tasks_title: String::new(),
            project_form: ProjectForm::default(),
            task_form: TaskForm::default(),
            status_form: StatusForm::default(),
            category_form: CategoryForm::default(),
            discussion_form: DiscussionForm::default(),
            member_form: MemberForm::default(),
            profile_form: ProfileForm::default(),
            report: ReportState::default(),
            status: "Not signed in".to_string(),
            error: None,
            pending: 0,
            tx,
            rx,
        }
    }

    fn poll(&mut self) {
        while let Ok(message) = self.rx.try_recv() {
            self.pending = self.pending.saturating_sub(1);
            self.handle_message(message);
        }
    }

    fn handle_message(&mut self, message: ApiMessage) {
        match message.tag {
            Req::Login => match message.result {
                Ok(value) => match serde_json::from_value::<SessionDto>(value) {
                    Ok(session) if !session.access_token.is_empty() => {
                        self.token = Some(session.access_token.clone());
                        self.api.set_token(Some(session.access_token));
                        self.status = "Signed in".to_string();
                        self.error = None;
                        self.me_identity = self.auth.identity.clone();
                        let identity = self.me_identity.clone();
                        self.spawn_get(
                            Req::Me,
                            format!("/user/identity/{}", encode(&identity)),
                        );
                        self.refresh_lists();
                    }
                    Ok(_) => self.error = Some("login did not return a token".to_string()),
                    Err(error) => {
                        self.error = Some(format!("invalid login response: {error}"))
                    }
                },
                Err(error) => self.error = Some(format!("login failed: {error}")),
            },
            Req::Signup => match message.result {
                Ok(_) => {
                    self.auth.identity = self.auth.username.clone();
                    self.status = "Account created; signing in...".to_string();
                    self.error = None;
                    self.do_login();
                }
                Err(error) => self.error = Some(format!("signup failed: {error}")),
            },
            Req::Me => match message.result {
                Ok(value) => match serde_json::from_value::<User>(value) {
                    Ok(user) => {
                        self.profile_form.first_name = user.first_name.clone();
                        self.profile_form.second_name = user.second_name.clone();
                        self.profile_form.bio = user.bio.clone().unwrap_or_default();
                        self.current_user = Some(user);
                        self.apply_user_defaults();
                    }
                    Err(error) => self.error = Some(format!("invalid user response: {error}")),
                },
                Err(error) => self.error = Some(format!("could not load current user: {error}")),
            },
            Req::Users => match parse_page::<User>(message.result) {
                Ok(users) => self.users = users,
                Err(error) => self.error = Some(error),
            },
            Req::Projects => match parse_page::<Project>(message.result) {
                Ok(projects) => self.projects = projects,
                Err(error) => self.error = Some(error),
            },
            Req::Tasks => match parse_page::<Task>(message.result) {
                Ok(tasks) => self.tasks = tasks,
                Err(error) => self.error = Some(error),
            },
            Req::Statuses => match parse_page::<Status>(message.result) {
                Ok(statuses) => self.statuses = statuses,
                Err(error) => self.error = Some(error),
            },
            Req::Categories => match parse_page::<Category>(message.result) {
                Ok(categories) => self.categories = categories,
                Err(error) => self.error = Some(error),
            },
            Req::Discussions => match parse_page::<DiscussionPost>(message.result) {
                Ok(posts) => self.discussions = posts,
                Err(error) => self.error = Some(error),
            },
            Req::ProjectTasks => match parse_page::<Task>(message.result) {
                Ok(tasks) => {
                    self.project_tasks = tasks;
                    self.status = format!("Loaded tasks for {}", self.project_tasks_title);
                }
                Err(error) => self.error = Some(error),
            },
            Req::Report(kind) => {
                if kind == self.report.kind {
                    match message.result {
                        Ok(value) => {
                            self.report.result = Some(value);
                            self.report.error = None;
                        }
                        Err(error) => {
                            self.report.error = Some(error);
                            self.report.result = None;
                        }
                    }
                }
            }
            Req::Mutation(label) => match message.result {
                Ok(_) => {
                    self.status = format!("{label}.");
                    self.error = None;
                    self.refresh_lists();
                }
                Err(error) => self.error = Some(format!("{label} failed: {error}")),
            },
        }
    }

    fn apply_user_defaults(&mut self) {
        let Some(id) = self.current_user.as_ref().and_then(|user| user.id) else {
            return;
        };
        let id = id.to_string();
        if self.project_form.manager_id.trim().is_empty() {
            self.project_form.manager_id = id.clone();
        }
        if self.task_form.assignee_id.trim().is_empty() {
            self.task_form.assignee_id = id.clone();
        }
        if self.member_form.member_id.trim().is_empty() {
            self.member_form.member_id = id.clone();
        }
        if self.report.user_id == "1" || self.report.user_id.trim().is_empty() {
            self.report.user_id = id;
        }
    }

    fn spawn_get(&mut self, tag: Req, path: String) {
        let api = self.api.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        self.pending += 1;
        std::thread::spawn(move || {
            let result = api.get(&path);
            let _ = tx.send(ApiMessage { tag, result });
            ctx.request_repaint();
        });
    }

    fn spawn_post(&mut self, tag: Req, path: String, body: Value) {
        let api = self.api.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        self.pending += 1;
        std::thread::spawn(move || {
            let result = api.post(&path, &body);
            let _ = tx.send(ApiMessage { tag, result });
            ctx.request_repaint();
        });
    }

    fn spawn_delete(&mut self, tag: Req, path: String) {
        let api = self.api.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        self.pending += 1;
        std::thread::spawn(move || {
            let result = api.delete(&path);
            let _ = tx.send(ApiMessage { tag, result });
            ctx.request_repaint();
        });
    }

    fn refresh_lists(&mut self) {
        self.spawn_get(Req::Projects, "/project?limit=100&offset=0".to_string());
        self.spawn_get(Req::Tasks, "/task?limit=100&offset=0".to_string());
        self.spawn_get(Req::Statuses, "/reports/statuses".to_string());
        self.spawn_get(Req::Categories, "/category?limit=100&offset=0".to_string());
        self.spawn_get(Req::Users, "/user?limit=100&offset=0".to_string());
    }

    fn do_login(&mut self) {
        let identity = self.auth.identity.trim().to_string();
        if identity.is_empty() || self.auth.password.is_empty() {
            self.error = Some("enter both identity and password".to_string());
            return;
        }
        self.api
            .set_base_url(self.auth.base_url.trim().to_string());
        let body = json!({
            "identity": identity,
            "password": self.auth.password,
        });
        self.spawn_post(Req::Login, "/auth/login".to_string(), body);
    }

    fn do_signup(&mut self) {
        self.api
            .set_base_url(self.auth.base_url.trim().to_string());
        let bio = self.auth.bio.trim();
        let body = json!({
            "email": self.auth.email.trim(),
            "username": self.auth.username.trim(),
            "password": self.auth.password,
            "first_name": self.auth.first_name.trim(),
            "second_name": self.auth.second_name.trim(),
            "bio": if bio.is_empty() { Value::Null } else { json!(bio) },
        });
        self.spawn_post(Req::Signup, "/auth/signup".to_string(), body);
    }

    fn logout(&mut self) {
        self.token = None;
        self.api.set_token(None);
        self.current_user = None;
        self.users.clear();
        self.projects.clear();
        self.tasks.clear();
        self.statuses.clear();
        self.categories.clear();
        self.discussions.clear();
        self.project_tasks.clear();
        self.project_tasks_title.clear();
        self.report.result = None;
        self.report.error = None;
        self.error = None;
        self.status = "Signed out".to_string();
        self.tab = Tab::Projects;
    }

    fn create_project(&mut self) {
        let manager_id = match parse_i32(&self.project_form.manager_id) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("manager id: {error}"));
                return;
            }
        };
        let name = self.project_form.name.trim();
        if name.is_empty() {
            self.error = Some("project name is required".to_string());
            return;
        }
        let start = match parse_datetime(&self.project_form.start) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("start time: {error}"));
                return;
            }
        };
        let deadline = match parse_datetime(&self.project_form.deadline) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("deadline: {error}"));
                return;
            }
        };
        let body = json!({
            "manager_id": manager_id,
            "name": name,
            "description": self.project_form.description.trim(),
            "start_time": start.to_rfc3339(),
            "deadline": deadline.to_rfc3339(),
        });
        self.spawn_post(
            Req::Mutation("Project created"),
            "/project".to_string(),
            body,
        );
    }

    fn create_task(&mut self) {
        let project_id = match parse_i32(&self.task_form.project_id) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("project id: {error}"));
                return;
            }
        };
        let status_id = match parse_i32(&self.task_form.status_id) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("status id: {error}"));
                return;
            }
        };
        let assignee_id = match parse_i32(&self.task_form.assignee_id) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("assignee id: {error}"));
                return;
            }
        };
        let estimated_time = match parse_i32(&self.task_form.estimated_time) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("estimated time: {error}"));
                return;
            }
        };
        let deadline = match parse_datetime(&self.task_form.deadline) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("deadline: {error}"));
                return;
            }
        };
        let body = json!({
            "project_id": project_id,
            "name": self.task_form.name.trim(),
            "description": self.task_form.description.trim(),
            "deadline": deadline.to_rfc3339(),
            "status_id": status_id,
            "assignee_id": assignee_id,
            "estimated_time": estimated_time,
        });
        self.spawn_post(Req::Mutation("Task created"), "/task".to_string(), body);
    }

    fn create_status(&mut self) {
        let color = match parse_i32(&self.status_form.color) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("color: {error}"));
                return;
            }
        };
        let body = json!({
            "name": self.status_form.name.trim(),
            "color": color,
            "ready": self.status_form.ready,
            "pending": self.status_form.pending,
        });
        self.spawn_post(
            Req::Mutation("Status created"),
            "/status".to_string(),
            body,
        );
    }

    fn create_category(&mut self) {
        let badge_color = match parse_i32(&self.category_form.badge_color) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("badge color: {error}"));
                return;
            }
        };
        let urgency = match parse_i32(&self.category_form.urgency) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("urgency: {error}"));
                return;
            }
        };
        let body = json!({
            "name": self.category_form.name.trim(),
            "badge_color": badge_color,
            "urgency": urgency,
        });
        self.spawn_post(
            Req::Mutation("Category created"),
            "/category".to_string(),
            body,
        );
    }

    fn create_discussion(&mut self) {
        let task_id = match parse_i32(&self.discussion_form.task_id) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("task id: {error}"));
                return;
            }
        };
        let author_id = self
            .current_user
            .as_ref()
            .and_then(|user| user.id)
            .unwrap_or(0);
        let parent_id = if self.discussion_form.parent_id.trim().is_empty() {
            Value::Null
        } else {
            match parse_i32(&self.discussion_form.parent_id) {
                Ok(value) => json!(value),
                Err(error) => {
                    self.error = Some(format!("parent id: {error}"));
                    return;
                }
            }
        };
        let body = json!({
            "task_id": task_id,
            "author_id": author_id,
            "title": self.discussion_form.title.trim(),
            "body": self.discussion_form.body.trim(),
            "parent_id": parent_id,
        });
        self.spawn_post(
            Req::Mutation("Discussion post created"),
            "/discussion".to_string(),
            body,
        );
    }

    fn add_member(&mut self) {
        let project_id = match parse_i32(&self.member_form.project_id) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("project id: {error}"));
                return;
            }
        };
        let member_id = match parse_i32(&self.member_form.member_id) {
            Ok(value) => value,
            Err(error) => {
                self.error = Some(format!("member id: {error}"));
                return;
            }
        };
        let body = json!({
            "member_id": member_id,
            "role": self.member_form.role,
        });
        self.spawn_post(
            Req::Mutation("Project member added"),
            format!("/project/{project_id}/member"),
            body,
        );
    }

    fn update_profile(&mut self) {
        let Some(id) = self.current_user.as_ref().and_then(|user| user.id) else {
            self.error = Some("no signed-in user".to_string());
            return;
        };
        let body = json!({
            "first_name": self.profile_form.first_name.trim(),
            "second_name": self.profile_form.second_name.trim(),
            "bio": if self.profile_form.bio.trim().is_empty() {
                Value::Null
            } else {
                json!(self.profile_form.bio.trim())
            },
        });
        self.spawn_post(
            Req::Mutation("Profile updated"),
            format!("/user/{id}"),
            body,
        );
    }

    fn load_discussions(&mut self) {
        let Ok(task_id) = parse_i32(&self.discussion_form.task_id) else {
            self.error = Some("enter a task id to load discussions".to_string());
            return;
        };
        self.spawn_get(
            Req::Discussions,
            format!("/discussion/by_task/{task_id}?limit=100&offset=0"),
        );
    }

    fn fetch_project_tasks(&mut self, project_id: i32, project_name: String) {
        self.project_tasks_title = format!("#{project_id} {project_name}");
        self.spawn_get(
            Req::ProjectTasks,
            format!("/project/{project_id}/tasks?limit=100&offset=0"),
        );
    }

    fn run_report(&mut self) {
        let kind = self.report.kind;
        let mut path = match kind {
            ReportKind::DeadlineRank => {
                let Ok(project_id) = parse_i32(&self.report.project_id) else {
                    self.report.error = Some("project id is required".to_string());
                    return;
                };
                format!("/reports/deadline-rank?project_id={project_id}")
            }
            ReportKind::DeadlinesMissed => {
                let Ok(user_id) = parse_i32(&self.report.user_id) else {
                    self.report.error = Some("user id is required".to_string());
                    return;
                };
                let mut path = format!("/reports/deadlines-missed?user_id={user_id}");
                add_param(&mut path, "limit", self.report.limit.trim());
                add_param(&mut path, "offset", self.report.offset.trim());
                path
            }
            ReportKind::DeadlinesNext => {
                let Ok(user_id) = parse_i32(&self.report.user_id) else {
                    self.report.error = Some("user id is required".to_string());
                    return;
                };
                let mut path = format!("/reports/deadlines-next?user_id={user_id}");
                add_param(&mut path, "limit", self.report.limit.trim());
                path
            }
            ReportKind::LeastBusy => {
                let mut path = "/reports/least-busy".to_string();
                if !self.report.project_id.trim().is_empty() {
                    add_param(&mut path, "project_id", self.report.project_id.trim());
                }
                add_param(&mut path, "max_tasks", self.report.max_tasks.trim());
                path
            }
            ReportKind::TaskStatus => {
                let mut path = "/reports/task-status-table".to_string();
                if !self.report.project_id.trim().is_empty() {
                    add_param(&mut path, "project_id", self.report.project_id.trim());
                }
                if !self.report.user_id.trim().is_empty() {
                    add_param(&mut path, "user_id", self.report.user_id.trim());
                }
                path
            }
            ReportKind::TasksPerUrgency => {
                let Ok(user_id) = parse_i32(&self.report.user_id) else {
                    self.report.error = Some("user id is required".to_string());
                    return;
                };
                format!("/reports/tasks-per-urgency?user_id={user_id}")
            }
            ReportKind::SearchProjects => {
                if self.report.query.trim().is_empty() {
                    self.report.error = Some("search query is required".to_string());
                    return;
                }
                let mut path = format!("/reports/search/projects?q={}", encode(self.report.query.trim()));
                add_param(&mut path, "threshold", self.report.threshold.trim());
                add_param(&mut path, "limit", self.report.limit.trim());
                add_param(&mut path, "offset", self.report.offset.trim());
                path
            }
            ReportKind::SearchTasks => {
                if self.report.query.trim().is_empty() {
                    self.report.error = Some("search query is required".to_string());
                    return;
                }
                let mut path = format!("/reports/search/tasks?q={}", encode(self.report.query.trim()));
                add_param(&mut path, "threshold", self.report.threshold.trim());
                path
            }
            ReportKind::SearchUsers => {
                if self.report.query.trim().is_empty() {
                    self.report.error = Some("search query is required".to_string());
                    return;
                }
                let mut path = format!("/reports/search/users?q={}", encode(self.report.query.trim()));
                add_param(&mut path, "limit", self.report.limit.trim());
                add_param(&mut path, "offset", self.report.offset.trim());
                path
            }
        };

        if !path.contains('?') {
            path.push('?');
        }
        self.report.result = None;
        self.report.error = None;
        self.spawn_get(Req::Report(kind), path);
    }

    fn login_ui(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(24.0);
            ui.vertical_centered(|ui| {
                ui.heading(RichText::new("Project Management Desktop").size(28.0));
                ui.label("egui client for the Axum/SQLx backend");
                ui.add_space(12.0);
            });

            ui.horizontal(|ui| {
                ui.label("API URL");
                ui.add(
                    egui::TextEdit::singleline(&mut self.auth.base_url)
                        .desired_width(360.0)
                        .hint_text("http://localhost:6200"),
                );
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(!self.auth.signup, "Login")
                    .clicked()
                {
                    self.auth.signup = false;
                }
                if ui
                    .selectable_label(self.auth.signup, "Sign up")
                    .clicked()
                {
                    self.auth.signup = true;
                }
            });
            ui.separator();

            if self.auth.signup {
                egui::Grid::new("signup-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Username");
                        ui.text_edit_singleline(&mut self.auth.username);
                        ui.end_row();
                        ui.label("Email");
                        ui.text_edit_singleline(&mut self.auth.email);
                        ui.end_row();
                        ui.label("Password");
                        ui.add(egui::TextEdit::singleline(&mut self.auth.password).password(true));
                        ui.end_row();
                        ui.label("First name");
                        ui.text_edit_singleline(&mut self.auth.first_name);
                        ui.end_row();
                        ui.label("Second name");
                        ui.text_edit_singleline(&mut self.auth.second_name);
                        ui.end_row();
                        ui.label("Bio");
                        ui.text_edit_multiline(&mut self.auth.bio);
                        ui.end_row();
                    });
                if ui.button("Create account and sign in").clicked() {
                    self.do_signup();
                }
            } else {
                egui::Grid::new("login-grid")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Username or email");
                        ui.text_edit_singleline(&mut self.auth.identity);
                        ui.end_row();
                        ui.label("Password");
                        ui.add(egui::TextEdit::singleline(&mut self.auth.password).password(true));
                        ui.end_row();
                    });
                if ui.button("Login").clicked() {
                    self.do_login();
                }
            }

            ui.add_space(12.0);
            if let Some(error) = &self.error {
                ui.colored_label(Color32::LIGHT_RED, error);
            } else {
                ui.colored_label(Color32::GRAY, &self.status);
            }
            if self.pending > 0 {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(format!("{} request(s) in flight", self.pending));
                });
            }
        });
    }

    fn main_ui(&mut self, ui: &mut egui::Ui) {
        let who = self
            .current_user
            .as_ref()
            .map(|user| {
                format!(
                    "#{} {} ({})",
                    user.id.unwrap_or_default(),
                    user.username,
                    user.display_name()
                )
            })
            .unwrap_or_else(|| self.me_identity.clone());

        egui::Panel::top("top-bar").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.heading("PM Desktop");
                ui.separator();
                for tab in Tab::ALL {
                    if ui
                        .selectable_label(self.tab == tab, tab.label())
                        .clicked()
                    {
                        self.tab = tab;
                    }
                }
                ui.separator();
                if ui.button("Refresh").clicked() {
                    self.refresh_lists();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Logout").clicked() {
                        self.logout();
                    }
                    ui.label(who);
                });
            });
            ui.add_space(4.0);
        });

        egui::Panel::bottom("bottom-bar").show(ui, |ui| {
            ui.horizontal(|ui| {
                if self.pending > 0 {
                    ui.spinner();
                    ui.label(format!("{} request(s) in flight", self.pending));
                } else {
                    ui.label("Ready");
                }
                ui.separator();
                if let Some(error) = &self.error {
                    ui.colored_label(Color32::LIGHT_RED, truncate(error, 180));
                } else {
                    ui.label(&self.status);
                }
            });
        });

        egui::CentralPanel::default().show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match self.tab {
                    Tab::Projects => self.projects_ui(ui),
                    Tab::Tasks => self.tasks_ui(ui),
                    Tab::Statuses => self.statuses_ui(ui),
                    Tab::Categories => self.categories_ui(ui),
                    Tab::Users => self.users_ui(ui),
                    Tab::Discussions => self.discussions_ui(ui),
                    Tab::Reports => self.reports_ui(ui),
                });
        });
    }

    fn projects_ui(&mut self, ui: &mut Ui) {
        ui.heading("Projects");
        ui.horizontal(|ui| {
            ui.label(format!("{} loaded", self.projects.len()));
            if ui.button("Refresh projects").clicked() {
                self.spawn_get(Req::Projects, "/project?limit=100&offset=0".to_string());
            }
        });

        egui::CollapsingHeader::new("Create project")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("project-form")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Manager id");
                        ui.text_edit_singleline(&mut self.project_form.manager_id);
                        ui.end_row();
                        ui.label("Name");
                        ui.text_edit_singleline(&mut self.project_form.name);
                        ui.end_row();
                        ui.label("Description");
                        ui.text_edit_multiline(&mut self.project_form.description);
                        ui.end_row();
                        ui.label("Start (RFC3339)");
                        ui.text_edit_singleline(&mut self.project_form.start);
                        ui.end_row();
                        ui.label("Deadline (RFC3339)");
                        ui.text_edit_singleline(&mut self.project_form.deadline);
                        ui.end_row();
                    });
                if ui.button("Create project").clicked() {
                    self.create_project();
                }
            });

        egui::CollapsingHeader::new("Add project member")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("member-form")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Project id");
                        ui.text_edit_singleline(&mut self.member_form.project_id);
                        ui.end_row();
                        ui.label("User id");
                        ui.text_edit_singleline(&mut self.member_form.member_id);
                        ui.end_row();
                        ui.label("Role");
                        egui::ComboBox::from_id_salt("member-role")
                            .selected_text(&self.member_form.role)
                            .show_ui(ui, |ui| {
                                for role in ["worker", "manager", "observer"] {
                                    ui.selectable_value(
                                        &mut self.member_form.role,
                                        role.to_string(),
                                        role,
                                    );
                                }
                            });
                        ui.end_row();
                    });
                if ui.button("Add member").clicked() {
                    self.add_member();
                }
            });

        ui.separator();
        let projects = self.projects.clone();
        egui::ScrollArea::horizontal().show(ui, |ui| {
            egui::Grid::new("projects-table")
                .striped(true)
                .num_columns(7)
                .spacing([14.0, 4.0])
                .show(ui, |ui| {
                    for header in [
                        "ID",
                        "Name",
                        "Manager",
                        "Start",
                        "Deadline",
                        "Members",
                        "Actions",
                    ] {
                        ui.strong(header);
                    }
                    ui.end_row();

                    for project in &projects {
                        ui.label(project.id.map(|id| id.to_string()).unwrap_or_default());
                        ui.label(&project.name);
                        ui.label(project.manager.id.to_string());
                        ui.label(project.start_time.clone().unwrap_or_default());
                        ui.label(project.deadline.clone().unwrap_or_default());
                        ui.label(project.members.id.to_string());
                        ui.horizontal(|ui| {
                            if ui.small_button("Tasks").clicked() {
                                if let Some(id) = project.id {
                                    self.fetch_project_tasks(id, project.name.clone());
                                }
                            }
                            if ui.small_button("Delete").clicked() {
                                if let Some(id) = project.id {
                                    self.spawn_delete(
                                        Req::Mutation("Project deleted"),
                                        format!("/project/{id}"),
                                    );
                                }
                            }
                        });
                        ui.end_row();
                    }
                });
        });

        if !self.project_tasks_title.is_empty() {
            ui.separator();
            ui.heading(format!("Tasks of {}", self.project_tasks_title));
            let tasks = self.project_tasks.clone();
            render_tasks_grid(ui, "project-tasks-table", &tasks);
        }
    }

    fn tasks_ui(&mut self, ui: &mut Ui) {
        ui.heading("Tasks");
        ui.horizontal(|ui| {
            ui.label(format!("{} loaded", self.tasks.len()));
            if ui.button("Refresh tasks").clicked() {
                self.spawn_get(Req::Tasks, "/task?limit=100&offset=0".to_string());
            }
        });

        egui::CollapsingHeader::new("Create task")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("task-form")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Project id");
                        ui.text_edit_singleline(&mut self.task_form.project_id);
                        ui.end_row();
                        ui.label("Name");
                        ui.text_edit_singleline(&mut self.task_form.name);
                        ui.end_row();
                        ui.label("Description");
                        ui.text_edit_multiline(&mut self.task_form.description);
                        ui.end_row();
                        ui.label("Deadline (RFC3339)");
                        ui.text_edit_singleline(&mut self.task_form.deadline);
                        ui.end_row();
                        ui.label("Status id");
                        ui.text_edit_singleline(&mut self.task_form.status_id);
                        ui.end_row();
                        ui.label("Assignee id");
                        ui.text_edit_singleline(&mut self.task_form.assignee_id);
                        ui.end_row();
                        ui.label("Estimated time (minutes)");
                        ui.text_edit_singleline(&mut self.task_form.estimated_time);
                        ui.end_row();
                    });
                if ui.button("Create task").clicked() {
                    self.create_task();
                }
            });

        ui.separator();
        let tasks = self.tasks.clone();
        render_tasks_grid(ui, "tasks-table", &tasks);
    }

    fn statuses_ui(&mut self, ui: &mut Ui) {
        ui.heading("Statuses");
        ui.horizontal(|ui| {
            ui.label(format!("{} loaded", self.statuses.len()));
            if ui.button("Refresh statuses").clicked() {
                self.spawn_get(Req::Statuses, "/reports/statuses".to_string());
            }
        });

        egui::CollapsingHeader::new("Create status")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("status-form")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut self.status_form.name);
                        ui.end_row();
                        ui.label("Color (integer)");
                        ui.text_edit_singleline(&mut self.status_form.color);
                        ui.end_row();
                        ui.label("Ready");
                        ui.checkbox(&mut self.status_form.ready, "");
                        ui.end_row();
                        ui.label("Pending");
                        ui.checkbox(&mut self.status_form.pending, "");
                        ui.end_row();
                    });
                if ui.button("Create status").clicked() {
                    self.create_status();
                }
            });

        ui.separator();
        let statuses = self.statuses.clone();
        egui::ScrollArea::horizontal().show(ui, |ui| {
            egui::Grid::new("statuses-table")
                .striped(true)
                .num_columns(6)
                .spacing([14.0, 4.0])
                .show(ui, |ui| {
                    for header in ["ID", "Name", "Color", "Ready", "Pending", "Actions"] {
                        ui.strong(header);
                    }
                    ui.end_row();
                    for status in &statuses {
                        ui.label(status.id.map(|id| id.to_string()).unwrap_or_default());
                        ui.label(&status.name);
                        ui.label(status.color.to_string());
                        ui.label(status.ready.to_string());
                        ui.label(status.pending.to_string());
                        if ui.small_button("Delete").clicked() {
                            if let Some(id) = status.id {
                                self.spawn_delete(
                                    Req::Mutation("Status deleted"),
                                    format!("/status/{id}"),
                                );
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    }

    fn categories_ui(&mut self, ui: &mut Ui) {
        ui.heading("Categories");
        ui.horizontal(|ui| {
            ui.label(format!("{} loaded", self.categories.len()));
            if ui.button("Refresh categories").clicked() {
                self.spawn_get(Req::Categories, "/category?limit=100&offset=0".to_string());
            }
        });

        egui::CollapsingHeader::new("Create category")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("category-form")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Name");
                        ui.text_edit_singleline(&mut self.category_form.name);
                        ui.end_row();
                        ui.label("Badge color (integer)");
                        ui.text_edit_singleline(&mut self.category_form.badge_color);
                        ui.end_row();
                        ui.label("Urgency");
                        ui.text_edit_singleline(&mut self.category_form.urgency);
                        ui.end_row();
                    });
                if ui.button("Create category").clicked() {
                    self.create_category();
                }
            });

        ui.separator();
        let categories = self.categories.clone();
        egui::ScrollArea::horizontal().show(ui, |ui| {
            egui::Grid::new("categories-table")
                .striped(true)
                .num_columns(5)
                .spacing([14.0, 4.0])
                .show(ui, |ui| {
                    for header in ["ID", "Name", "Badge color", "Urgency", "Actions"] {
                        ui.strong(header);
                    }
                    ui.end_row();
                    for category in &categories {
                        ui.label(category.id.map(|id| id.to_string()).unwrap_or_default());
                        ui.label(&category.name);
                        ui.label(category.badge_color.to_string());
                        ui.label(category.urgency.to_string());
                        if ui.small_button("Delete").clicked() {
                            if let Some(id) = category.id {
                                self.spawn_delete(
                                    Req::Mutation("Category deleted"),
                                    format!("/category/{id}"),
                                );
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    }

    fn users_ui(&mut self, ui: &mut Ui) {
        ui.heading("Users");
        ui.horizontal(|ui| {
            ui.label(format!("{} loaded", self.users.len()));
            if ui.button("Refresh users").clicked() {
                self.spawn_get(Req::Users, "/user?limit=100&offset=0".to_string());
            }
        });

        if self.current_user.is_some() {
            egui::CollapsingHeader::new("Update my profile")
                .default_open(false)
                .show(ui, |ui| {
                    egui::Grid::new("profile-form")
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("First name");
                            ui.text_edit_singleline(&mut self.profile_form.first_name);
                            ui.end_row();
                            ui.label("Second name");
                            ui.text_edit_singleline(&mut self.profile_form.second_name);
                            ui.end_row();
                            ui.label("Bio");
                            ui.text_edit_multiline(&mut self.profile_form.bio);
                            ui.end_row();
                        });
                    if ui.button("Update profile").clicked() {
                        self.update_profile();
                    }
                });
        }

        ui.separator();
        let users = self.users.clone();
        egui::ScrollArea::horizontal().show(ui, |ui| {
            egui::Grid::new("users-table")
                .striped(true)
                .num_columns(6)
                .spacing([14.0, 4.0])
                .show(ui, |ui| {
                    for header in ["ID", "Username", "Email", "Name", "Bio", "Actions"] {
                        ui.strong(header);
                    }
                    ui.end_row();
                    for user in &users {
                        ui.label(user.id.map(|id| id.to_string()).unwrap_or_default());
                        ui.label(&user.username);
                        ui.label(&user.email);
                        ui.label(user.display_name());
                        ui.label(truncate(user.bio.as_deref().unwrap_or(""), 60));
                        if ui.small_button("Delete").clicked() {
                            if let Some(id) = user.id {
                                self.spawn_delete(
                                    Req::Mutation("User deleted"),
                                    format!("/user/{id}"),
                                );
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    }

    fn discussions_ui(&mut self, ui: &mut Ui) {
        ui.heading("Discussions");
        ui.horizontal(|ui| {
            ui.label("Task id");
            ui.add(
                egui::TextEdit::singleline(&mut self.discussion_form.task_id)
                    .desired_width(70.0),
            );
            if ui.button("Load thread").clicked() {
                self.load_discussions();
            }
            ui.label(format!("{} loaded", self.discussions.len()));
        });

        egui::CollapsingHeader::new("Create discussion post")
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("discussion-form")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Task id");
                        ui.text_edit_singleline(&mut self.discussion_form.task_id);
                        ui.end_row();
                        ui.label("Parent post id (optional)");
                        ui.text_edit_singleline(&mut self.discussion_form.parent_id);
                        ui.end_row();
                        ui.label("Title");
                        ui.text_edit_singleline(&mut self.discussion_form.title);
                        ui.end_row();
                        ui.label("Body");
                        ui.text_edit_multiline(&mut self.discussion_form.body);
                        ui.end_row();
                    });
                if ui.button("Create post").clicked() {
                    self.create_discussion();
                }
            });

        ui.separator();
        let posts = self.discussions.clone();
        egui::ScrollArea::horizontal().show(ui, |ui| {
            egui::Grid::new("discussions-table")
                .striped(true)
                .num_columns(7)
                .spacing([14.0, 4.0])
                .show(ui, |ui| {
                    for header in ["ID", "Task", "Author", "Title", "Body", "Parent", "Actions"] {
                        ui.strong(header);
                    }
                    ui.end_row();
                    for post in &posts {
                        ui.label(post.id.map(|id| id.to_string()).unwrap_or_default());
                        ui.label(post.task.id.to_string());
                        ui.label(post.author.id.to_string());
                        ui.label(&post.title);
                        ui.label(truncate(&post.body, 80));
                        ui.label(post.parent.id.to_string());
                        if ui.small_button("Delete").clicked() {
                            if let Some(id) = post.id {
                                self.spawn_delete(
                                    Req::Mutation("Discussion post deleted"),
                                    format!("/discussion/{id}"),
                                );
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    }

    fn reports_ui(&mut self, ui: &mut Ui) {
        ui.heading("Analytical reports");
        ui.label("Read-only queries from app/sql/complex");

        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("report-kind")
                .selected_text(self.report.kind.label())
                .width(280.0)
                .show_ui(ui, |ui| {
                    for kind in ReportKind::ALL {
                        if ui
                            .selectable_label(self.report.kind == kind, kind.label())
                            .clicked()
                        {
                            self.report.kind = kind;
                            self.report.result = None;
                            self.report.error = None;
                        }
                    }
                });
            if ui.button("Run report").clicked() {
                self.run_report();
            }
            if ui.button("Clear").clicked() {
                self.report.result = None;
                self.report.error = None;
            }
        });

        ui.separator();
        egui::Grid::new("report-params")
            .num_columns(4)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                match self.report.kind {
                    ReportKind::DeadlineRank => {
                        ui.label("Project id");
                        ui.text_edit_singleline(&mut self.report.project_id);
                        ui.end_row();
                    }
                    ReportKind::DeadlinesMissed => {
                        ui.label("User id");
                        ui.text_edit_singleline(&mut self.report.user_id);
                        ui.label("Limit");
                        ui.text_edit_singleline(&mut self.report.limit);
                        ui.end_row();
                        ui.label("Offset");
                        ui.text_edit_singleline(&mut self.report.offset);
                        ui.end_row();
                    }
                    ReportKind::DeadlinesNext => {
                        ui.label("User id");
                        ui.text_edit_singleline(&mut self.report.user_id);
                        ui.label("Limit");
                        ui.text_edit_singleline(&mut self.report.limit);
                        ui.end_row();
                    }
                    ReportKind::LeastBusy => {
                        ui.label("Project id (optional)");
                        ui.text_edit_singleline(&mut self.report.project_id);
                        ui.label("Max tasks");
                        ui.text_edit_singleline(&mut self.report.max_tasks);
                        ui.end_row();
                    }
                    ReportKind::TaskStatus => {
                        ui.label("Project id (optional)");
                        ui.text_edit_singleline(&mut self.report.project_id);
                        ui.label("User id (optional)");
                        ui.text_edit_singleline(&mut self.report.user_id);
                        ui.end_row();
                    }
                    ReportKind::TasksPerUrgency => {
                        ui.label("User id");
                        ui.text_edit_singleline(&mut self.report.user_id);
                        ui.end_row();
                    }
                    ReportKind::SearchProjects => {
                        ui.label("Query");
                        ui.text_edit_singleline(&mut self.report.query);
                        ui.label("Threshold");
                        ui.text_edit_singleline(&mut self.report.threshold);
                        ui.end_row();
                        ui.label("Limit");
                        ui.text_edit_singleline(&mut self.report.limit);
                        ui.label("Offset");
                        ui.text_edit_singleline(&mut self.report.offset);
                        ui.end_row();
                    }
                    ReportKind::SearchTasks => {
                        ui.label("Query");
                        ui.text_edit_singleline(&mut self.report.query);
                        ui.label("Threshold");
                        ui.text_edit_singleline(&mut self.report.threshold);
                        ui.end_row();
                    }
                    ReportKind::SearchUsers => {
                        ui.label("Query");
                        ui.text_edit_singleline(&mut self.report.query);
                        ui.label("Limit");
                        ui.text_edit_singleline(&mut self.report.limit);
                        ui.end_row();
                        ui.label("Offset");
                        ui.text_edit_singleline(&mut self.report.offset);
                        ui.end_row();
                    }
                }
            });

        ui.separator();
        if let Some(error) = &self.report.error {
            ui.colored_label(Color32::LIGHT_RED, error);
        } else if let Some(result) = &self.report.result {
            render_value_table(ui, "report-result", result);
        } else {
            ui.label("Run a report to see results.");
        }
    }
}

impl eframe::App for DesktopApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll();
        if self.token.is_some() {
            self.main_ui(ui);
        } else {
            self.login_ui(ui);
        }
        if self.pending > 0 {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
    }
}

fn parse_page<T: DeserializeOwned>(result: Result<Value, String>) -> Result<Vec<T>, String> {
    let value = result?;
    serde_json::from_value::<Page<T>>(value)
        .map(|page| page.items)
        .map_err(|error| format!("invalid list response: {error}"))
}

fn parse_i32(value: &str) -> Result<i32, String> {
    value
        .trim()
        .parse::<i32>()
        .map_err(|_| format!("expected an integer, got `{value}`"))
}

fn parse_datetime(value: &str) -> Result<DateTime<Utc>, String> {
    let value = value.trim();
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Ok(parsed.with_timezone(&Utc));
    }
    for format in ["%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(parsed) = NaiveDateTime::parse_from_str(value, format) {
            return Ok(Utc.from_utc_datetime(&parsed));
        }
    }
    if let Ok(parsed) = NaiveDate::parse_from_str(value, "%Y-%m-%d") {
        return Ok(Utc.from_utc_datetime(&parsed.and_hms_opt(0, 0, 0).unwrap()));
    }
    Err(format!(
        "expected RFC3339 or `YYYY-MM-DD HH:MM:SS`, got `{value}`"
    ))
}

fn format_input_datetime(value: DateTime<Utc>) -> String {
    value.format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn add_param(path: &mut String, key: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    let separator = if path.contains('?') { '&' } else { '?' };
    path.push(separator);
    path.push_str(key);
    path.push('=');
    path.push_str(&encode(value));
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        value.to_string()
    } else {
        let mut out = value.chars().take(max).collect::<String>();
        out.push('…');
        out
    }
}

fn render_tasks_grid(ui: &mut Ui, id: &str, tasks: &[Task]) {
    egui::ScrollArea::horizontal().show(ui, |ui| {
        egui::Grid::new(id)
            .striped(true)
            .num_columns(7)
            .spacing([14.0, 4.0])
            .show(ui, |ui| {
                for header in [
                    "ID",
                    "Name",
                    "Deadline",
                    "Status",
                    "Assignee",
                    "Minutes",
                    "Description",
                ] {
                    ui.strong(header);
                }
                ui.end_row();
                for task in tasks {
                    ui.label(task.id.map(|id| id.to_string()).unwrap_or_default());
                    ui.label(&task.name);
                    ui.label(&task.deadline);
                    ui.label(task.status.id.to_string());
                    ui.label(task.assignee.id.to_string());
                    ui.label(task.estimated_time.to_string());
                    ui.label(
                        task.description
                            .as_deref()
                            .map(|value| truncate(value, 40))
                            .unwrap_or_default(),
                    );
                    ui.end_row();
                }
            });
    });
}

fn render_value_table(ui: &mut Ui, id: &str, value: &Value) {
    match value {
        Value::Array(items) if items.is_empty() => {
            ui.label("No rows returned.");
        }
        Value::Array(items) if items.iter().all(Value::is_object) => {
            let keys = items
                .first()
                .and_then(Value::as_object)
                .map(|map| map.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default();

            egui::ScrollArea::both().show(ui, |ui| {
                egui::Grid::new(id)
                    .striped(true)
                    .num_columns(keys.len().max(1))
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        for key in &keys {
                            ui.strong(key);
                        }
                        ui.end_row();
                        for item in items {
                            for key in &keys {
                                let cell = item.get(key).unwrap_or(&Value::Null);
                                ui.label(truncate(&value_string(cell), 64));
                            }
                            ui.end_row();
                        }
                    });
            });
        }
        _ => {
            ui.monospace(
                serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()),
            );
        }
    }
}

fn value_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}
