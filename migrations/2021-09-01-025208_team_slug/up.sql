-- Your SQL goes here
ALTER TABLE teams
ADD COLUMN slug TEXT NOT NULL UNIQUE,
    ALTER COLUMN name DROP NOT NULL