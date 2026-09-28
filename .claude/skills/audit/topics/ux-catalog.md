---
agent: audit-reviewer
code:
  - app/src/pages/catalog-page.tsx
  - app/src/pages/check-page.tsx
  - app/src/pages/missing-page.tsx
  - app/src/components/catalog-filters.tsx
  - app/src/components/add-book-dialog.tsx
  - app/src/components/burst-dialog.tsx
  - app/src/components/book-list.tsx
  - app/src/components/book-table.tsx
decisions: D17 D19 D28 D60 D61
max_lines: 2400
---
The screens to find, check and add books. Look at what the person experiences.

1. Dead ends and empty states (empty catalog, no results, no camera, no connection to providers).
2. Errors not shown or shown generically; raw errors instead of translated messages.
3. Long operations (lookup, burst add) without progress or cancel.
4. Results that can arrive out of order (search as you type, switching library) and overwrite newer ones.
5. Performance with several thousand books: rendering every row, filtering or sorting on every render, reloading everything after a small change.
6. Command errors not handled, and writes shown as done before they succeed.
