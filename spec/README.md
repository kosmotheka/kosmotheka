# Operation spec

Every write to the catalog is an **operation**: an atomic batch of entity changes with an intent label. This folder is the reference that every implementation (the Rust app, the future PHP server, any third-party client) must follow. It is licensed under [MIT](LICENSE), unlike the rest of the repository, so anyone can build a compatible client or server.

| File | Contents |
|---|---|
| `operation.schema.json` | JSON Schema of an operation: envelope, changes and the fields each entity accepts |
| `conformance/operation-validation.json` | Operations with the expected validation result (`valid`, or `invalid` with an error code, change index and field) |
| `conformance/signatures.json` | Signed operations with the canonical bytes and the verification result every implementation must get |
| `conformance/library-merge.json` | Operations from several libraries, delivered in different orders, with the rows every order must end in |
| `conformance/merge.json` | The merge rules inside one library (field by field, last writer, delete wins, repeated delivery, broken references, one HLC per operation), in the same shape |
| `conformance/permissions.json` | Operations from devices with different roles, delivered in different orders, with which ones are applied and which become requests |
| `conformance/draft/` | Cases for the next version of the format, when there are any. Not loaded by the implementations |

## Rules the schema cannot express

- Every operation belongs to **one library** (`library`, the unit that is shared, synced and permissioned). Changes never name a library themselves: a `library_id` field in a change is an unknown field.
- A change to the **`library` entity** must have the operation's own library as its id: an operation can create, rename or delete only its own library.
- Entity ids only need to be unique **within a library**: the same id in two libraries is two different rows.
- **Deleting the library** (a `delete` change on the `library` entity) removes the library and everything in it on every device that has it. Who may do it is a permission rule, defined with roles. Leaving a library is not an operation on it: the device simply stops having it (and, with roles, the person's membership ends).
- A **`device`** row can only be created by that device itself: its id must be the device in the operation's `hlc` (`other_device` otherwise). The row carries the public key the device signs with, and the operation that creates it is signed with that key, which proves the device holds it.
- A **`mark`** (what one person notes about one work, D68) has an id derived from its person and work: the UUID v5, in the namespace `7b2f0c1e-4d8a-4f3e-9c61-2a5d8e0b4f17`, of the text `<member_id>:<work_id>` in lowercase. A mark change that sets `member_id` or `work_id` must set both, and its id must be the derived one (`wrong_id` otherwise). So the same person marking the same work on two devices writes the same row. A mark is **never deleted by a change** (`undeletable`): the tombstone would keep that id, and so that person, from ever marking the work again. Clearing a mark sets its fields to `null`, and creates send only the fields that are set, so an empty value does not overwrite what another device of the same person set. Marks go when their work or their person is deleted. Example: member `01997c7e-0000-7000-8000-000000000013` and work `01997c7e-0000-7000-8000-0000000000d1` give `9ae21929-1e26-5646-8c48-52cf10f8e911`.
- A **rating** is a whole number from 1 to 5 (`wrong_type` otherwise).
- A **text** value never contains U+0000, because PostgreSQL's JSONB cannot hold it (`wrong_type` otherwise). The six characters `\u0000` typed by a user are ordinary text.
- An **intent** is two or more words joined by single dots; a word is one or more lowercase ASCII letters or underscores, and nothing else (no digits, no capitals, no line break after the last word): `book.add`, `a_b.c_d.e` (`invalid_intent` otherwise).
- The **`hlc`** has the text form `<16 digits>-<8 hex digits>-<device>`, with a non-empty device. An operation whose `hlc` is not in that form is not an operation (it does not deserialize), so it has no validation case.
- **Create** must include every required field of the entity.
- **Required fields** can never be set to `null`.
- **Dates** must exist on the calendar (`2026-02-30` is invalid).
- **`previous`** (D87) is the id of the latest operation of the same device in the same library when the operation was made, or `null` for the first. The key is **required** even when it is null: an operation without it is not an operation (it does not deserialize). It must be a UUID and never the operation's own id (`invalid_previous`). It is part of what the signature covers. A receiver that does not have `previous` yet makes the operation **wait** (not stored, the rest of its author's with it) and one whose `previous` belongs to another device or library, or is not earlier in the clock, refuses it; a null `previous` from an author that already has operations, or two operations with the same `previous`, are accepted (forks are left to the fences). See `docs/architecture/sync.md`.
- A device's **`trusted_through`** (D84, D87) is the id of the last operation of that device the administrator had when it lost power, or `""`. Of that device's operations dated before the one that sets it, only the marked one and its ancestors through `previous` count; any other is a request.
- Errors are reported in this order: operation id, library, previous, intent, empty changes, then each change in order (id, other library, other device, action, fields in alphabetical order, missing required fields).

## Error codes

`invalid_id`, `invalid_library`, `invalid_previous`, `invalid_intent`, `no_changes`, `other_library`, `other_device`, `unknown_field`, `missing_field`, `null_required`, `wrong_type`, `empty_update`, `fields_on_delete`, `wrong_id`, `undeletable`.

## Signatures

Every operation is signed by the device that made it (the device in its `hlc`), with Ed25519:

1. Take the operation **without** its `signature` field.
2. Write it as **canonical JSON** (RFC 8785): object keys sorted by UTF-16 code units, no whitespace, strings escaped only where JSON requires it (`\"`, `\\`, `\b`, `\f`, `\n`, `\r`, `\t`, and `\u00xx` in lowercase for the other control characters), everything else written as is, in UTF-8. Operations only contain strings, integers, booleans and nulls.
3. Sign those bytes. `signature` is the 64-byte signature in base64url without padding.

The public key of each device travels with the library as a **`device`** row (32 bytes, base64url without padding). A device registers itself in the first operation it signs in a library. Receivers check the signature against the key registered for the author; the first key registered for a device is the one that counts. A key never changes: an operation that sets a different `public_key` on a registered device, its own included, is **refused**. A device that is not registered yet is known only through the `device` row it creates **for itself** in the operation (an update of its own row, or the row of another device, does not register it), and an operation from a device whose key cannot be found that way is refused. The permission cases pin this down with `rejected` operations. Operations from before signatures existed carry none; how they are treated once a library has roles is still to be decided.

`conformance/signatures.json` has, for each case, the operation, the public key, the exact canonical text and the result: `valid`, or `invalid` with `missing`, `bad_key`, `malformed` or `mismatch`. The keys in it come from fixed test bytes and must never be used for anything else.

## Roles and permissions

Every device checks each operation it receives, and each one it makes, against the roles of the library (D55, D73 in `docs/DECISIONS.md`). All devices reach the same verdict, whatever the order operations arrive in.

1. **A library without roles works as before:** until some member has a role, every device may do everything. The first role is assigned under that same rule; from then on the library has roles.
2. **Who wrote an operation** is the device in its `hlc` (signed). Its **person** is the member its `device` row points to (`member_id`), and its **role** is that member's `role` at the moment of the operation. A device linked to no member counts as a **reader**. An operation without a signature is never allowed: it is refused before it is judged (D56).
3. **Roles** (`member.role`: `admin`, `editor` or `reader`; `member.can_share`: true or false):
   - **admin** may do everything, including roles, who owns each device, access (`revocation`), the `library` row and deleting the library.
   - **editor** creates and edits the catalog. It deletes only rows its person created: a row whose first create change came from a device of the same person.
   - **reader** changes nothing in the catalog: what it tries becomes a request.
   - **Everyone keeps their own marks** (D68), whatever their role: a device with a person creates, changes and deletes the `mark` rows of that person (a create names it in `member_id`), and only administrators touch other people's. A device linked to no member may not.
   - **Nobody gives more than they have:** changing `role`, `can_share` or a device's `member_id` is for administrators.
4. **Every device registers and renames itself**, whatever its role: its own `device` row, without `member_id`.
5. **All or nothing:** if any change in an operation is not allowed, none is applied and the whole operation becomes a **request**.
6. **Roles as they were at that moment:** each operation is judged against the roles in force at its HLC, following the log in HLC order. When an operation arrives older than what a device has already applied, in a library with roles, the device re-evaluates that library from its log.
7. **Nothing is lost:** an operation that becomes a request stays in the log and travels on to other devices like any other.
8. **Resolving a request** is for administrators: an operation that creates the `change_request` row whose id is the requested operation's id, with `status` `accepted` or `rejected`. Accepting repeats the requested changes in that same operation, so they are applied as the administrator's. A request with no `change_request` row is pending.
9. **Withdrawing a request** is for whoever asked: a device of the same person as the requested operation's author may create that `change_request` row with `status` `withdrawn`, and nothing else. It is the only `change_request` row a non-administrator may write.
10. **The asker's note travels inside the request:** the requested operation may carry a `change_request` row with its own id and only a `note`. Only an administrator could create that row, so the operation always stays a request; accepting repeats the requested changes without it.

`conformance/permissions.json` has cases in the shape of the merge cases (`operations`, `deliveries`), with `expect` listing the ids of the operations that end **applied**, those that end as **requests**, and, optionally, those **rejected** (not stored at all, like an unsigned one); every delivery order must give the same lists. Deliveries keep each device's own operations in order (each one follows its `previous`), as the sync protocol does, so a device's key registration always comes before its signed operations. Every operation is signed with the test key of `conformance/signatures.json`, except those marked `"unsigned": true` (a field of the case, not of the operation), which every implementation must refuse.

The merge cases (`conformance/library-merge.json`) are written without signatures and without key registrations, to keep them about merging: each implementation registers every author of a case with the test key before delivering it (a `device` row it then leaves out of the comparison) and signs every operation with that key.

## Running the conformance cases

Each implementation loads the files in `conformance/` (not `conformance/draft/`) in its test suite and checks that every case produces the expected result. The Rust implementation does this in `crates/domain/src/operation.rs` (validation), `crates/domain/src/signature.rs` (signatures) and `app/src-tauri/src/sync.rs` (merging and permissions, all four files). The PHP server runs the validation and signature cases today (`server/tests/Validator/`, `server/tests/Sync/`). It does **not** run `library-merge.json`, `merge.json` or `permissions.json` yet: it will when it applies operations (milestones 7.4.4 and 7.4.5 of `docs/plans/fase-7.md`), and until then the Rust runners are the only check of those rules.

## Merging operations from several libraries

`conformance/library-merge.json` describes how a device applies operations received from other devices when they belong to different libraries. Each case has:

- `operations`: full operations, with their library and HLC.
- `deliveries`: orders in which to deliver them, as lists of indexes into `operations`. Each operation is delivered alone, in that order, as if it came in its own sync. **Every order must end in the same state.** Orders respect causality, as the sync protocol guarantees: a change to a row never arrives before the operation that created that row (a device always sends everything the other lacks, oldest first).
- `expect.rows`: every catalog row that must exist afterwards, as `{ library, entity, id, fields }`. Rows not listed must not exist. Fields not listed must be null: an optional field nobody set, such as a member's `role`, is left out.
- `expect.rejected` (optional): the ids of the operations that are **refused**, that is, not stored (see `previous` and the HLC rule below). When present, every other operation of the case must be taken, in every delivery. A skipped *change* (see below) is not a refused operation. When absent, the case does not say.
- A delivery may **repeat an index**: the same operation delivered again, as a device that asks twice. The second time changes nothing and is not refused.

Rules these cases pin down:

- A row is identified by **library, entity and id** together. Field versions and tombstones are kept per library too, so nothing done in one library affects another.
- Changes apply **even if the library row has not arrived yet**: the library is only the scope, not a parent row that must exist first.
- **Deleting the library** leaves a tombstone for the whole library: its rows are removed, and any later change in that library is ignored, whatever its HLC. The delete wins, as it does for any row.

## Merging operations within a library

`conformance/merge.json` is the same kind of case with one library. Every operation in it has the `previous` of its author's chain and every delivery keeps each author's operations in order, except in the one case about two operations with the same HLC. Where an operation needs a row, that row's creation comes first in every delivery (causality, as above). The rules:

- **Field by field.** A change to a row only touches the fields it names. Each field of each row remembers the HLC of the change that wrote it; a change writes a field only if its HLC is **greater** than the remembered one. Two devices that change different fields of one row keep both changes.
- **Last writer per field.** The HLC text order decides: milliseconds, then counter, then the device id as text (a higher counter beats a higher device id in the same millisecond). The result is the same in any delivery order. A tie is not newer: the same HLC never overwrites a field.
- **A change keeps the versions of the fields it lost.** A change that loses one field and wins another writes only the one it won and leaves the lost field's remembered HLC where it was, so a third change dated between the two cannot win the field later.
- **Delete wins, with tombstones.** A delete removes the row and leaves a tombstone with the row's entity and id. Whatever arrives afterwards for that row is ignored, whatever its HLC: an update, or a create with the same id. A delete also beats an update that has a higher HLC, in either order of arrival. Rows that hang from the deleted one (its `work_tag`, `series_entry` and the like) go with it, but they get no tombstone of their own.
- **Delivering an operation twice changes nothing.** An operation already stored is counted as known and skipped.
- **A broken reference skips the change, not the operation.** Each change is applied on its own: an update of a row that does not exist (never created, or removed with its parent), and a create or update whose reference points to a row that does not exist (a `copy` of an edition nobody has, a `work_tag` of a work already deleted), write nothing: no row, no field version, no tombstone. The operation itself is stored and forwarded like any other, and its other changes apply.
- **One HLC per operation.** HLCs are unique in a library's log: a second operation with an HLC already stored is **refused** (not stored), and so are the operations of its author that chain to it, because their `previous` is missing. Which of the two is kept depends on which arrives first; the case delivers the first one first. This is a rule about a modified app: an honest device never signs two operations with one clock reading.
