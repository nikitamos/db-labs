SELECT assignee_id,
       COUNT(*) AS task_count
FROM tasks
WHERE assignee_id IN
        (SELECT assignee_id
         FROM tasks
         WHERE $1 IS NULL
             OR project_id = $1)
GROUP BY assignee_id
HAVING COUNT(*) < $2
ORDER BY task_count;