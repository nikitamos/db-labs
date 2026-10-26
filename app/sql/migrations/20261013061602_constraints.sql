ALTER TABLE users ADD CONSTRAINT username_length_greater_than_3 CHECK(LENGTH(username) > 3);

ALTER TABLE discussion_posts ADD COLUMN created_at TIMESTAMP DEFAULT NOW();
