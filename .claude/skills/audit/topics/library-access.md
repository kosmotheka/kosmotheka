---
agent: audit-reviewer-strong
code:
  - app/src-tauri/src/cloud/mod.rs::status|upload|sync_library|pull_and_merge|push_missing|sync_all|adopt_link|download|link_of|has_unsigned_history|link_change|ensure_not_linked|linked_libraries|links_to|is_registered|remove_unlinked
  - app/src-tauri/src/cloud/client.rs::@prod
  - app/src-tauri/src/commands.rs::*cloud*
  - app/src-tauri/src/lan/mod.rs::reconcile|notices_for|revokes|shared_libraries|only_shared*|register_in_shared
  - app/src-tauri/src/libraries.rs::retire_fixed_library|ensure_first_library|first_library|library_known|forget_here|remove_revoked|revoke_access|create_with_id
  - app/src-tauri/src/ops.rs::active_library
  - app/src-tauri/src/permissions.rs::judged_by_order|fence_mark|fence_changes|note_fences|rebuild|allowed
  - app/src-tauri/src/sync.rs::signature_holds
  - server/src/Controller/CloudController.php
  - server/src/Sync/CloudLibraryAccess.php
  - server/src/Sync/CloudLibraryCreator.php
  - server/src/Sync/CloudPull.php
  - server/src/Sync/CloudPush.php
  - server/src/Sync/OperationSignature.php
  - server/src/Entity/CloudLibrary.php
  - server/src/Entity/Membership.php
  - server/src/Entity/CloudOperation.php
  - server/src/Entity/CloudDeviceKey.php
  - server/src/Entity/Device.php
  - server/src/Repository/CloudLibraryRepository.php
  - server/src/Repository/CloudOperationRepository.php
  - server/src/Repository/CloudDeviceKeyRepository.php
  - server/src/Repository/MembershipRepository.php
context:
  - docs/architecture/cloud.md
  - docs/plans/fase-7.md#7.2 · Detalles cerrados al aceptar D84 (03/10/2026)
  - docs/plans/fase-7.md#Batería de acceso
  - docs/audits/amenazas.md#Acceso a las bibliotecas
decisions: D39 D56 D57 D58 D71 D73 D76 D81 D84 D85
max_lines: 3000
---
Who can end up holding a library's data, on the LAN and through the cloud (milestone 7.2, built). The maintainer's three worries, in this order: **no library data reaches a person or device that was not allowed it**; **local-first never breaks** (the app works fully with no server and no account); **working offline and resyncing never loses or forks data**, with the cloud and the LAN active at the same time (D39).

The design (D84): the data layer is the LAN's (signed operations, merge rules, roles D73); the cloud's door is classic. The server creates its own `cloud_library_id` on upload, keeps a `membership` table, answers 404 when there is no access, and **never trusts an id from the client**: library, operation and device ids in a request are compared, never used to look up or decide access. A signed `library` update (admin only) carries `cloud_library_id` and `cloud_server`; a device takes a library from the cloud only if that link operation was written by a device already registered in its local library (`adopt_link`) or it has no library with that local id. Every operation must be signed, everywhere. Since D85 no library has a fixed id; `retire_fixed_library` moves an old fixed-id library to a random id at startup. The server's code has not been reviewed by the maintainer yet: be thorough there.

Assume a hostile client: a modified app with a valid account that knows other people's library, operation and device ids, signing keys of its own, and speaks the protocol.

1. **The door.** For every endpoint in `CloudController`, trace what decides access. Can an account read, write, list or learn the existence of a cloud library it has no membership in (ids from the URL, the body, an operation's `library_id`, a `device_id`, a key)? Do the responses for "does not exist" and "not yours" differ in status, body or timing in a way that matters? Can one account's upload or operation collide with, overwrite, shadow or block another account's (same local library id, same operation id, same device id with another key)?
2. **Signatures and devices.** Is a device identity id + key everywhere on the server (SEG-016)? Can an operation be accepted under another device's name, replayed into another cloud library, or keep a signature that does not cover what is stored? Is `signature_holds` refusing unsigned operations on every merge path (LAN and cloud)?
3. **The app side.** `upload`, `sync_library`, `sync_all`, `adopt_link`, `download`, `remove_unlinked`: can a library reach the cloud without an admin's choice, or arrive on a device from a cloud library that is not the one linked to it? What happens with a library linked to another server, a library deleted or left here (`deleted_here`), a revoked device, a device given away (D81), signing out, another account signing in on the same device?
4. **Fences and revocations.** `note_fences`, `fence_mark`, `rebuild`, `judged_by_order`: can an operation written after a revocation or a role change get in by arriving late, by the other path (cloud vs LAN), or by backdating its HLC? Can a fence be forged by a non-admin?
5. **Local-first and resync.** Does anything make the app depend on the server (a library that cannot be edited or shared on the LAN while the server is down, after signing out, or after the cloud lost the library)? A device edits offline for a week, then syncs with the cloud and with a LAN peer in either order: duplicates, lost operations, a fork (DAT-001: the cloud rejects one operation and accepts a later one from the same author), two devices linking the same library at the same time.
6. **D85.** `retire_fixed_library` and `ensure_first_library`: can they lose data, run twice with different results, or leave two devices of the same family unable to sync over the LAN? Is the legacy id still meaningful anywhere in production code?

Also check the open findings SEG-012, SEG-013 and DAT-001: say if each is resolved by the code, with the line that does it.

End with `## Propiedades comprobadas`: the guarantees you verified, each with the code that ensures it, and `## Huecos en la batería`: hostile cases from point 1–6 that no test in the «Batería de acceso» covers.
