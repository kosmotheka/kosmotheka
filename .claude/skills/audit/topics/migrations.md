---
agent: audit-reviewer
code:
  - app/src-tauri/migrations/*.sql
  - app/src-tauri/src/db.rs::@prod
context:
  - facts/contracts.md#Schema chain
  - facts/contracts.md#Tables built from the migrations
decisions: D07 D31 D46
max_lines: 2000
---
Database migrations applied at startup.

1. For each migration: can it lose or corrupt data in an existing database (table rebuilds that forget a column or an index, dropped columns, constraints that reject rows that already exist, backfills with wrong defaults)?
2. Are migrations applied inside a transaction? What happens at startup if one fails: half-migrated database, a backup before migrating, can the app still start and tell the user?
3. An older app version opening a database migrated by a newer one.
4. Indexes for the hot paths (catalog list, search, sync by HLC and library) and foreign keys with the right `ON DELETE` behavior for local-first data (a deletion on one device must not silently cascade in ways the operation log does not record).
5. The schema-chain facts say whether migrations, `schema.rs`, the JSON Schema and the PHP mirror agree; explain any difference shown.
