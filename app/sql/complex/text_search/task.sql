SELECT t.id,
       t.name,
       t.description,
       t.deadline,
       t.status_id AS status,
       t.assignee_id AS assignee,
       t.estimated_time,
       ts_rank_cd(tsearchable, websearch_to_tsquery($1)) AS rank
FROM tasks t
WHERE ts_rank_cd(tsearchable, websearch_to_tsquery($1)) > $2
ORDER BY rank DESC;