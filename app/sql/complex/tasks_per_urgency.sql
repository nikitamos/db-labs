-- User statistics: count of tasks with each urgency
WITH task_urgencies AS
    (SELECT MAX(categories.urgency) AS task_urgency,
            tasks.id
     FROM categories
     JOIN task_categories ON task_categories.category_id = categories.id
     JOIN tasks ON tasks.id = task_categories.task_id
     WHERE tasks.assignee_id = $1
     GROUP BY tasks.id)
SELECT COUNT(*),
       task_urgency
FROM task_urgencies
GROUP BY task_urgency;