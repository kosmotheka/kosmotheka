---
name: record-decision
description: Add or update an architecture decision in docs/DECISIONS.md. Use when a technical or product decision is proposed, accepted, reopened or replaced during a conversation.
---

# Record a decision

`docs/DECISIONS.md` is the project's decision log. It is written **in Spanish**, like everything in `docs/`.

## Statuses

`abierta` (under discussion) · `propuesta` (recommendation awaiting approval) · `aceptada` · `reemplazada por Dxx`.

Only mark a decision `aceptada` when the maintainer has explicitly agreed. Never delete a decision: mark it replaced and link the new one, so the reasoning is kept.

## Steps

1. Read `docs/DECISIONS.md` to find the next free number and any related decisions.
2. Add or update the row in the **Resumen** table at the top.
3. Add or update the section, keeping numeric order, with this heading format:
   `## Dnn · Short title — \`status\``
4. Write the body with the parts that apply:
   - **Contexto**: the problem and what forces the decision.
   - **Opciones**: a table with each option and its pros and cons.
   - **Recomendación** or **Decisión**: what was chosen and why.
   - **Consecuencias**: what this changes elsewhere; reference the affected decisions (Dxx).
5. If the decision affects the plan, update `docs/ROADMAP.md`.
6. If it changes conventions for agents (stack, layout, rules), update `AGENTS.md`.
7. Tell the maintainer, in Spanish, which decision was added or changed.
