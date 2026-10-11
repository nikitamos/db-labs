create domain Color as integer
not null
check(0 <= value AND value < (1<<24));

create type ProjectRole as enum ('observer', 'manager', 'worker');
create type AuditAction as enum ('create', 'update', 'delete');

create table users (
id integer generated always as identity primary key,
    username varchar(32) not null unique,
    password_hash char(60) not null,
    email varchar(254) not null unique,
    first_name varchar(256) not null,
    second_name varchar(256) not null,
    bio text
);

create table roles (
id integer generated always as identity primary key,
  name varchar(32) not null
);

create table permissions (
id integer generated always as identity primary key,
  name varchar(192) not null
);

create table role_permissions (
  role_id integer not null references roles(id),
  permission_id integer not null references permissions(id),
  primary key (role_id, permission_id)
);

create table user_roles (
  user_id integer not null references users(id),
  role_id integer references roles(id),
  primary key (user_id, role_id)
);

create table audit_log (
id integer generated always as identity primary key,
  event_timestamp timestamp not null,
  user_id integer references users(id),
  table_name varchar(48) not null,
  entity_id integer not null,
  action AuditAction not null,
  description varchar(1024) not null
);

create table sessions (
id integer generated always as identity primary key,
  user_id integer not null references users(id),
  auth_token varchar(4096) not null
);

create table projects (
id integer generated always as identity primary key,
    manager_id integer not null references users(id),
    name varchar(64) not null,
    description varchar(512),
    start_time timestamp,
    deadline timestamp
);

create table project_members (
    user_id integer not null references users(id),
    project_id integer not null references projects(id),
    project_role ProjectRole,
    primary key (user_id, project_id)
);


create table categories (
id integer generated always as identity primary key,
    name varchar(24) not null,
    badge_color Color,
    urgency integer check(urgency > 0)
);

create table statuses (
id integer generated always as identity primary key,
    name varchar(24) not null,
    color Color unique,
    ready boolean,
    pending boolean
);

create table tasks (
id integer generated always as identity primary key,
    name varchar(64) not null,
    description varchar(1024) not null,
    deadline timestamp not null,
    status_id integer not null references statuses(id),
    project_id integer not null references projects(id),
    assignee_id integer not null references users(id),
    estimated_time integer not null
);

create table tasks_dependencies (
    dependent_id integer not null references tasks(id),
    dependency_id integer not null references tasks(id),
    primary key (dependent_id, dependency_id)
);

create table task_categories (
    task_id integer not null references tasks(id),
    category_id integer not null references categories(id),
    primary key (task_id, category_id)
);

create table discussion_posts (
id integer generated always as identity primary key,
    task_id integer references tasks(id),
    author_id integer references users(id),
    title varchar(48) not null,
    body varchar(512) not null,
    parent_id integer references discussion_posts(id)
);
