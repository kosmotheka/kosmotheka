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
