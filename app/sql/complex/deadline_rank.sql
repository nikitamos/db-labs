SELECT t1.id AS task_id,
    t1.assignee_id,
    row_number() OVER w AS deadline_rank,
    t1.deadline - lag(t1.deadline) OVER w AS t1.deadline_gap,
    t1.deadline - lag(t1.deadline) OVER w - t1.estimated_time * INTERVAL '1 hour' AS spare_time,
FROM tasks t1
WHERE assignee_id IN (SELECT assignee_id FROM tasks WHERE project_id = $1)
WINDOW w AS (
        PARTITION BY t1.assignee_id
        ORDER BY t1.deadline, t1.estimated_time, id
    );