---
agent: audit-reviewer
code:
  - app/index.html
  - app/vite.config.ts
  - app/src/main.tsx
  - app/src/components/book-detail.tsx
  - app/src/components/cover.tsx
  - app/src/components/book-list.tsx
  - app/src/components/book-table.tsx
  - app/src/components/history-list.tsx
  - app/src/components/wish-list.tsx
  - app/src/lib/sync-errors.ts
decisions: D17 D27
max_lines: 2200
---
Untrusted data rendered in the webview. Untrusted sources: metadata providers (titles, descriptions, authors, cover URLs), data synced from other devices and members, imported CSV files.

1. Sinks: `dangerouslySetInnerHTML`, `innerHTML`, `href` or `src` built from data (`javascript:` URLs), `window.open`, Markdown or HTML renderers. React escapes text by default: report only real sinks. Because of IPC, an XSS here is `crítica`.
2. Covers: how the image source is built (`convertFileSrc`, remote URLs?); can a synced value make the webview load a remote or local-network URL?
3. Errors shown to people: raw errors (`String(e)`, `e.message`) instead of translated messages through `sync-errors.ts`.
4. The development IPC mock: can it load in a production build (`main.tsx`, `vite.config.ts`)?
