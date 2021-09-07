-- Your SQL goes here
CREATE TABLE apps (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    slug TEXT NOT NULL,
    team_id INTEGER NOT NULL REFERENCES teams (id),
    enabled BOOLEAN NOT NULL DEFAULT false,
    container_id TEXT
)