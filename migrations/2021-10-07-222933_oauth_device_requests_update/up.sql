-- Your SQL goes here
ALTER TABLE oauth_device_requests
ADD COLUMN token_retrieved BOOLEAN NOT NULL DEFAULT false,
    ADD COLUMN access_denied BOOLEAN NOT NULL DEFAULT false