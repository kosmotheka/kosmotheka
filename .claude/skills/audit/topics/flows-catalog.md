---
agent: audit-reviewer
code:
  - app/src-tauri/src/catalog.rs::@prod
  - app/src-tauri/src/marks.rs::@prod
  - app/src-tauri/src/wishes.rs::@prod
  - app/src-tauri/src/search.rs::@prod
  - app/src-tauri/src/libraries.rs::*move*|*copy*|*share*
decisions: D24 D28 D33 D46 D60 D61 D65 D68 D69
max_lines: 2400
---
Seams between catalog features. For each flow, trace the real code path and say what happens to the person.

1. **Moving a book between libraries**: what devices that receive only one of the two libraries see; copies, marks and wishes attached to it.
2. **Marks per person** (D68, D69): marks on a book that is deleted, moved or merged as a duplicate (D33); marks of a member who is removed; the hidden gift reservation (D69) really hidden where it must be.
3. **Wishes × catalog** (D61): adding a book that is on the wishlist; the same wish from two members.
4. **Quick add** (D28): it must not duplicate what already exists; the same ISBN added on two devices offline.
5. **Search**: deleted books in results; results from a library the person cannot see; the search index kept up to date after sync.
6. **Works, editions and copies** (D24): deleting the last copy, the last edition, or a work with copies on other devices.

Add a final section `## Escenarios de prueba manual propuestos` for the `crítica` and `alta` findings, in Spanish, in the style of `docs/testing/`.
