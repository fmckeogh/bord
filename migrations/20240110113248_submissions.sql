-- Add migration script here
CREATE TABLE IF NOT EXISTS submissions (
    username VARCHAR(16) NOT NULL,
    submitted_at BIGINT NOT NULL DEFAULT CAST(EXTRACT(EPOCH FROM CURRENT_TIMESTAMP) AS BIGINT),
    file_contents BYTEA NOT NULL,
    file_name TEXT NOT NULL,
    PRIMARY KEY(username, submitted_at)
);
