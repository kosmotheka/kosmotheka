# Operation spec

Every write to the catalog is an **operation**: an atomic batch of entity changes with an intent label. This folder is the reference that every implementation (the Rust app, the future PHP server, any third-party client) must follow. It is licensed under [MIT](LICENSE), unlike the rest of the repository, so anyone can build a compatible client or server.

| File | Contents |
|---|---|
| `operation.schema.json` | JSON Schema of an operation: envelope, changes and the fields each entity accepts |
| `conformance/operation-validation.json` | Operations with the expected validation result (`valid`, or `invalid` with an error code, change index and field) |

## Rules the schema cannot express

- **Create** must include every required field of the entity.
- **Required fields** can never be set to `null`.
- **Dates** must exist on the calendar (`2026-02-30` is invalid).
- Errors are reported in this order: operation id, intent, empty changes, then each change in order (id, action, fields in alphabetical order, missing required fields).

## Error codes

`invalid_id`, `invalid_intent`, `no_changes`, `unknown_field`, `missing_field`, `null_required`, `wrong_type`, `empty_update`, `fields_on_delete`.

## Running the conformance cases

Each implementation loads `conformance/*.json` in its test suite and checks that every case produces the expected result. The Rust implementation does this in `crates/domain/src/operation.rs`.
