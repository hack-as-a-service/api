-- Your SQL goes here
ALTER TABLE oauth_device_requests DROP CONSTRAINT oauth_device_requests_token_fkey;
ALTER TABLE oauth_device_requests
ADD CONSTRAINT oauth_device_requests_token_fkey FOREIGN KEY (token) REFERENCES tokens (token) ON DELETE CASCADE;