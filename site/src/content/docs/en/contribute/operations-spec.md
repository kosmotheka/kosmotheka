---
title: "The operations: the specification and the conformance cases"
description: The format of an operation, the rules the receiver checks and the test cases that Rust and PHP share, so that anyone can write a compatible client or server.
sidebar:
  order: 7
---

Every change to the catalog is an **operation**. The [`spec/`](https://github.com/kosmotheka/kosmotheka/tree/main/spec) folder defines what an operation is and which rules it follows, in a format that does not belong to any language. This page explains what is in it, how the app (Rust) and the server (PHP) use it, and how to add a test case. To understand why operations exist, start with [The sync engine](../sync-engine/).

**`spec/` has an MIT licence**, different from the AGPL of the rest of the project. *Why:* so that anyone can write a compatible client or server without inheriting the AGPL. Nothing in that folder depends on our code.

## What is in `spec/`

| File | What it contains |
|---|---|
| `operation.schema.json` | The JSON Schema of an operation: the envelope, the changes and the fields each entity accepts |
| `sync.json` | What everyone must share to sync: the format version (`schemaVersion`). Devices and servers with another version do not sync |
| `README.md` | The rules a JSON Schema cannot express, with their error codes |
| `conformance/operation-validation.json` | Operations with the result of validating them: `valid`, or `invalid` with an error code, the index of the change and the field |
| `conformance/signatures.json` | Signed operations, with the exact canonical bytes and whether the signature holds |
| `conformance/merge.json` | How operations are merged within a library (field by field, deletion wins, repeated deliveries…) |
| `conformance/library-merge.json` | The same, with several libraries |
| `conformance/permissions.json` | Operations from devices with different roles, delivered in different orders: which are applied and which stay as requests |

## The format of an operation

An operation is a JSON object with these fields. The JSON Schema requires all of them except `signature`, but **an operation without a signature is always rejected** by whoever receives it:

| Field | What it is |
|---|---|
| `id` | A UUID v7 |
| `library` | The library that **all** its changes belong to. A change cannot name another one |
| `hlc` | The mark of the [hybrid logical clock](../sync-engine/#2-the-hybrid-logical-clock-putting-things-in-order-without-a-shared-clock): 16 digits, 8 hexadecimal characters and the device's id (which is a lowercase UUID), separated by hyphens |
| `intent` | The intent label: two or more lowercase words separated by dots (`book.add`). **It only informs**: the permissions and the merge look at the changes, never at the label |
| `previous` | The id of the previous operation of the same device in that library, or `null` if it is the first. **The key is always present**, even if null. It is signed |
| `changes` | A list of one or more changes |
| `signature` | The Ed25519 signature of the device that made it (the one in the `hlc`) |

Each change is `{ entity, id, action, fields }`: the entity it touches, the id of its row, the action (`create`, `update` or `delete`) and the fields. A `delete` carries no fields; an `update` carries at least one.

An example, simplified: adding a book creates four rows in a single operation.

```json
{
  "id": "0199a1b2-…",
  "library": "0199a0aa-…",
  "hlc": "0001727222222222-00000000-0199a0ff-…",
  "intent": "book.add",
  "previous": "0199a1a0-…",
  "changes": [
    { "entity": "work", "id": "0199…11", "action": "create",
      "fields": { "kind": "book", "title": "The Name of the Wind" } },
    { "entity": "edition", "id": "0199…12", "action": "create",
      "fields": { "work_id": "0199…11", "title": "The Name of the Wind" } },
    { "entity": "book_details", "id": "0199…12", "action": "create",
      "fields": { "isbn13": "9788401352188" } },
    { "entity": "copy", "id": "0199…13", "action": "create",
      "fields": { "edition_id": "0199…12", "format": "hardcover", "quantity": 1 } }
  ],
  "signature": "…"
}
```

### Entities

The rows that an operation can touch (`entity`) are: `library`, `work`, `person`, `work_contributor`, `edition`, `book_details`, `comic_details`, `boardgame_details`, `film_details`, `tv_series_details`, `member`, `copy`, `publisher`, `tag`, `work_tag`, `work_genre`, `series`, `series_entry`, `work_relation`, `revocation`, `device`, `change_request` and `mark`. Which fields each accepts is in `operation.schema.json`, and it must match `crates/domain/src/schema.rs` and `server/src/Enum/CatalogEntity.php`. The names are explained in the [glossary](../glossary/).

### Intents

The label says what the person wanted to do, for the history and for showing it. They are things like `book.add`, `copy.add`, `copy.edit`, `copy.remove`, `book.remove`, `edition.edit`, `series.add`, `mark.set`, `library.found`, `library.revoke`, `device.assign`, `member.withdraw` or `request.accept`. **There is no closed list**: the format's rule is only "lowercase words separated by dots". That it does not count for the permissions is deliberate: that way nobody can smuggle a forbidden change under an innocent label.

## The rules the receiver checks

Whoever receives an operation (the app or the server) makes two checks, in this order.

**1. It is a well-formed operation (validation).** The envelope and each change are checked. These are the most important rules:

- the `id` is a UUID; the `library` and the `previous` have a valid form, and `previous` is never the id of the operation itself;
- the intent has the right format and there is at least one change;
- a change cannot name another library (a `library_id` in a change is an unknown field), nor create the `device` row of another device;
- the fields exist for that entity and have the right type; the required ones are present when creating and are **never null**;
- dates exist in the calendar (`2026-02-30` is not valid); an ISBN is thirteen digits; the rating goes from 1 to 5; the quantity of a copy is 1 or more;
- no text contains the null character (PostgreSQL cannot store it).

Only the **first error** is reported, in a fixed order (id, library, `previous`, intent, empty changes and then each change), with a stable code: `invalid_id`, `invalid_library`, `invalid_previous`, `invalid_intent`, `no_changes`, `other_library`, `other_device`, `unknown_field`, `missing_field`, `null_required`, `wrong_type`, `empty_update`, `fields_on_delete`, `wrong_id` and `undeletable`. *Why only one and in order:* so that Rust and PHP give **exactly the same result**, case by case.

**2. It is allowed and trustworthy (signature, permissions, chain).**

- The **signature** holds. It is computed over the operation without the `signature` field, written in canonical JSON ([RFC 8785](https://www.rfc-editor.org/rfc/rfc8785): sorted keys, no spaces), with the public key of its author, which travels with the library in a `device` row. The first key registered for a device is the one that counts.
- The **permissions**: each change is judged by the author's role at that moment. An administrator can do everything; an editor creates and edits, and deletes only what their person created; a reader changes nothing in the catalog. Each person notes their own things (their marks) whatever their role. Nobody grants more role than they have. If any change is not allowed, **the whole operation is stored as a request**. The full rules are in `spec/README.md`, section "Roles and permissions".
- The **chain**: if the author's previous operation is not known, the operation waits. An operation more than an hour in the future is rejected. Two operations with the same mark: the second is rejected.

More detail on the merge rules is in the section "Merging operations within a library" of `spec/README.md`.

## The conformance cases

A **conformance case** is an example operation (or several), with the result that **every** implementation must give. They are how we check that the app and the server have not drifted apart, and what lets a third party check that its client or server is compatible. There are five files, and they have different shapes.

- **`operation-validation.json`:** a list of `{ name, operation, expect }`, where `expect` is `{ "result": "valid" }` or `{ "result": "invalid", "code": …, … }`.
- **`signatures.json`:** the operation, the public key, the exact canonical text and the result (`valid`, or `invalid` with `missing`, `bad_key`, `malformed` or `mismatch`). The keys in these cases come from fixed test bytes and must not be used for anything else.
- **`merge.json`, `library-merge.json` and `permissions.json`:** a list of `operations`, a list of `deliveries` (orders in which they are delivered, as lists of indices) and the expected result. **Every delivery order must end the same.** `merge` and `library-merge` say which rows must exist at the end; `permissions` says which operations end applied, which as requests and, optionally, which are rejected.

### How Rust and PHP run them

| Cases | Rust | PHP |
|---|---|---|
| Validation | `crates/domain/src/operation.rs` | `server/tests/Validator/ValidOperationValidatorTest.php` |
| Signatures | `crates/domain/src/signature.rs` | `server/tests/Sync/OperationSignatureTest.php` |
| Merge | `app/src-tauri/src/sync.rs` (`merge_conformance_cases`) | `server/tests/Sync/Merge/MergeConformanceTest.php` |
| Permissions | `app/src-tauri/src/sync.rs` (`permission_conformance_cases`) | `server/tests/Sync/Permission/PermissionConformanceTest.php` |

Each implementation **loads the files in `conformance/`** (not those in `conformance/draft/`, which are for the next version of the format) and checks that each case gives what is expected. If one diverges, its test fails. In addition, `npm run check:spec` (in `app/`) checks that the JSON Schema agrees with the validation cases.

## Adding a case

When you change a rule, or find a case nobody had tested, the case goes **first**, before the change in the code (and you see it fail, if it is a fix).

1. **Choose the file.** A rule about shape or types goes in `operation-validation.json`; one about merging, in `merge.json` (or `library-merge.json` if several libraries are involved); one about roles, in `permissions.json`; one about signatures, in `signatures.json`.
2. **Write the case** following the ones already in that file, with a `name` that says what it tests. Use recognisable test ids, like the ones already there (`01997c7e-0000-7000-8000-…`).
3. **Write the expected result** reasoning from the rule, not from what the code does today. If it is a merge or permissions case, write several delivery orders: that is what shows it converges.
4. **Run the tests on both sides**:
   - `npm run check:spec` and `cargo test` (the app);
   - `make test` in `server/` (PHP).
5. **If the format changed**, also update `operation.schema.json`, `schema.rs` and `CatalogEntity.php`, the schema version in `sync.json` (`schemaVersion`) and the rules in `spec/README.md`. It is explained step by step in [Adding a field or an entity](../how-to-contribute/#adding-a-field-or-an-entity-step-by-step).

A rule that changes **goes first to `spec/`**, with its cases, and then to both implementations.

## Writing a compatible client or server

If you want to make another implementation, this is what you need:

1. Accept and produce operations with the format of `operation.schema.json` and the rules in `spec/README.md`.
2. Sign and check signatures with Ed25519 over the canonical JSON.
3. Compute hybrid logical clock marks and compare them by text order.
4. Apply the changes with the merge rules (field by field, deletion wins, tombstones).
5. Judge the permissions with the role rules.
6. Pass the cases in `conformance/`.

The cases are designed so that you can start with them: the complete set of rules is what they enforce.
