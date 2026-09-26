# Draft: roles and permissions (phase 5)

**Not part of the spec yet.** These cases describe how devices are proposed to check permissions without a server (D55 in `docs/DECISIONS.md`, plan in `docs/plans/fase-5.md`). No implementation loads them. They move to `conformance/` once the maintainer has reviewed them.

## Proposed rules

1. **A library without roles works as before:** until some member has a role, every device may do everything. The first role is assigned under that same rule, and from then on the library has roles.
2. **Who wrote an operation** is the device in its `hlc` (signed, D56). Its **person** is the member that device's `device` row points to (`member_id`), and its **role** is that member's `role` at the moment of the operation. A device linked to no member, or an operation without a signature, counts as a **reader**.
3. **Roles** (a `role` field on `member`: `admin`, `editor` or `reader`, and `can_share`, true or false):
   - **admin** may do everything, including roles, who owns each device, access (`revocation`) and deleting the library.
   - **editor** creates and edits the catalog. It deletes only rows its person created (D45): a row whose create change came from a device of the same person.
   - **reader** changes nothing in the catalog: what it tries becomes a request.
   - **Nobody gives more than they have:** changing `role`, `can_share` or a device's `member_id` is for administrators.
4. **Every device registers and renames itself**, whatever its role: its own `device` row without `member_id`.
5. **All or nothing:** if any change in an operation is not allowed, none is applied and the whole operation becomes a request.
6. **Roles as they were at that moment:** each operation is judged against the roles in force at its HLC, following the log in HLC order. When an operation arrives with an HLC older than what a device has already applied, and it changes roles, the device re-evaluates the library from its log. So every delivery order ends in the same state.
7. **Nothing is lost:** an operation that is not allowed stays in the log, and travels on to other devices like any other, as a request that administrators accept or reject (milestone 5.5).

## Case format

`permissions.json` follows `conformance/library-merge.json`: `operations`, then `deliveries` (orders in which to deliver them, as indexes), then `expect`. Here `expect` lists which operations are **applied** and which become **requests**; the rows that result follow from the applied ones and the merge rules. Every order must give the same lists.

Signatures are left out to keep the cases readable: every operation is assumed correctly signed by the device in its `hlc`, except those marked `"unsigned": true`. All devices register the same test key, which is fine because signatures are not what these cases check.

The fields `role`, `can_share` and `member_id` do not exist in the schema yet; they are part of what is proposed. `can_share` needs a boolean field type, which the schema does not have either.
