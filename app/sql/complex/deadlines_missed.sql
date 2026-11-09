-- Returns tasks of employee with missed deadlines
SELECT *
FROM tasks
WHERE assignee_id = $1
    AND deadline < NOW()
    AND NOT
        (SELECT ready
         FROM statuses
         WHERE id = tasks.status_id)
ORDER BY deadline DESC
LIMIT $2
OFFSET $3;