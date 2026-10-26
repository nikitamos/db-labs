SELECT id,
       username,
       email,
       first_name,
       second_name,
       bio,
       strict_word_similarity($1, fuzz_searchable) AS rank
FROM users
WHERE strict_word_similarity($1, fuzz_searchable) >= 0.3
ORDER BY (rank) DESC
LIMIT $2
OFFSET $3;