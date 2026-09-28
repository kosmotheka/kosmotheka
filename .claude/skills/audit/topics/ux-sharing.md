---
agent: audit-reviewer
code:
  - app/src/pages/library-page.tsx
  - app/src/pages/devices-page.tsx
  - app/src/pages/people-page.tsx
  - app/src/pages/requests-page.tsx
  - app/src/components/pair-dialog.tsx
decisions: D17 D54 D55 D71
max_lines: 2600
---
The screens for sharing: libraries, devices, people and requests. Look at what the person experiences.

1. Dead ends: a dialog or page with no way back, or one that needs a state the person cannot reach from the UI.
2. Empty states: no libraries, no devices, no people, no requests, no connection.
3. Errors: not shown, or shown generically when the code knows the cause; raw errors instead of translated messages.
4. Destructive actions (remove a person, withdraw access, delete a library) without confirmation or undo; actions that look destructive but are not.
5. Long operations (pairing, sync) without progress or without a clear end.
6. Command errors not handled (`await` without `try/catch`, `.then` without `.catch`), and writes shown as done before they succeed.
