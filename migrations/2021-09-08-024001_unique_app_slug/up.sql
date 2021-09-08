-- Your SQL goes here
ALTER TABLE apps
ADD CONSTRAINT apps_slug_key UNIQUE (slug)