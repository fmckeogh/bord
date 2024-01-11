-- Add migration script here
CREATE TABLE IF NOT EXISTS leaderboard (
    username VARCHAR(16) NOT NULL PRIMARY KEY,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    file_name TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    score INTEGER NOT NULL
);
