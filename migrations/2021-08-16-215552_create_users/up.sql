-- Your SQL goes here
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    slack_user_id TEXT NOT NULL UNIQUE,
    name TEXT,
    avatar TEXT
)