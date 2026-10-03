-- The cloud a library is linked to (D84): its id there and the server's address,
-- set by the signed link operation of an administrator.
ALTER TABLE libraries ADD COLUMN cloud_library_id TEXT;
ALTER TABLE libraries ADD COLUMN cloud_server TEXT;
