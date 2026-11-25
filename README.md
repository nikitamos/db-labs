# Data Models & Database Management Systems
Project for the first semester of the database course (work-in-progress).

| | |
|-|-|
|**Domain** | Project Management System (like GitHub Projects) |
|**Author** | Nikita Moskaliov, 453501                         |
|**Stack**  | Rust, Axum, Postgres, SQLx                       |

This repository contains backend for project tracking system.
In such system each project consists of tasks, and tasks may
depend on another tasks. Different persons may be responsible
for different tasks. Tasks within a project may depend
on each other. Each task has a status, like on a Kanban board.

## Build and run instructions
### Prerequisites
The following software is required to build this system:
* Rust 1.98 or newer
* Docker or Podman
* .NET Aspire (https://aspire.dev/)
* SQLx CLI (https://crates.io/crates/sqlx-cli)

### Build and run
First, run the aspire in the root directory:
```
aspire start
```
Aspire will start the database and attempt to build
the project, but the build will fail because the
database is not migrated.

In order to migrate the database and restart the application,
do the following:
1. Navigate to the Aspire dashboard and copy the database connection
   string from the `APP_DB_URI` environment variable of the `backend` service
2. In the `app` directory execute the command:
   ```
   APP_DB_URI=<copied connection string> sqlx migrate run
   ```
3. Restart the `backend` service from the aspire dashboard.

After that, the system will be available at `http://localhost:6200/`.

OpenAPI docs are hosted at `http://localhost:6200/openapi.json`,
and Scalar UI is hosted at `http://localhost:6200/docs`.

