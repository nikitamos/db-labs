SELECT id,
       manager_id AS manager,
       name,
       description,
       start_time,
       deadline,
       ts_rank_cd(tsearchable, websearch_to_tsquery($1)) AS rank
FROM projects
WHERE rank > $2
ORDER BY rank DESC
LIMIT $3
OFFSET $4;