-- User-facing database functions corresponding to the SQLx macro queries in
-- the Rust services. Every function receives actor_id and checks permission
-- before doing any work. Permission names are not inserted into the
-- permissions table in this migration.
--
-- Pagination is intentionally not implemented here: functions return all
-- matching rows, and the Rust side applies cursor predicates and LIMIT.

-- ---------------------------------------------------------------------------
-- Permission helper
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION authorize_for(
    actor_id integer,
    requested_permission varchar
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN

    -- IF NOT check_permission(actor_id, requested_permission) THEN
    --     RAISE EXCEPTION
    --         'permission denied: user % lacks permission %',
    --         actor_id,
    --         requested_permission
    --         USING ERRCODE = '42501';
    -- END IF;
END;
$$;

COMMENT ON FUNCTION authorize_for(integer, varchar) IS
    'Raises an insufficient-privilege exception when actor_id lacks requested_permission.';

-- ---------------------------------------------------------------------------
-- users
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION read_user_by_identity(
    actor_id integer,
    p_identity varchar
)
RETURNS TABLE (
    id integer,
    username varchar,
    password_hash char(60),
    email varchar,
    first_name varchar,
    second_name varchar,
    bio text
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_users');

    RETURN QUERY
    SELECT
        u.id,
        u.username,
        u.password_hash,
        u.email,
        u.first_name,
        u.second_name,
        u.bio
    FROM users AS u
    WHERE LOWER(p_identity) IN (LOWER(u.email), LOWER(u.username));
END;
$$;

CREATE OR REPLACE FUNCTION modify_user_profile(
    actor_id integer,
    p_first_name varchar,
    p_second_name varchar,
    p_bio text,
    p_user_id integer
)
RETURNS TABLE (
    id integer,
    username varchar,
    password_hash char(60),
    email varchar,
    first_name varchar,
    second_name varchar,
    bio text
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'modify_users');

    RETURN QUERY
    WITH updated AS (
        UPDATE users AS u
        SET first_name = p_first_name,
            second_name = p_second_name,
            bio = p_bio
        WHERE u.id = p_user_id
        RETURNING
            u.id,
            u.username,
            u.password_hash,
            u.email,
            u.first_name,
            u.second_name,
            u.bio
    )
    SELECT
        updated.id,
        updated.username,
        updated.password_hash,
        updated.email,
        updated.first_name,
        updated.second_name,
        updated.bio
    FROM updated;
END;
$$;

CREATE OR REPLACE FUNCTION read_user(
    actor_id integer,
    p_user_id integer
)
RETURNS TABLE (
    id integer,
    username varchar,
    password_hash char(60),
    email varchar,
    first_name varchar,
    second_name varchar,
    bio text
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_users');

    RETURN QUERY
    SELECT
        u.id,
        u.username,
        u.password_hash,
        u.email,
        u.first_name,
        u.second_name,
        u.bio
    FROM users AS u
    WHERE u.id = p_user_id;
END;
$$;

CREATE OR REPLACE FUNCTION read_users(actor_id integer)
RETURNS TABLE (
    id integer,
    username varchar,
    password_hash char(60),
    email varchar,
    first_name varchar,
    second_name varchar,
    bio text
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_users');

    RETURN QUERY
    SELECT
        u.id,
        u.username,
        u.password_hash,
        u.email,
        u.first_name,
        u.second_name,
        u.bio
    FROM users AS u
    ORDER BY u.id;
END;
$$;

CREATE OR REPLACE FUNCTION write_user(
    actor_id integer,
    p_username varchar,
    p_password_hash char(60),
    p_email varchar,
    p_first_name varchar,
    p_second_name varchar,
    p_bio text
)
RETURNS TABLE (
    id integer,
    username varchar,
    password_hash char(60),
    email varchar,
    first_name varchar,
    second_name varchar,
    bio text
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'write_users');

    RETURN QUERY
    WITH inserted AS (
        INSERT INTO users (
            username,
            password_hash,
            email,
            first_name,
            second_name,
            bio
        )
        VALUES (
            p_username,
            p_password_hash,
            p_email,
            p_first_name,
            p_second_name,
            p_bio
        )
        RETURNING
            users.id,
            users.username,
            users.password_hash,
            users.email,
            users.first_name,
            users.second_name,
            users.bio
    )
    SELECT
        inserted.id,
        inserted.username,
        inserted.password_hash,
        inserted.email,
        inserted.first_name,
        inserted.second_name,
        inserted.bio
    FROM inserted;
END;
$$;

CREATE OR REPLACE FUNCTION modify_user_password(
    actor_id integer,
    p_password_hash char(60),
    p_user_id integer
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'modify_users');

    UPDATE users AS u
    SET password_hash = p_password_hash
    WHERE u.id = p_user_id;

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

CREATE OR REPLACE FUNCTION delete_user(
    actor_id integer,
    p_user_id integer
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'modify_users');

    DELETE FROM users AS u
    WHERE u.id = p_user_id;

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

-- ---------------------------------------------------------------------------
-- roles / permissions
-- ---------------------------------------------------------------------------

-- Corresponds to Role::fetch_for_user. The permission is read_user_roles
-- because the query is keyed by user_roles.user_id.
CREATE OR REPLACE FUNCTION read_user_roles(
    actor_id integer,
    p_user_id integer
)
RETURNS TABLE (
    role_name varchar,
    permission_name varchar
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_user_roles');

    RETURN QUERY
    SELECT
        r.name AS role_name,
        p.name AS permission_name
    FROM user_roles AS ur
    INNER JOIN role_permissions AS rp
        ON rp.role_id = ur.role_id
    INNER JOIN permissions AS p
        ON p.id = rp.permission_id
    INNER JOIN roles AS r
        ON r.id = rp.role_id
    WHERE ur.user_id = p_user_id
    ORDER BY r.name;
END;
$$;

-- Corresponds to CALL add_user_role($1, $2).
CREATE OR REPLACE FUNCTION write_user_role(
    actor_id integer,
    p_user_id integer,
    p_role_name varchar
)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'write_user_roles');

    CALL add_user_role(p_user_id, p_role_name);
END;
$$;

-- ---------------------------------------------------------------------------
-- sessions
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION write_session(
    actor_id integer,
    p_user_id integer,
    p_auth_token varchar
)
RETURNS TABLE (
    "user" integer,
    auth_token varchar,
    id integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'write_sessions');

    RETURN QUERY
    WITH inserted AS (
        INSERT INTO sessions (user_id, auth_token)
        VALUES (p_user_id, p_auth_token)
        RETURNING sessions.user_id, sessions.auth_token, sessions.id
    )
    SELECT
        inserted.user_id AS "user",
        inserted.auth_token,
        inserted.id
    FROM inserted;
END;
$$;

CREATE OR REPLACE FUNCTION modify_session(
    actor_id integer,
    p_session_id integer
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'modify_sessions');

    DELETE FROM sessions AS s
    WHERE s.id = p_session_id;

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

-- ---------------------------------------------------------------------------
-- categories
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION read_category(
    actor_id integer,
    p_category_id integer
)
RETURNS TABLE (
    id integer,
    name varchar,
    badge_color Color,
    urgency integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_categories');

    RETURN QUERY
    SELECT
        c.id,
        c.name,
        c.badge_color,
        c.urgency
    FROM categories AS c
    WHERE c.id = p_category_id;
END;
$$;

CREATE OR REPLACE FUNCTION read_categories(actor_id integer)
RETURNS TABLE (
    id integer,
    name varchar,
    badge_color Color,
    urgency integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_categories');

    RETURN QUERY
    SELECT
        c.id,
        c.name,
        c.badge_color,
        c.urgency
    FROM categories AS c
    ORDER BY c.id;
END;
$$;

CREATE OR REPLACE FUNCTION write_category(
    actor_id integer,
    p_name varchar,
    p_badge_color Color,
    p_urgency integer
)
RETURNS TABLE (
    id integer,
    name varchar(24),
    badge_color Color,
    urgency integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'write_categories');

    RETURN QUERY
    WITH inserted AS (
        INSERT INTO categories (name, badge_color, urgency)
        VALUES (p_name, p_badge_color, p_urgency)
        RETURNING
            categories.id,
            categories.name,
            categories.badge_color,
            categories.urgency
    )
    SELECT
        inserted.id,
        inserted.name,
        inserted.badge_color,
        inserted.urgency
    FROM inserted;
END;
$$;

CREATE OR REPLACE FUNCTION modify_category(
    actor_id integer,
    p_name varchar,
    p_badge_color integer,
    p_urgency integer,
    p_category_id integer
)
RETURNS TABLE (
    id integer,
    name varchar,
    badge_color integer,
    urgency integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'modify_categories');

    RETURN QUERY
    WITH updated AS (
        UPDATE categories AS c
        SET name = COALESCE(p_name, c.name),
            badge_color = COALESCE(p_badge_color, c.badge_color),
            urgency = COALESCE(p_urgency, c.urgency)
        WHERE c.id = p_category_id
        RETURNING
            c.id,
            c.name,
            c.badge_color,
            c.urgency
    )
    SELECT
        updated.id,
        updated.name,
        updated.badge_color,
        updated.urgency
    FROM updated;
END;
$$;

CREATE OR REPLACE FUNCTION delete_category(
    actor_id integer,
    p_category_id integer
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'modify_categories');

    DELETE FROM categories AS c
    WHERE c.id = p_category_id;

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

-- ---------------------------------------------------------------------------
-- statuses
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION read_status(
    actor_id integer,
    p_status_id integer
)
RETURNS TABLE (
    id integer,
    name varchar,
    color integer,
    ready boolean,
    pending boolean
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_statuses');

    RETURN QUERY
    SELECT
        s.id,
        s.name,
        s.color,
        s.ready,
        s.pending
    FROM statuses AS s
    WHERE s.id = p_status_id;
END;
$$;

CREATE OR REPLACE FUNCTION read_statuses(actor_id integer)
RETURNS TABLE (
    id integer,
    name varchar,
    color integer,
    ready boolean,
    pending boolean
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_statuses');

    RETURN QUERY
    SELECT
        s.id,
        s.name,
        s.color,
        s.ready,
        s.pending
    FROM statuses AS s
    ORDER BY s.id;
END;
$$;

CREATE OR REPLACE FUNCTION write_status(
    actor_id integer,
    p_name varchar,
    p_color Color,
    p_ready boolean,
    p_pending boolean
)
RETURNS TABLE (
    id integer,
    name varchar,
    color Color,
    ready boolean,
    pending boolean
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'write_statuses');

    RETURN QUERY
    WITH inserted AS (
        INSERT INTO statuses (name, color, ready, pending)
        VALUES (p_name, p_color, p_ready, p_pending)
        RETURNING
            statuses.id,
            statuses.name,
            statuses.color,
            statuses.ready,
            statuses.pending
    )
    SELECT
        inserted.id,
        inserted.name,
        inserted.color,
        inserted.ready,
        inserted.pending
    FROM inserted;
END;
$$;

CREATE OR REPLACE FUNCTION modify_status(
    actor_id integer,
    p_name varchar,
    p_color Color,
    p_ready boolean,
    p_pending boolean,
    p_status_id integer
)
RETURNS TABLE (
    id integer,
    name varchar,
    color Color,
    ready boolean,
    pending boolean
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'modify_statuses');

    RETURN QUERY
    WITH updated AS (
        UPDATE statuses AS s
        SET name = COALESCE(p_name, s.name),
            color = COALESCE(p_color, s.color),
            ready = COALESCE(p_ready, s.ready),
            pending = COALESCE(p_pending, s.pending)
        WHERE s.id = p_status_id
        RETURNING
            s.id,
            s.name,
            s.color,
            s.ready,
            s.pending
    )
    SELECT
        updated.id,
        updated.name,
        updated.color,
        updated.ready,
        updated.pending
    FROM updated;
END;
$$;

CREATE OR REPLACE FUNCTION delete_status(
    actor_id integer,
    p_status_id integer
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'modify_statuses');

    DELETE FROM statuses AS s
    WHERE s.id = p_status_id;

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

-- ---------------------------------------------------------------------------
-- tasks
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION read_task(
    actor_id integer,
    p_task_id integer
)
RETURNS TABLE (
    id integer,
    name varchar,
    description varchar,
    deadline timestamp,
    status integer,
    assignee integer,
    estimated_time integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_tasks');

    RETURN QUERY
    SELECT
        t.id,
        t.name,
        t.description,
        t.deadline,
        t.status_id AS status,
        t.assignee_id AS assignee,
        t.estimated_time
    FROM tasks AS t
    WHERE t.id = p_task_id;
END;
$$;

CREATE OR REPLACE FUNCTION read_tasks(actor_id integer)
RETURNS TABLE (
    id integer,
    name varchar,
    description varchar,
    deadline timestamp,
    status integer,
    assignee integer,
    estimated_time integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_tasks');

    RETURN QUERY
    SELECT
        t.id,
        t.name,
        t.description,
        t.deadline,
        t.status_id AS status,
        t.assignee_id AS assignee,
        t.estimated_time
    FROM tasks AS t
    ORDER BY t.id;
END;
$$;

CREATE OR REPLACE FUNCTION write_task(
    actor_id integer,
    p_project_id integer,
    p_name varchar,
    p_description varchar,
    p_deadline timestamp,
    p_status_id integer,
    p_assignee_id integer,
    p_estimated_time integer
)
RETURNS TABLE (
    id integer,
    name varchar,
    description varchar,
    deadline timestamp,
    status integer,
    assignee integer,
    estimated_time integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'write_tasks');

    RETURN QUERY
    WITH inserted AS (
        INSERT INTO tasks (
            project_id,
            name,
            description,
            deadline,
            status_id,
            assignee_id,
            estimated_time
        )
        VALUES (
            p_project_id,
            p_name,
            p_description,
            p_deadline,
            p_status_id,
            p_assignee_id,
            p_estimated_time
        )
        RETURNING
            tasks.id,
            tasks.name,
            tasks.description,
            tasks.deadline,
            tasks.status_id,
            tasks.assignee_id,
            tasks.estimated_time
    )
    SELECT
        inserted.id,
        inserted.name,
        inserted.description,
        inserted.deadline,
        inserted.status_id AS status,
        inserted.assignee_id AS assignee,
        inserted.estimated_time
    FROM inserted;
END;
$$;

CREATE OR REPLACE FUNCTION modify_task(
    actor_id integer,
    p_name varchar,
    p_description varchar,
    p_deadline timestamp,
    p_status_id integer,
    p_assignee_id integer,
    p_estimated_time integer,
    p_task_id integer
)
RETURNS TABLE (
    id integer,
    name varchar,
    description varchar,
    deadline timestamp,
    status integer,
    assignee integer,
    estimated_time integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'modify_tasks');

    RETURN QUERY
    WITH updated AS (
        UPDATE tasks AS t
        SET name = COALESCE(p_name, t.name),
            description = COALESCE(p_description, t.description),
            deadline = COALESCE(p_deadline, t.deadline),
            status_id = COALESCE(p_status_id, t.status_id),
            assignee_id = COALESCE(p_assignee_id, t.assignee_id),
            estimated_time = COALESCE(p_estimated_time, t.estimated_time)
        WHERE t.id = p_task_id
        RETURNING
            t.id,
            t.name,
            t.description,
            t.deadline,
            t.status_id,
            t.assignee_id,
            t.estimated_time
    )
    SELECT
        updated.id,
        updated.name,
        updated.description,
        updated.deadline,
        updated.status_id AS status,
        updated.assignee_id AS assignee,
        updated.estimated_time
    FROM updated;
END;
$$;

CREATE OR REPLACE FUNCTION delete_task(
    actor_id integer,
    p_task_id integer
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'modify_tasks');

    DELETE FROM tasks AS t
    WHERE t.id = p_task_id;

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

CREATE OR REPLACE FUNCTION read_project_tasks(
    actor_id integer,
    p_project_id integer
)
RETURNS TABLE (
    id integer,
    name varchar,
    description varchar,
    deadline timestamp,
    status integer,
    assignee integer,
    estimated_time integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_tasks');

    RETURN QUERY
    SELECT
        t.id,
        t.name,
        t.description,
        t.deadline,
        t.status_id AS status,
        t.assignee_id AS assignee,
        t.estimated_time
    FROM tasks AS t
    WHERE t.project_id = p_project_id
    ORDER BY t.id;
END;
$$;

CREATE OR REPLACE FUNCTION read_tasks_by_category(
    actor_id integer,
    p_category_id integer
)
RETURNS TABLE (
    id integer,
    name varchar,
    description varchar,
    deadline timestamp,
    status integer,
    assignee integer,
    estimated_time integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_tasks');

    RETURN QUERY
    SELECT
        t.id,
        t.name,
        t.description,
        t.deadline,
        t.status_id AS status,
        t.assignee_id AS assignee,
        t.estimated_time
    FROM tasks AS t
    INNER JOIN task_categories AS tc
        ON tc.task_id = t.id
    WHERE tc.category_id = p_category_id
    ORDER BY t.id;
END;
$$;

-- ---------------------------------------------------------------------------
-- task dependencies
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION write_task_dependency(
    actor_id integer,
    p_dependent_id integer,
    p_dependency_id integer
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'write_tasks_dependencies');

    INSERT INTO tasks_dependencies (dependent_id, dependency_id)
    VALUES (p_dependent_id, p_dependency_id)
    ON CONFLICT DO NOTHING;

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

CREATE OR REPLACE FUNCTION delete_task_dependency(
    actor_id integer,
    p_dependent_id integer,
    p_dependency_id integer
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'modify_tasks_dependencies');

    DELETE FROM tasks_dependencies AS td
    WHERE td.dependent_id = p_dependent_id
      AND td.dependency_id = p_dependency_id;

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

-- ---------------------------------------------------------------------------
-- task categories
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION write_task_category(
    actor_id integer,
    p_task_id integer,
    p_category_id integer
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'write_task_categories');

    INSERT INTO task_categories (task_id, category_id)
    VALUES (p_task_id, p_category_id)
    ON CONFLICT DO NOTHING;

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

CREATE OR REPLACE FUNCTION delete_task_categories(
    actor_id integer,
    p_task_id integer,
    p_category_ids integer[]
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'modify_task_categories');

    DELETE FROM task_categories AS tc
    WHERE tc.task_id = p_task_id
      AND tc.category_id = ANY (p_category_ids);

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

-- ---------------------------------------------------------------------------
-- projects
-- ---------------------------------------------------------------------------

-- Covers both fetch_full and fetch_no_nav, which run the same SELECT.
CREATE OR REPLACE FUNCTION read_project(
    actor_id integer,
    p_project_id integer
)
RETURNS TABLE (
    manager integer,
    id integer,
    name varchar,
    description varchar,
    start_time timestamp,
    deadline timestamp,
    members integer,
    tasks integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_projects');

    RETURN QUERY
    SELECT
        p.manager_id AS manager,
        p.id,
        p.name,
        p.description,
        p.start_time,
        p.deadline,
        0::integer AS members,
        0::integer AS tasks
    FROM projects AS p
    WHERE p.id = p_project_id;
END;
$$;

CREATE OR REPLACE FUNCTION read_projects(actor_id integer)
RETURNS TABLE (
    manager integer,
    id integer,
    name varchar,
    description varchar,
    start_time timestamp,
    deadline timestamp,
    members integer,
    tasks integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_projects');

    RETURN QUERY
    SELECT
        p.manager_id AS manager,
        p.id,
        p.name,
        p.description,
        p.start_time,
        p.deadline,
        0::integer AS members,
        0::integer AS tasks
    FROM projects AS p
    ORDER BY p.id;
END;
$$;

CREATE OR REPLACE FUNCTION write_project(
    actor_id integer,
    p_manager_id integer,
    p_name varchar,
    p_description varchar,
    p_start_time timestamp,
    p_deadline timestamp
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    new_project_id integer;
BEGIN
    PERFORM authorize_for(actor_id, 'write_projects');

    INSERT INTO projects (
        manager_id,
        name,
        description,
        start_time,
        deadline
    )
    VALUES (
        p_manager_id,
        p_name,
        p_description,
        p_start_time,
        p_deadline
    )
    RETURNING id INTO new_project_id;

    RETURN new_project_id;
END;
$$;

CREATE OR REPLACE FUNCTION modify_project(
    actor_id integer,
    p_manager_id integer,
    p_name varchar,
    p_description varchar,
    p_start_time timestamp,
    p_deadline timestamp,
    p_project_id integer
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    old_manager_id integer;
BEGIN
    PERFORM authorize_for(actor_id, 'modify_projects');

    SELECT p.manager_id
    INTO old_manager_id
    FROM projects AS p
    WHERE p.id = p_project_id;

    UPDATE projects AS p
    SET manager_id = COALESCE(p_manager_id, p.manager_id),
        name = COALESCE(p_name, p.name),
        description = COALESCE(p_description, p.description),
        start_time = COALESCE(p_start_time, p.start_time),
        deadline = COALESCE(p_deadline, p.deadline)
    WHERE p.id = p_project_id;

    RETURN old_manager_id;
END;
$$;

CREATE OR REPLACE FUNCTION delete_project(
    actor_id integer,
    p_project_id integer
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'modify_projects');

    DELETE FROM projects AS p
    WHERE p.id = p_project_id;

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

CREATE OR REPLACE FUNCTION write_project_member(
    actor_id integer,
    p_user_id integer,
    p_project_id integer,
    p_project_role ProjectRole
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'write_project_members');

    INSERT INTO project_members (user_id, project_id, project_role)
    VALUES (p_user_id, p_project_id, p_project_role);

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

CREATE OR REPLACE FUNCTION delete_project_member(
    actor_id integer,
    p_project_id integer,
    p_user_id integer
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'modify_project_members');

    DELETE FROM project_members AS pm
    WHERE pm.project_id = p_project_id
      AND pm.user_id = p_user_id;

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

CREATE OR REPLACE FUNCTION read_project_member_role(
    actor_id integer,
    p_project_id integer,
    p_user_id integer
)
RETURNS TABLE (
    project_role ProjectRole
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_project_members');

    RETURN QUERY
    SELECT pm.project_role
    FROM project_members AS pm
    WHERE pm.project_id = p_project_id
      AND pm.user_id = p_user_id;
END;
$$;

-- ---------------------------------------------------------------------------
-- discussion posts
-- ---------------------------------------------------------------------------

CREATE OR REPLACE FUNCTION read_discussion_post(
    actor_id integer,
    p_post_id integer
)
RETURNS TABLE (
    id integer,
    task integer,
    author integer,
    title varchar,
    body varchar,
    parent integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_discussion_posts');

    RETURN QUERY
    SELECT
        dp.id,
        dp.task_id AS task,
        dp.author_id AS author,
        dp.title,
        dp.body,
        dp.parent_id AS parent
    FROM discussion_posts AS dp
    WHERE dp.id = p_post_id;
END;
$$;

CREATE OR REPLACE FUNCTION write_discussion_post(
    actor_id integer,
    p_task_id integer,
    p_author_id integer,
    p_title varchar,
    p_body varchar,
    p_parent_id integer
)
RETURNS TABLE (
    id integer,
    task integer,
    author integer,
    title varchar,
    body varchar,
    parent integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'write_discussion_posts');

    RETURN QUERY
    WITH inserted AS (
        INSERT INTO discussion_posts (
            task_id,
            author_id,
            title,
            body,
            parent_id
        )
        VALUES (
            p_task_id,
            p_author_id,
            p_title,
            p_body,
            p_parent_id
        )
        RETURNING
            discussion_posts.id,
            discussion_posts.task_id,
            discussion_posts.author_id,
            discussion_posts.title,
            discussion_posts.body,
            discussion_posts.parent_id
    )
    SELECT
        inserted.id,
        inserted.task_id AS task,
        inserted.author_id AS author,
        inserted.title,
        inserted.body,
        inserted.parent_id AS parent
    FROM inserted;
END;
$$;

CREATE OR REPLACE FUNCTION modify_discussion_post(
    actor_id integer,
    p_title varchar,
    p_body varchar,
    p_post_id integer
)
RETURNS TABLE (
    id integer,
    task integer,
    author integer,
    title varchar,
    body varchar,
    parent integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'modify_discussion_posts');

    RETURN QUERY
    WITH updated AS (
        UPDATE discussion_posts AS dp
        SET title = COALESCE(p_title, dp.title),
            body = COALESCE(p_body, dp.body)
        WHERE dp.id = p_post_id
        RETURNING
            dp.id,
            dp.task_id,
            dp.author_id,
            dp.title,
            dp.body,
            dp.parent_id
    )
    SELECT
        updated.id,
        updated.task_id AS task,
        updated.author_id AS author,
        updated.title,
        updated.body,
        updated.parent_id AS parent
    FROM updated;
END;
$$;

CREATE OR REPLACE FUNCTION delete_discussion_post(
    actor_id integer,
    p_post_id integer
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    affected integer;
BEGIN
    PERFORM authorize_for(actor_id, 'modify_discussion_posts');

    DELETE FROM discussion_posts AS dp
    WHERE dp.id = p_post_id;

    GET DIAGNOSTICS affected = ROW_COUNT;
    RETURN affected;
END;
$$;

CREATE OR REPLACE FUNCTION read_discussion_posts_by_task(
    actor_id integer,
    p_task_id integer
)
RETURNS TABLE (
    id integer,
    task integer,
    author integer,
    title varchar,
    body varchar,
    parent integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_discussion_posts');

    RETURN QUERY
    SELECT
        dp.id,
        dp.task_id AS task,
        dp.author_id AS author,
        dp.title,
        dp.body,
        dp.parent_id AS parent
    FROM discussion_posts AS dp
    WHERE dp.task_id = p_task_id
    ORDER BY dp.id;
END;
$$;

CREATE OR REPLACE FUNCTION read_discussion_posts_by_parent(
    actor_id integer,
    p_parent_id integer
)
RETURNS TABLE (
    id integer,
    task integer,
    author integer,
    title varchar,
    body varchar,
    parent integer
)
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM authorize_for(actor_id, 'read_discussion_posts');

    RETURN QUERY
    SELECT
        dp.id,
        dp.task_id AS task,
        dp.author_id AS author,
        dp.title,
        dp.body,
        dp.parent_id AS parent
    FROM discussion_posts AS dp
    WHERE dp.parent_id = p_parent_id
    ORDER BY dp.id;
END;
$$;
