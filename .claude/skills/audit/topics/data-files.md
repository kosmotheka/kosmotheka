---
agent: audit-reviewer
code:
  - app/src-tauri/src/backup.rs::@prod
  - app/src-tauri/src/csv_io.rs::@prod
  - app/src-tauri/src/data_folder.rs::@prod
  - app/src-tauri/src/db.rs::@prod
  - app/src-tauri/src/settings.rs::@prod
decisions: D27 D30 D58 D65
max_lines: 2000
---
Files that hold the family's data: database, backups, CSV, the data folder.

1. **SQLite settings** (`db.rs`): `foreign_keys`, `journal_mode`, `synchronous`, `busy_timeout`; several connections or threads writing at once.
2. **Backups** (D30, D58): how the copy is made (SQLite's backup API or `VACUUM INTO` are safe; a plain copy of a database in WAL mode can be corrupt or incomplete); is it verified after writing?; rotation (can it delete the only good copy?); failure reporting; what D58 says about keys in backups and whether the code follows it.
3. **Restore against sync**: after restoring an old backup on a paired device, what happens to the HLC, the operation log and tombstones at the next sync (deleted rows coming back, newer changes lost on this or other devices)?
4. **CSV**: export then import gives the same catalog (commas, quotes, line breaks, non-ASCII, empty values, dates, BOM); importing twice (duplicates?); a malformed row in the middle (atomic or half applied?); imports recorded as operations (AGENTS.md); cells starting with `=`, `+`, `-`, `@` exported from synced data run as formulas in spreadsheets (CWE-1236).
5. **Data folder**: moving it (copy then delete, or move? failure half way? another disk? not writable?); two instances on the same folder (lock); Android paths; the `KOSMOTHEKA_DATA_DIR` override limited to debug builds (AGENTS.md).
6. **Secrets in files**: signing seeds, sync keys and certificates, API keys: where they are written and with which permissions.
