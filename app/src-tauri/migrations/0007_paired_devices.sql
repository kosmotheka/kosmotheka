-- Devices paired for LAN sync (D34). Local to this device: never synced or exported.

-- Devices this one connects to. The secret proves who we are; the fingerprint pins their certificate.
CREATE TABLE known_servers (
    device_id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    addresses TEXT NOT NULL CHECK (json_valid(addresses)),
    fingerprint TEXT NOT NULL,
    secret TEXT NOT NULL,
    paired_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_sync_at TEXT
) STRICT;

-- Devices that connect to this one. Only a hash of their secret is kept.
CREATE TABLE known_clients (
    device_id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    secret_hash TEXT NOT NULL,
    paired_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    last_sync_at TEXT
) STRICT;
