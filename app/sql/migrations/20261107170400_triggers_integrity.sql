CREATE OR REPLACE VIEW user_permissions AS
SELECT roles.name as role_name,
       permissions.name as permission_name
FROM user_roles
INNER JOIN role_permissions ON role_permissions.role_id=user_roles.role_id
INNER JOIN permissions ON permissions.id=role_permissions.permission_id
INNER JOIN roles ON roles.id=role_permissions.role_id
ORDER BY roles.name;

-- COMMON FUNCTIONS --

CREATE OR REPLACE FUNCTION check_permission(user_id int, permission_name VARCHAR) RETURNS BOOLEAN AS $$
BEGIN

PERFORM 1 FROM user_permissions p
WHERE user_id = p.user_id AND p.permission_name = permission_name;
RETURN FOUND;

END;
$$ LANGUAGE plpgsql;

-- TRIGGERS --
 --

CREATE OR REPLACE FUNCTION add_manager_to_project() RETURNS trigger AS $$
BEGIN
    -- Adds manager to project members. Verifies that the manager has
    -- the Manager global role

    -- TODO: authorize

    IF NOT check_permission(NEW.manager_id, "manage_projects") THEN
        RAISE EXCEPTION 'insufficient manager permission';
        RETURN OLD; -- is it required?
    END IF;


    DELETE FROM project_members pm
    WHERE pm.project_id = NEW.project_id AND pm.user_id = NEW.manager_id;

    INSERT INTO project_members (project_id, user_id, project_role)
    VALUES (NEW.project_id, NEW.manager_id, 'manager');

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


CREATE OR REPLACE TRIGGER trg_project_manager_membership AFTER
INSERT
OR
UPDATE ON projects
FOR EACH ROW EXECUTE FUNCTION add_manager_to_project();

--

CREATE FUNCTION remove_member_from_project() RETURNS trigger AS $$
DECLARE
-- record project projects%ROWTYPE;
active_tasks int;
BEGIN
-- validates that the user is not manager and has no opened tasks
-- todo: authorize

-- SELECT * INTO project
-- FROM projects WHERE manager_id = OLD.user_id;

IF OLD.project_role = 'manager' THEN
    RAISE EXCEPTION 'Can not delete manager';
END IF;

SELECT COUNT(*) INTO active_tasks FROM tasks
WHERE assignee_id = OLD.user_id;

IF active_tasks > 0 THEN
    RAISE EXCEPTION 'User have active tasks: %', active_tasks;
END IF;

RETURN NULL;
END;
$$ LANGUAGE plpgsql;


CREATE OR REPLACE TRIGGER trg_project_member_removal
BEFORE
DELETE ON project_members
FOR EACH ROW EXECUTE FUNCTION remove_member_from_project();


--
CREATE OR REPLACE FUNCTION validate_assignee_is_member() RETURNS trigger AS $$
DECLARE
role ProjectRole;
BEGIN
    -- validates that the employee being assigned to task is a member of the project and not an observer
    -- TODO: authorize.

    SELECT project_role INTO role
    FROM project_members
    WHERE user_id = NEW.assignee_id AND project_id = NEW.project_id;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Can not assign non-project-member to the task';
    ELSEIF role = 'observer' THEN
        RAISE EXCEPTION 'Can not assign observer to the task';
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


CREATE OR REPLACE TRIGGER trg_task_assignee_membership
AFTER INSERT OR UPDATE ON tasks
FOR EACH ROW EXECUTE FUNCTION validate_assignee_is_member();

CREATE OR REPLACE FUNCTION validate_post_author_is_project_member RETURNS trigger AS $$
DECLARE
project int;
BEGIN
    -- validates that the poster on the task is member of the project

    SELECT project_id INTO project
    FROM tasks
    WHERE id = NEW.task_id;

    PERFORM * FROM project_members pm
    WHERE pm.user_id = NEW.author_id AND pm.project_id = project;

    IF NOT FOUND
        RAISE EXCEPTION 'Non-project member can not participate in discussion';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


CREATE OR REPLACE TRIGGER trg_validate_post_author_is_project_member
AFTER UPDATE OR INSERT ON discussion_posts
FOR EACH ROW EXECUTE FUNCTION validate_post_author_is_project_member();

-- todo: forbid history modifications

CREATE OR REPLACE FUNCTION validate_no_message_cycles() RETURNS trigger AS $$
DECLARE
parent_created_at TIMESTAMP;
BEGIN
    -- validates that each of the message's parents does not refer to the message

    -- TODO: is there a better way to ensure immutability?
    IF NEW.created_at <> OLD.created_at THEN 
        RAISE EXCEPTION 'It is forbidden to update creation time of the post';
    END IF;

    IF NEW.parent <> OLD.parent THEN
        RAISE EXCEPTION 'It is forbidden to reparent posts';
    END IF;

    SELECT created_at FROM discussion_posts
    INTO parent_created_at
    WHERE id = NEW.parent_id;

    IF NEW.created_at > parent_created_at THEN
        RAISE EXCEPTION 'Child can not be created before parent: potential post cycle detected';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;


CREATE TRIGGER trg_validate_no_message_cycles
AFTER UPDATE ON discussion_posts
FOR EACH ROW EXECUTE FUNCTION trg_validate_no_message_cycles();


CREATE FUNCTION validate_task_deadline_within_project RETURNS trigger AS $$
BEGIN
-- validates that the task's deadline ends before the project's one
END;
$$ LANGUAGE plpgsql;


CREATE TRIGGER trg_validate_task_deadline_within_project
BEFORE
UPDATE
OR
INSERT ON tasks
FOR EACH ROW EXECUTE FUNCTION validate_task_deadline_within_project();

-- todo: shift of dependency's deadline

CREATE FUNCTION validate_dependencies_belong_to_same_project RETURNS trigger AS $$
BEGIN
-- validates that task and its dependency are belonging to the same project
END;
$$ LANGUAGE plpgsql;


CREATE TRIGGER trg_validate_dependencies_belong_to_same_project
BEFORE
INSERT
OR
UPDATE ON task_dependencies
FOR EACH ROW EXECUTE FUNCTION validate_dependencies_belong_to_same_project();

-- todo: constraint to forbid self-dependencies

CREATE FUNCTION validate_task_dependencies_no_cycles RETURNS trigger AS $$
BEGIN
-- Validates that there are no cycles in the task dependency graph
END;
$$ LANGUAGE plpgsql;


CREATE TRIGGER trg_validate_task_dependencies_no_cycles
BEFORE
INSERT ON task_dependencies
FOR EACH ROW EXECUTE FUNCTION validate_task_dependencies_no_cycles();


CREATE FUNCTION task_dependents_deadline_shift RETURNS trigger AS $$
BEGIN
-- Shifts deadlines of the dependent tasks when its final dependency with missed deadline is marked ready
END;
$$ LANGUAGE plpgsql;


CREATE TRIGGER trg_task_dependents_deadline_shift AFTER
UPDATE ON tasks
FOR EACH ROW EXECUTE FUNCTION task_dependents_deadline_shift();

-- todo: constraint to forbid pending and ready

CREATE FUNCTION validate_task_dependencies_ready RETURNS trigger AS $$
BEGIN
-- before marking the task as ready, checks that all its dependencies are ready
END;
$$ LANGUAGE plpgsql;


CREATE FUNCTION validate_dependent_not_ready RETURNS trigger AS $$
BEGIN
-- validates that depending task is not ready when adding a non-ready dependency
END;
$$ LANGUAGE plpgsql;


CREATE TRIGGER trg_validate_dependent_not_ready
BEFORE
INSERT ON task_dependencies
FOR EACH ROW EXECUTE FUNCTION validate_dependent_not_ready();

-- Block of procedures
