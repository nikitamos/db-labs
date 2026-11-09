TRUNCATE TABLE permissions CASCADE;
TRUNCATE TABLE roles CASCADE;

ALTER TABLE permissions
ADD CONSTRAINT ck_permission_name_unique
UNIQUE(name);

INSERT INTO permissions (name)
VALUES ('create_projects'),
       ('manage_projects'),
       ('assign_project_managers');

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