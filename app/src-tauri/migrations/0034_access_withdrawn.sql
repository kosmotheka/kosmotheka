-- A person whose access was withdrawn (D98): they stay in the catalog, but none of their devices counts
-- as theirs and the cloud links them no more. Null reads as false, like `can_share`.
ALTER TABLE members ADD COLUMN access_withdrawn INTEGER CHECK (access_withdrawn IS NULL OR access_withdrawn IN (0, 1));
