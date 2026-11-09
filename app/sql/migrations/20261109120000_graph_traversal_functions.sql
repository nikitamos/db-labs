CREATE OR REPLACE FUNCTION task_dependencies(root_task_id integer)
RETURNS TABLE (
    task_id integer,
    depth integer
)
LANGUAGE sql
STABLE
AS $$
WITH RECURSIVE traversal(task_id, depth) AS (
    SELECT root_task_id, 0
    UNION ALL
    SELECT dependency.dependency_id,
           traversal.depth + 1
    FROM traversal
    JOIN tasks_dependencies dependency
      ON dependency.dependent_id = traversal.task_id
),
reachable AS (
    SELECT traversal.task_id, MIN(traversal.depth) AS depth
    FROM traversal
    GROUP BY traversal.task_id
)
SELECT reachable.task_id, reachable.depth
FROM reachable
WHERE reachable.depth > 0
ORDER BY reachable.depth, reachable.task_id;
$$;


CREATE OR REPLACE FUNCTION task_dependents(root_task_id integer)
RETURNS TABLE (
    task_id integer,
    depth integer
)
LANGUAGE sql
STABLE
AS $$
WITH RECURSIVE traversal(task_id, depth) AS (
    SELECT root_task_id, 0
    UNION ALL
    SELECT dependency.dependent_id,
           traversal.depth + 1
    FROM traversal
    JOIN tasks_dependencies dependency
      ON dependency.dependency_id = traversal.task_id
),
reachable AS (
    SELECT traversal.task_id, MIN(traversal.depth) AS depth
    FROM traversal
    GROUP BY traversal.task_id
)
SELECT reachable.task_id, reachable.depth
FROM reachable
WHERE reachable.depth > 0
ORDER BY reachable.depth, reachable.task_id;
$$;


CREATE OR REPLACE FUNCTION discussion_post_ancestors(root_post_id integer)
RETURNS TABLE (
    post_id integer,
    parent_id integer,
    depth integer
)
LANGUAGE sql
STABLE
AS $$
WITH RECURSIVE traversal(post_id, parent_id, depth) AS (
    SELECT post.id, post.parent_id, 0
    FROM discussion_posts post
    WHERE post.id = root_post_id
    UNION ALL
    SELECT parent.id,
           parent.parent_id,
           traversal.depth + 1
    FROM traversal
    JOIN discussion_posts parent
      ON parent.id = traversal.parent_id
)
SELECT traversal.post_id, traversal.parent_id, traversal.depth
FROM traversal
ORDER BY traversal.depth, traversal.post_id;
$$;


CREATE OR REPLACE FUNCTION discussion_post_descendants(root_post_id integer)
RETURNS TABLE (
    post_id integer,
    parent_id integer,
    depth integer
)
LANGUAGE sql
STABLE
AS $$
WITH RECURSIVE traversal(post_id, parent_id, depth) AS (
    SELECT post.id, post.parent_id, 0
    FROM discussion_posts post
    WHERE post.id = root_post_id
    UNION ALL
    SELECT child.id,
           child.parent_id,
           traversal.depth + 1
    FROM traversal
    JOIN discussion_posts child
      ON child.parent_id = traversal.post_id
)
SELECT traversal.post_id, traversal.parent_id, traversal.depth
FROM traversal
ORDER BY traversal.depth, traversal.post_id;
$$;