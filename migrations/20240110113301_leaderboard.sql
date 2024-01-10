-- Add migration script here
CREATE TABLE IF NOT EXISTS leaderboard (
    username VARCHAR(16) NOT NULL PRIMARY KEY,
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    language VARCHAR(16) NOT NULL,
    score INTEGER NOT NULL
);
