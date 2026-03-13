ALTER TABLE plugin
    ADD COLUMN github_username_claim TEXT NOT NULL DEFAULT '',
    ADD COLUMN github_user_id_claim BIGINT;

UPDATE plugin
SET github_username_claim = COALESCE(NULLIF(author_name, ''), slug)
WHERE github_username_claim = '';

ALTER TABLE plugin
    ALTER COLUMN github_username_claim DROP DEFAULT;
