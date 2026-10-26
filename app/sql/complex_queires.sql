


-- === Least busy employees ===


--=============
-- SELECT tasks.name,
--        tasks.id AS "task_id",
--        categories.name as "cat_name",
--        categories.urgency,
--        tasks.assignee_id,
--        AVG(categories.urgency) OVER (PARTITION BY tasks.assignee_id)
-- FROM tasks
-- JOIN task_categories ON task_categories.task_id = tasks.id
-- JOIN categories ON categories.id = task_categories.category_id
-- ORDER BY tasks.assignee_id;
-- ========
 -- Search: task, project by name and description
 -- Search: users by identity (names, login, email)
 -- TODO: index on tasks by assignee_id?

SELECT 
