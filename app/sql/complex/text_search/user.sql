SELECT id,
       username,
       email,
       first_name,
       second_name,
       bio,
       strict_word_similarity($1, fuzz_searchable) AS rank
FROM users
WHERE rank >= 0.3
ORDER BY (rank) DESC
LIMIT $2
OFFSET $3;