-- Your SQL goes here
ALTER TABLE domains DROP CONSTRAINT domains_app_id_fkey;
ALTER TABLE domains
ADD CONSTRAINT domains_app_id_fkey FOREIGN KEY (app_id) REFERENCES apps (id) ON DELETE CASCADE;