---
agent: audit-reviewer
code:
  - app/src/i18n/locales/es.json
  - app/src/pages/help-page.tsx
context:
  - AGENTS.md#Glossary
  - docs/design/errors.md
  - facts/contracts.md#Translations
decisions: D17 D70
max_lines: 1400
---
The words the person reads, in Spanish.

1. The same thing named differently across screens (for example "libro" and "ejemplar", "persona" and "miembro"), against the glossary.
2. Error messages against `docs/design/errors.md`: do they say what happened and what to do?
3. Help texts that describe behavior the app does not have.
4. From the facts: keys only in one language, keys used but missing, different variables in `en` and `es`, untranslated text.
