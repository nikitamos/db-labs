ALTER TABLE tasks_dependencies
ADD CONSTRAINT ck_no_self_dependencies
CHECK (dependency_id <> dependent_id);

ALTER TABLE statuses
ADD CONSTRAINT ck_forbid_pending_and_ready
CHECK (NOT (ready AND pending));
