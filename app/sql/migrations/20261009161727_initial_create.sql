CREATE DOMAIN Color AS integer CHECK (0 <= value
    AND value < (1 << 24));

CREATE TYPE ProjectRole AS enum (
    'observer',
    'manager',
    'worker'
);

CREATE TYPE AuditAction AS enum (
    'create',
    'update',
    'delete'
);

CREATE TABLE users (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    username varchar(32) NOT NULL UNIQUE CHECK (username ~ '^[0-9a-zA-Z]+$'),
    password_hash char(60) NOT NULL,
    email varchar(254) NOT NULL UNIQUE UNIQUE CHECK (email ~ '^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}$'),
    first_name varchar(256) NOT NULL,
    second_name varchar(256) NOT NULL,
    bio text
);

CREATE TABLE roles (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name varchar(32) UNIQUE NOT NULL
);

CREATE TABLE permissions (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name varchar(192) NOT NULL
);

CREATE TABLE role_permissions (
    role_id integer NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    permission_id integer NOT NULL REFERENCES permissions (id) ON DELETE CASCADE,
    PRIMARY KEY (role_id, permission_id)
);

CREATE TABLE user_roles (
    user_id integer NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    role_id integer NOT NULL REFERENCES roles (id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, role_id)
);

CREATE TABLE audit_log (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    event_timestamp timestamp NOT NULL,
    user_id integer REFERENCES users (id),
    table_name varchar(48) NOT NULL,
    entity_id integer NOT NULL,
    action AuditAction NOT NULL,
    description varchar(1024) NOT NULL
);

CREATE TABLE sessions (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    user_id integer NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    auth_token varchar(4096) NOT NULL
);

CREATE TABLE projects (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    manager_id integer NOT NULL REFERENCES users (id) ON DELETE RESTRICT,
    name varchar(64) NOT NULL,
    description varchar(512),
    start_time timestamp,
    deadline timestamp,
    CHECK (deadline > start_time)
);

CREATE INDEX ON projects (name);

CREATE TABLE project_members (
    user_id integer NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    project_id integer NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    project_role ProjectRole NOT NULL,
    PRIMARY KEY (user_id, project_id)
);

CREATE TABLE categories (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name varchar(24) NOT NULL,
    badge_color Color NOT NULL,
    urgency integer NOT NULL CHECK (urgency > 0)
);

CREATE TABLE statuses (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name varchar(24) NOT NULL,
    color Color UNIQUE NOT NULL,
    ready boolean NOT NULL DEFAULT FALSE,
    pending boolean NOT NULL DEFAULT FALSE
);

CREATE TABLE tasks (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    name varchar(64) NOT NULL,
    description varchar(1024) NOT NULL,
    deadline timestamp NOT NULL,
    status_id integer NOT NULL REFERENCES statuses (id) ON DELETE RESTRICT,
    project_id integer NOT NULL REFERENCES projects (id) ON DELETE CASCADE,
    assignee_id integer REFERENCES users (id) ON DELETE SET NULL,
    estimated_time integer NOT NULL
);

CREATE INDEX ON tasks (name);

CREATE TABLE tasks_dependencies (
    dependent_id integer NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    dependency_id integer NOT NULL REFERENCES tasks (id) ON DELETE SET NULL,
    PRIMARY KEY (dependent_id, dependency_id)
);

CREATE TABLE task_categories (
    task_id integer NOT NULL REFERENCES tasks (id) ON DELETE CASCADE,
    category_id integer NOT NULL REFERENCES categories (id) ON DELETE CASCADE,
    PRIMARY KEY (task_id, category_id)
);

CREATE TABLE discussion_posts (
    id integer GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    task_id integer REFERENCES tasks (id) ON DELETE CASCADE,
    author_id integer REFERENCES users (id) ON DELETE SET NULL,
    title varchar(48) NOT NULL,
    body varchar(512) NOT NULL,
    parent_id integer REFERENCES discussion_posts (id) ON DELETE CASCADE
);

-- TODO:
-- deadline span check
