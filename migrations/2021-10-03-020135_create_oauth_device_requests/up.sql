-- Your SQL goes here
CREATE TABLE oauth_device_requests (
    id SERIAL PRIMARY KEY,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMP NOT NULL DEFAULT NOW() + INTERVAL '15 minutes',
    oauth_app_id TEXT NOT NULL REFERENCES oauth_apps (client_id),
    token TEXT NULL REFERENCES tokens (token),
    device_code TEXT NOT NULL,
    user_code TEXT NOT NULL
)