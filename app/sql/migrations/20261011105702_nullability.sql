ALTER TABLE statuses
ALTER ready SET DEFAULT false,
ALTER ready SET NOT NULL,
ALTER pending SET DEFAULT false,
ALTER pending SET NOT NULL,
ALTER color SET NOT NULL;

ALTER TABLE categories
ALTER COLUMN badge_color SET not null,
ALTER COLUMN urgency SET not null;

ALTER TABLE discussion_posts
ALTER COLUMN task_id SET not null,
ALTER COLUMN author_id SET not null;

ALTER DOMAIN Color
DROP NOT NULL;