# Operation spec

Every write to the catalog is an **operation**: an atomic batch of entity changes with an intent label. This folder is the reference that every implementation (the Rust app, the future PHP server, any third-party client) must follow. It is licensed under [MIT](LICENSE), unlike the rest of the repository, so anyone can build a compatible client or server.

| File | Contents |
|---|---|
| `operation.schema.json` | JSON Schema of an operation: envelope, changes and the fields each entity accepts |
| `conformance/operation-validation.json` | Operations with the expected validation result (`valid`, or `invalid` with an error code, change index and field) |
| `conformance/draft/` | Cases for the next version of the format. Not loaded by the implementations yet; see below |

## Rules the schema cannot express

- **Create** must include every required field of the entity.
- **Required fields** can never be set to `null`.
- **Dates** must exist on the calendar (`2026-02-30` is invalid).
- Errors are reported in this order: operation id, intent, empty changes, then each change in order (id, action, fields in alphabetical order, missing required fields).

## Error codes

`invalid_id`, `invalid_intent`, `no_changes`, `unknown_field`, `missing_field`, `null_required`, `wrong_type`, `empty_update`, `fields_on_delete`.

## Running the conformance cases

Each implementation loads the files in `conformance/` (not `conformance/draft/`) in its test suite and checks that every case produces the expected result. The Rust implementation does this in `crates/domain/src/operation.rs`.

## Draft: operations that belong to a library

`conformance/draft/library-scope.json` describes the next version of the format, where every operation belongs to one library (the unit that is shared, synced and permissioned). It becomes part of the spec, together with the matching schema change, when the implementations support it.

- The envelope gains **`library`**: the id (UUID) of the library the operation belongs to. Changes never name a library themselves; a `library_id` field in a change is an unknown field.
- A new entity **`library`**, with one required text field, `name`. A change to a `library` entity must have the operation's own library as its id: an operation can create, rename or otherwise change only its own library.
- Entity ids only need to be unique within a library: the same id in two libraries is two different rows.
- **Deleting the library** (a `delete` change on the `library` entity) removes the library and everything in it on every device that has it. Who may do it is a permission rule, defined with roles. Leaving a library is not an operation on it: the device simply stops having it (and, with roles, the person's membership ends).
- New error codes: `invalid_library` (the library is not a UUID) and `other_library` (a change to a library that is not the operation's).
- Error order: operation id, **library**, intent, empty changes, then each change in order (id, **other library**, action, fields in alphabetical order, missing required fields).
