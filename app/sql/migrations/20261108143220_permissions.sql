TRUNCATE TABLE permissions CASCADE;
TRUNCATE TABLE roles CASCADE;

ALTER TABLE permissions
ADD CONSTRAINT ck_permission_name_unique
UNIQUE(name);

INSERT INTO permissions (name)
VALUES ('create_projects'),
       ('manage_projects'),
       ('assign_project_managers'),
       ('read_users'),
       ('modify_user_roles'),
       ('read_user_roles'),
       ('modify_categories'),
       ('modify_discussion_posts'),
       ('modify_project_members'),
       ('modify_projects'),
       ('modify_sessions'),
       ('modify_statuses'),
       ('modify_task_categories'),
       ('modify_tasks'),
       ('modify_tasks_dependencies'),
       ('modify_users'),
       ('read_categories'),
       ('read_discussion_posts'),
       ('read_project_members'),
       ('read_projects'),
       ('read_statuses'),
       ('read_tasks'),
       ('write_categories'),
       ('write_discussion_posts'),
       ('write_project_members'),
       ('write_projects'),
       ('write_sessions'),
       ('write_statuses'),
       ('write_task_categories'),
       ('write_tasks'),
       ('write_tasks_dependencies'),
       ('write_user_roles'),
       ('write_users');

INSERT INTO roles (name)
VALUES ('admin');

WITH admin_id AS (
    SELECT id FROM roles
    WHERE name = 'admin'
)
INSERT INTO role_permissions (permission_id, role_id)
SELECT p.id, a.id
FROM permissions p
CROSS JOIN admin_id a;

SELECT * FROM role_permissions;