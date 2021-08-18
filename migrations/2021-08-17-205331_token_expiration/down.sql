-- This file should undo anything in `up.sql`
ALTER TABLE tokens
ALTER COLUMN expires_at DROP DEFAULT