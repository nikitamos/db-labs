SELECT id,
       name,
       description,
       deadline,
       status_id AS status,
       project_id AS project,
       assignee_id AS assignee,
       estimated_time
FROM tasks
WHERE assignee_id = $1
    AND deadline > NOW()
    AND NOT
        (SELECT ready
         FROM statuses
         WHERE id = tasks.status_id)
ORDER BY deadline
LIMIT 1;