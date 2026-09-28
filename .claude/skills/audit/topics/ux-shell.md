---
agent: audit-reviewer
code:
  - app/src/App.tsx
  - app/src/components/shell/*.ts*
  - app/src/components/settings-dialog.tsx
  - app/src/components/backups-dialog.tsx
  - app/src/components/request-notices.tsx
  - app/src/components/libraries-context.tsx
  - app/src/components/requests-context.tsx
decisions: D17 D46
max_lines: 2400
---
The shell: navigation, library picker, sync status, settings and backups.

1. Every action reachable on desktop (sidebar) is reachable on mobile (bottom navigation, drawer) and the other way round (`nav-items.ts`).
2. Dead ends and empty states (no library yet, first run).
3. Sync status: does it tell the truth (in progress, failed, when it last worked)?
4. Backups dialog: errors, progress, and a clear result.
5. Contexts: effects without cleanup, listeners not removed, results out of order when switching library, re-render storms.
6. Accessibility of navigation: keyboard access, accessible names, focus in dialogs.
