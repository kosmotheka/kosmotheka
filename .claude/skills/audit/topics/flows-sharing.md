---
agent: audit-reviewer-strong
code:
  - app/src-tauri/src/requests.rs::@prod
  - app/src-tauri/src/people.rs::@prod
  - app/src-tauri/src/permissions.rs::@prod
  - app/src-tauri/src/libraries.rs::@prod
context:
  - docs/testing/fase-5.md
decisions: D45 D47 D54 D55 D58 D65 D67 D71 D73
max_lines: 2400
---
Seams where sharing features meet. For each flow, trace the real code path and say what happens to the person.

1. **Roles × offline work**: an editor edits offline and is lowered to reader before syncing; an admin lowers the only other admin; the last admin leaves (D58); what the person whose changes are rejected sees (D47).
2. **Change requests × everything**: a request on a copy deleted meanwhile; accepting a request whose target changed after it was made; a request from a device later withdrawn; two editors accepting the same request on two devices.
3. **Retirada de acceso × LAN** (D71): a withdrawn device still on the same network; pairing it again; a withdrawal made on a device that does not sync for a week.
4. **People and members × devices**: deleting a member who owns copies, has lent books or made requests; a member linked to a lost device.
5. **Libraries**: deleting or leaving a shared library (D65) while others still use it; share targets changed while a peer is offline.

Add a final section `## Escenarios de prueba manual propuestos`: one per `crítica` or `alta` finding and one per flow that works but has no check in `fase-5.md`. In Spanish, in the style of `docs/testing/`: title, preconditions (devices, roles, data), numbered steps, expected result.
