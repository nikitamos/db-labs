DROP VIEW IF EXISTS post_with_author;
CREATE OR REPLACE VIEW post_with_author AS
SELECT discussion_posts.body,
       discussion_posts.id,
       discussion_posts.task_id,
       discussion_posts.parent_id,

       jsonb_build_object(
       'email', author.email, 
       'username', author.username,
       'first_name', author.first_name,
       'second_name', author.second_name,
       'author.id', author.id
       ) AS "author"
FROM discussion_posts
LEFT JOIN users author ON author.id = discussion_posts.author_id;

DROP VIEW IF EXISTS task_with_navigation_json;
CREATE VIEW task_with_navigation_json AS
SELECT t.id,
       t.name,
       t.description,
       t.deadline,
       t.estimated_time,

        jsonb_build_object(
       'id', users.id, 
       'username', users.username,
       'password_hash', users.password_hash,
       'email', users.email,
       'first_name', users.first_name,
       'second_name', users.second_name,
       'bio', users.bio) AS "assignee",

        jsonb_build_object(
       'id', statuses.id,
       'name', statuses.name,
       'color', statuses.color,
       'ready', statuses.ready,
       'pending', statuses.pending) AS "status",

        -- TODO: check that children and parents are not mixed up
       jsonb_agg(jsonb_build_object(
        'id', child.id,
        'name', child.name,
        'description', child.description,
        'deadline', child.deadline,
        'estimated_time', child.estimated_time,
        'status', child.status_id,
        'project', child.project_id,
        'assignee', child.assignee_id
       )) FILTER (WHERE child.id IS NOT NULL) AS "children",

       jsonb_agg(jsonb_build_object(
        'id', parent.id,
        'name', parent.name,
        'description', parent.description,
        'deadline', parent.deadline,
        'estimated_time', parent.estimated_time,
        'status', parent.status_id,
        'project', parent.project_id,
        'assignee', parent.assignee_id
       )) FILTER (WHERE parent.id IS NOT NULL) AS "parents",

       jsonb_build_object(
        'id', projects.id,
        'manager', projects.manager_id,
        'name', projects.name,
        'description', projects.description,
        'start_time', projects.start_time,
        'deadline', projects.deadline
       ) AS "project"
FROM tasks t
JOIN users ON users.id = t.assignee_id
LEFT JOIN tasks_dependencies t_children ON t_children.dependency_id = t.id
LEFT JOIN tasks_dependencies t_parents ON t_parents.dependent_id = t.id
LEFT JOIN tasks child ON child.id = t_children.dependent_id
LEFT JOIN tasks parent ON parent.id = t_parents.dependency_id
JOIN statuses ON statuses.id = t.status_id
JOIN projects ON projects.id = t.project_id
GROUP BY (t.id,
          users.id,
          statuses.id,
          projects.id);

CREATE EXTENSION IF NOT EXISTS pg_trgm;

-- TODO: full-text search, 'complex data operations'

ALTER TABLE projects
ADD COLUMN tsearchable tsvector
GENERATED ALWAYS AS (to_tsvector('english', name || ' ' || description)) STORED;

ALTER TABLE users
ADD COLUMN tsearchable tsvector
GENERATED ALWAYS AS (to_tsvector('english', username || ' ' || email || ' ' || first_name || ' ' || 'second_name')) STORED;

ALTER TABLE tasks
ADD COLUMN tsearchable tsvector
GENERATED ALWAYS AS (to_tsvector('english', name || ' ' || description)) STORED;

CREATE INDEX project_search_idx ON projects USING GIN (tsearchable);
CREATE INDEX user_search_idx ON users USING GIN (tsearchable);
CREATE INDEX task_search_idx ON tasks USING GIN(tsearchable);

CREATE INDEX task_by_assignee ON TASKS (assignee_id);
