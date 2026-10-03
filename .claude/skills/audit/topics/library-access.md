---
agent: audit-reviewer-strong
code:
  - app/src-tauri/src/lan/mod.rs::save_client|is_known_client|client_is_authorized|shared_libraries|share_library|devices_sharing|revocations_of|notices_for|reconcile|register_in_shared|only_shared*|forget_device
  - app/src-tauri/src/lan/server.rs::@prod
  - app/src-tauri/src/lan/client.rs::sync|sync_once|pair
  - app/src-tauri/src/lan/service.rs::open_pairing|start_if_paired|reset
  - app/src-tauri/src/lan/pairing.rs::@prod
  - app/src-tauri/src/libraries.rs::create_with_id|delete_library|leave_library|remove_revoked|revoke_access|grant_access|library_exists|duplicate*
  - app/src-tauri/src/commands.rs::*pair*|*sharing*|sync_with|*library*
  - app/src-tauri/src/ops.rs::active_library|ensure_registered|device_id
  - app/src-tauri/src/device_reset.rs::@prod
context:
  - docs/architecture/sync.md
  - docs/plans/fase-7.md#7.2 Sincronizar con la nube
  - docs/plans/fase-7.md#7.5 La cuenta en la biblioteca
  - docs/plans/fase-7.md#7.6 Invitar con un enlace
decisions: D39 D46 D54 D58 D67 D71 D73 D76 D81
max_lines: 3000
---
Who can end up holding a library's data, today on the LAN and tomorrow through the cloud. The maintainer's three worries, in this order: **no library data reaches a person or device that was not allowed it**; **local-first never breaks** (the app works fully with no server and no account); **working offline and resyncing never loses or forks data**, with the cloud and the LAN active at the same time (D39).

The key fact: migration `0009_libraries.sql` (line 13, `INSERT INTO libraries (id, name) VALUES ('00000000-0000-7000-8000-000000000001', '')`, run on every install) gives the first library the fixed id `00000000-0000-7000-8000-000000000001` (`ops::DEFAULT_LIBRARY_ID`) on **every** install, new ones included (D46). Library ids are the only thing that says which data belongs together.

1. **LAN today.** Trace every path by which a library's operations leave a device (`open_pairing`, `pair`, `reconcile`, `pull`, `push`, `set_sharing`, `grant_access`). With concrete timelines (devices A, B, C; who marks what), can a library reach a device nobody chose for it? Can two unrelated catalogs merge because of the fixed id, and what exactly must the user do for that to happen? Can a revoked or reset device (D71, D81) keep receiving or get a library back?
2. **The cloud as planned** (plan sections in the context; the server does not exist yet beyond accounts). Assume a hostile client that knows library ids and speaks the protocol. Find every way it could pull a library it is not allowed: claiming an id, the fixed id, uploading a library it got over the LAN as a reader, a device handed to someone else (D81), a device signed into another account, a revoked device, a deleted or left library, account deletion.
3. **The proposal under review** (not in the code yet; judge it):
   - (a) On the server, a library id belongs to the account that uploaded it first. Another account uploading the same id gets `library_taken`; nothing is ever merged across accounts. Until milestone 7.5 the server delivers a library only to the devices of that account; from 7.5, only to devices of accounts linked to a member of the library (D76) and not revoked.
   - (b) A new migration gives a random id to the fixed-id library when it is "empty" (no operations and no rows), so new installs stop sharing that id. Installs with data keep it, so the LAN between existing family devices is unchanged.
   - (c) An existing install whose fixed-id library is `library_taken` in the cloud is offered "Duplicate as new" (D58) and uploads the copy.

   For each part: does it close the leaks found in point 2? What does it break? In (b), define "empty" precisely against the real tables (`library_peers`, `leave_notices`, `revocations`, `local_state` with the active library, a device that paired but has not synced yet, a library received from a peer, the `sync.baseline` operation). Who gets ownership when two of the maintainer's family devices hold the same fixed-id library and both sign in? What can a member who is only a reader do with (a)? What is still missing on the server for (a) to be safe before 7.4 (upload permission, roles), and is "the app checks it" acceptable?
4. **Local-first and resync.** Does anything in the plan or the proposal make the app depend on the server (a library that cannot be edited, shared or synced over the LAN while the server is down, or after signing out)? A device edits offline for a week, then syncs with the cloud and with a LAN peer in either order: duplicate operations, vector versions across two paths, a revocation or a leave that arrives by one path only, a library re-keyed by (b) on one device while another still uses the old id.

End with `## Veredicto sobre la propuesta`: accept, accept with changes (list them) or reject, in five lines at most. Then `## Propiedades comprobadas`: the guarantees you verified, each with the code that ensures it.
