-- Your SQL goes here
ALTER TABLE apps
ADD CONSTRAINT apps_container_id_key UNIQUE (container_id)