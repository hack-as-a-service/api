-- Your SQL goes here
ALTER TABLE tokens
ALTER COLUMN expires_at
SET DEFAULT NOW() + INTERVAL '30 days'