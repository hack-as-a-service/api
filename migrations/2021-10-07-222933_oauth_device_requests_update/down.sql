-- This file should undo anything in `up.sql`
ALTER TABLE oauth_device_requests DROP COLUMN token_retrieved,
    DROP COLUMN access_denied