INSERT INTO roles(name)
VALUES ('manager'),
       ('employee')
ON CONFLICT DO NOTHING;

WITH manager_id AS
    (SELECT id
     FROM roles
     WHERE name = 'manager')
INSERT INTO role_permissions(permission_id, role_id)
SELECT p.id, m.id
FROM permissions p
CROSS JOIN manager_id m
WHERE p.name IN ('create_projects',
                 'manage_projects',
                 'read_users',
                 'modify_user_roles',
                 'read_user_roles',
                 'modify_categories',
                 'modify_project_members',
                 'modify_projects',
                 'modify_statuses',
                 'modify_tasks',
                 'modify_task_dependencies',
                 'read_categories',
                 'read_discussion_posts',
                 'read_project_members',
                 'read_projects',
                 'read_statuses',
                 'read_tasks',
                 'write_categories',
                 'write_discussion_posts',
                 'write_project_members',
                 'write_projects',
                 'write_statuses',
                 'write_task_categories',
                 'write_tasks',
                 'write_tasks_dependencies');

WITH employee_id AS
    (SELECT id
     FROM roles
     WHERE name = 'employee')
INSERT INTO role_permissions(permission_id, role_id)
SELECT p.id, e.id
FROM permissions p
CROSS JOIN employee_id e
WHERE p.name IN ('read_users',
                 'read_user_roles',
                 'modify_statuses',
                 'modify_task_dependencies',
                 'read_categories',
                 'read_discussion_posts',
                 'read_project_members',
                 'read_projects',
                 'read_statuses',
                 'read_tasks',
                 'modify_tasks',
                 'write_discussion_posts',
                 'write_project_members',
                 'write_projects',
                 'write_statuses',
                 'write_task_categories');

WITH mgr_role AS
    (SELECT id
     FROM roles
     WHERE name='manager')
UPDATE user_roles
SET role_id = mgr_role.id
FROM mgr_role
WHERE user_id IN (
    SELECT manager_id FROM projects
);

WITH emp_role AS
    (SELECT id
     FROM roles
     WHERE name='employee')
UPDATE user_roles
SET role_id = emp_role.id
FROM emp_role
WHERE user_id IN (
    SELECT pr.user_id FROM project_members pr
    WHERE pr.project_role IN ('observer', 'worker')
);