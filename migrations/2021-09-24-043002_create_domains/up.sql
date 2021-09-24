-- Your SQL goes here
CREATE TABLE domains (
    id SERIAL PRIMARY KEY,
    domain TEXT NOT NULL,
    verified BOOLEAN NOT NULL DEFAULT false,
    app_id INTEGER NOT NULL REFERENCES apps (id),
    UNIQUE (domain, app_id)
);
CREATE UNIQUE INDEX domain_uniqueness ON domains (domain)
WHERE (verified);