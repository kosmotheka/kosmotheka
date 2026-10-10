-- The key someone vouched for when they said whose a device is (D96). Empty on the
-- assignments made before it existed.
ALTER TABLE devices ADD COLUMN vouched_key TEXT;
