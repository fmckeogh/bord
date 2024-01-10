-- Add migration script here
CREATE TABLE IF NOT EXISTS users (
    username VARCHAR(16) NOT NULL PRIMARY KEY,
    pseudonym VARCHAR(64) NOT NULL
);
