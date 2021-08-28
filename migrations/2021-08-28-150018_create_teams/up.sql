-- Your SQL goes here
CREATE TABLE teams (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    name TEXT NOT NULL,
    avatar TEXT,
    personal BOOLEAN NOT NULL DEFAULT FALSE
)
