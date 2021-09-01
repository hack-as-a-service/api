-- This file should undo anything in `up.sql`
ALTER TABLE teams DROP COLUMN slug,
    ALTER COLUMN name
SET NOT NULL