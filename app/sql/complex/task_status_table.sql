SELECT * from statuses
FULL OUTER JOIN tasks ON tasks.status_id = statuses.id
WHERE ($1 IS NULL
       OR project_id = $1)
    AND ($2 IS NULL
         OR assignee_id = $2);