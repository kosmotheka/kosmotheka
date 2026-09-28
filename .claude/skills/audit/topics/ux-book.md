---
agent: audit-reviewer
code:
  - app/src/components/book-detail.tsx
  - app/src/components/book-form.tsx
  - app/src/components/marks/*.tsx
  - app/src/components/history-list.tsx
  - app/src/components/wish-list.tsx
  - app/src/components/ask-dialog.tsx
decisions: D17 D55 D61 D68 D69
max_lines: 2400
---
The book detail, the form, marks, history, wishes and asking for a change. Look at what the person experiences.

1. Dead ends and empty states; what a reader sees for actions they cannot do (hidden, disabled with a reason, or failing?).
2. Errors not shown or shown generically; raw errors.
3. Destructive actions without confirmation or undo.
4. Editing a book that another device changed or deleted meanwhile.
5. Command errors not handled; writes shown as done before they succeed; effects whose results can arrive out of order (switching book).
6. Accessibility of the main actions: icon buttons without an accessible name, inputs without labels, dialogs without a title.
