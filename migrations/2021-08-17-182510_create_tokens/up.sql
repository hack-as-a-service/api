-- Your SQL goes here
CREATE TABLE tokens (
    token TEXT PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL,
    user_id INTEGER NOT NULL REFERENCES users (id)
)