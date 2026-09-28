---
agent: audit-reviewer-strong
code:
  - crates/domain/src/permission.rs::@prod
  - app/src-tauri/src/permissions.rs::@prod
  - app/src-tauri/src/sync.rs::@prod
  - app/src-tauri/src/requests.rs::@prod
  - app/src-tauri/src/people.rs::@prod
context:
  - facts/contracts.md#Conformance cases
decisions: D40 D45 D47 D54 D55 D56 D58 D65 D71 D73
max_lines: 2800
---
Without a server each device runs code its owner controls: a modified app can write anything into its own database and send it. Permissions only hold if **every receiver checks them** (D73).

1. When `sync.rs` receives operations: is each change checked against the role its author had in that library **at that moment**, using data the receiver trusts, not what the sender claims? If a device trusts what a peer says about roles, that is `crítica`.
2. Roles (D73, D58, D45): who can grant or remove each role; is a role change itself a checked operation; races (two admins removing each other offline; the last admin; D58's recovery path); "the editor deletes only what they created" (D45).
3. Retirada de acceso (D71): who can withdraw it; how it is authenticated and spread; what a withdrawn device can still do. The classic hole: a withdrawn device stamps new operations with an HLC from **before** its withdrawal and the others accept them as old.
4. Change requests (D55): a reader's request cannot carry direct changes; accepting it is an operation checked like any other; a request cannot be accepted by someone without the role; a request from a withdrawn device.
5. The permission matrix (role × action) as the code implements it, against D73 and the `permissions` conformance cases: list every cell that differs.

Every finding names the attacker (a member with a modified app, a withdrawn device…) and the exact operation they would send.
