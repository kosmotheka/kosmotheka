-- The cloud's power in the protocol (D76, D95). `cloud_device_id` names the device that
-- is the cloud in this library; it is not a reference, because the device may register
-- after the field names it. `account_id` says a person has an account; only the cloud
-- writes it.
ALTER TABLE libraries ADD COLUMN cloud_device_id TEXT;
ALTER TABLE members ADD COLUMN account_id TEXT;
