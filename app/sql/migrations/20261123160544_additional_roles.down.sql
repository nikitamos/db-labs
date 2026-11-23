WITH admin AS (
    SELECT id
     FROM roles
     WHERE name = 'admin'
)
UPDATE user_roles
SET role_id = admin.role_id;

DELETE FROM roles
WHERE name <> 'admin';