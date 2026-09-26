# Operation spec

Every write to the catalog is an **operation**: an atomic batch of entity changes with an intent label. This folder is the reference that every implementation (the Rust app, the future PHP server, any third-party client) must follow. It is licensed under [MIT](LICENSE), unlike the rest of the repository, so anyone can build a compatible client or server.

| File | Contents |
|---|---|
| `operation.schema.json` | JSON Schema of an operation: envelope, changes and the fields each entity accepts |
| `conformance/operation-validation.json` | Operations with the expected validation result (`valid`, or `invalid` with an error code, change index and field) |
| `conformance/signatures.json` | Signed operations with the canonical bytes and the verification result every implementation must get |
| `conformance/library-merge.json` | Operations from several libraries, delivered in different orders, with the rows every order must end in |
| `conformance/draft/` | Cases for the next version of the format, when there are any. Not loaded by the implementations. Now: roles and permissions (phase 5), with the proposed rules in its README |

## Rules the schema cannot express

- Every operation belongs to **one library** (`library`, the unit that is shared, synced and permissioned). Changes never name a library themselves: a `library_id` field in a change is an unknown field.
- A change to the **`library` entity** must have the operation's own library as its id: an operation can create, rename or delete only its own library.
- Entity ids only need to be unique **within a library**: the same id in two libraries is two different rows.
- **Deleting the library** (a `delete` change on the `library` entity) removes the library and everything in it on every device that has it. Who may do it is a permission rule, defined with roles. Leaving a library is not an operation on it: the device simply stops having it (and, with roles, the person's membership ends).
- A **`device`** row can only be created by that device itself: its id must be the device in the operation's `hlc` (`other_device` otherwise). The row carries the public key the device signs with, and the operation that creates it is signed with that key, which proves the device holds it.
- **Create** must include every required field of the entity.
- **Required fields** can never be set to `null`.
- **Dates** must exist on the calendar (`2026-02-30` is invalid).
- Errors are reported in this order: operation id, library, intent, empty changes, then each change in order (id, other library, other device, action, fields in alphabetical order, missing required fields).

## Error codes

`invalid_id`, `invalid_library`, `invalid_intent`, `no_changes`, `other_library`, `other_device`, `unknown_field`, `missing_field`, `null_required`, `wrong_type`, `empty_update`, `fields_on_delete`.

## Signatures

Every operation is signed by the device that made it (the device in its `hlc`), with Ed25519:

1. Take the operation **without** its `signature` field.
2. Write it as **canonical JSON** (RFC 8785): object keys sorted by UTF-16 code units, no whitespace, strings escaped only where JSON requires it (`\"`, `\\`, `\b`, `\f`, `\n`, `\r`, `\t`, and `\u00xx` in lowercase for the other control characters), everything else written as is, in UTF-8. Operations only contain strings, integers, booleans and nulls.
3. Sign those bytes. `signature` is the 64-byte signature in base64url without padding.

The public key of each device travels with the library as a **`device`** row (32 bytes, base64url without padding). A device registers itself in the first operation it signs in a library. Receivers check the signature against the key registered for the author; the first key registered for a device is the one that counts. Operations from before signatures existed carry none; how they are treated once a library has roles is still to be decided.

`conformance/signatures.json` has, for each case, the operation, the public key, the exact canonical text and the result: `valid`, or `invalid` with `missing`, `bad_key`, `malformed` or `mismatch`. The keys in it come from fixed test bytes and must never be used for anything else.

## Running the conformance cases

Each implementation loads the files in `conformance/` (not `conformance/draft/`) in its test suite and checks that every case produces the expected result. The Rust implementation does this in `crates/domain/src/operation.rs` (validation) and `app/src-tauri/src/sync.rs` (merging); the PHP server runs the validation cases in `server/tests/Validator/` and will run the merge cases when it applies operations.

## Merging operations from several libraries

`conformance/library-merge.json` describes how a device applies operations received from other devices when they belong to different libraries. Each case has:

- `operations`: full operations, with their library and HLC.
- `deliveries`: orders in which to deliver them, as lists of indexes into `operations`. Each operation is delivered alone, in that order, as if it came in its own sync. **Every order must end in the same state.** Orders respect causality, as the sync protocol guarantees: a change to a row never arrives before the operation that created that row (a device always sends everything the other lacks, oldest first).
- `expect.rows`: every catalog row that must exist afterwards, as `{ library, entity, id, fields }`. Rows not listed must not exist.

Rules these cases pin down:

- A row is identified by **library, entity and id** together. Field versions and tombstones are kept per library too, so nothing done in one library affects another.
- Changes apply **even if the library row has not arrived yet**: the library is only the scope, not a parent row that must exist first.
- **Deleting the library** leaves a tombstone for the whole library: its rows are removed, and any later change in that library is ignored, whatever its HLC. The delete wins, as it does for any row.

