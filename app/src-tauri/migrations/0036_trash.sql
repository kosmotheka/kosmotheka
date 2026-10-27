-- The trash (D105): the four things a person deletes from the app stay 30 days with the moment (UTC, one writing,
-- `YYYY-MM-DDTHH:MM:SS.mmmZ`) and the person who sent them there; both null when they are not in it. No foreign key on
-- `trashed_by`, like `devices.member_id`: the sync checks references itself (D89), and a person deleted later leaves
-- the date, which is what says the row is in the trash.
ALTER TABLE editions ADD COLUMN trashed_at TEXT;
ALTER TABLE editions ADD COLUMN trashed_by TEXT;
ALTER TABLE copies ADD COLUMN trashed_at TEXT;
ALTER TABLE copies ADD COLUMN trashed_by TEXT;
ALTER TABLE tags ADD COLUMN trashed_at TEXT;
ALTER TABLE tags ADD COLUMN trashed_by TEXT;
ALTER TABLE series ADD COLUMN trashed_at TEXT;
ALTER TABLE series ADD COLUMN trashed_by TEXT;

-- When this device first saw each thing in the trash, in milliseconds (D105): it counts the 30 days from the later of
-- that and `trashed_at`, so a wrong clock elsewhere or a backup restored months later deletes nothing early. Local:
-- never synced, never in a backup.
CREATE TABLE trash_seen (
    library_id TEXT NOT NULL,
    entity TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    seen_at INTEGER NOT NULL,
    PRIMARY KEY (library_id, entity, entity_id)
);
