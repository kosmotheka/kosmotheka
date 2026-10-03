---
name: audit
description: Run the project audit within a fixed budget - scripts do the mechanical work, small agents judge prepared packets, and the report goes to docs/audits/. Use when the maintainer types /audit. Arguments - none (incremental since the last audit, or the first pass if there is none), topic names to review in full, or "presupuesto alto" to double the budget.
effort: medium
---

# Audit

You are the **coordinator**. You orchestrate; nobody explores the repository:

1. **Scripts** do everything mechanical, for free: snapshot, scanners, facts, and the context packets.
2. **You** turn mechanical facts into findings and verify what agents report.
3. **Agents** only judge the one packet you give them: the exact code, questions and context, already cut to size.

Keep your own context small: read the facts summary, the sections it flags and the agents' answers. Read code only to verify a `crítica` or `alta` finding.

The method, for humans, is in `docs/audits/README.md`; keep both in sync.

## Ground rules

- **Only when no other session is working.** They share the usage quota. If the maintainer has not said so in the request, ask before step 1.
- If this conversation already holds a lot of unrelated context, suggest `/clear` before starting: each of your turns re-sends it.
- The audit only reads. Write only in `docs/audits/`. Commit only your files with explicit paths; never `git add -A`, `.` or a folder; never push.
- Talk to the maintainer in Spanish.

## Budget per run

| | Default | "presupuesto alto" |
|---|---|---|
| Review agents | 2, at most 1 `audit-reviewer-strong` | 4, at most 2 strong |
| Triage packets | 3 | 6 |

Never relaunch an agent to improve an answer. Relaunch once only if it failed or broke the output format.

## 1 · Prepare (scripts, no model)

```bash
REPO=$(git rev-parse --show-toplevel)
W=<scratchpad>/audit-work
BASE=$(ls "$REPO"/docs/audits/20*.md 2>/dev/null | sort | tail -1 | xargs -r grep -m1 -o 'Commit auditado:\*\* `[0-9a-f]*' | grep -o '[0-9a-f]*$')
"$REPO/.claude/skills/audit/bin/prepare.sh" "$REPO" "$W" $BASE
```

This takes about 15 seconds. It runs the scanners and the facts script in Docker with pinned images. Then:
- If the printed `COMMIT` equals `BASE` and no topic was requested, say there is nothing new and stop.
- Read **only** `$W/facts/SUMMARY.md`.

## 2 · Mechanical findings (you, no agents)

Open only the sections the summary flags (⚠ or non-zero counts), with `sed -n '/^## Heading/,/^## /p' "$W/facts/<file>"`:

- `contracts.md`:
  - schema-chain differences, commands invoked but not registered, argument mismatches, keys used but missing in a language, conformance files not run by both Rust and PHP: `CON` findings, usually `alta`;
  - **never dismiss a mechanical difference as a script error without comparing the real sources yourself** (in the first audit, a stale genre list in the JSON Schema was dismissed that way);
  - closed lists and error codes (`bin/contracts.json`): a value missing in TypeScript or without a translation is `CON` `media`; an error code the UI cannot show specifically (account `switch`) is `baja`. If a list or family cannot be read, fix `bin/contracts.json`, not the finding;
  - Rust structs and TS interfaces with different fields: `CON`, `alta` when the UI reads a field Rust no longer sends, otherwise `media`;
  - commands never invoked, keys only in one language, different variables: `media`;
  - unused keys: one `baja` finding with the list.
- `tests.md`: ignored tests not named `live_*` or `print_*`: `PRU`, `baja`.
- `scanners.md`, «Dead code (knip)»: an unused export in `lib/api.ts` is a command with no screen (`COS` `media`: check the feature works end to end); anything else is `CAL` `baja`, grouped in one finding.
- `candidates.tsv` categories `*.todo`, `*.long-fn`, `*.big-file`: not findings; count them in the report's process notes.

## 3 · Choose the work

**First pass** (no `BASE`):
- Topics: `threat-model` and `supply-chain`.
- Triage, in this order:
  1. `rust.panic,rust.sql-format,rust.sql-write-catalog`
  2. `ts.sink,ts.mock-ipc-import,ts.ipc-outside-api,ts.listen,ts.effect`
  3. `semgrep`
- Record in the report, under "Pendiente", the triage groups left for later runs:
  1. `rust.swallow`
  2. `rust.sql-write-other`
  3. `ts.role-check`
  4. `ts.hardcoded-*,ts.any,ts.index-key,ts.console,ts.non-english,rust.non-english,rust.cast,rust.allow,rust.unsafe,php.*`

**Next runs** (`BASE` set):
1. **Triage of what changed:** `python3 "$REPO/.claude/skills/audit/bin/packet.py" triage "$W" "rust.*,ts.*,php.*,semgrep" --incremental`. Exit code 3 means there are no candidates.
2. **Pending:** one pending triage group from the last report, if any.
3. **Touched topics:** for each topic, `python3 "$REPO/.claude/skills/audit/bin/packet.py" review "$W" <topic> --incremental`. Exit code 0 means its code changed. Keep the first by rotation order.
4. **Deep dive:** the topic after "Siguiente a fondo" in the last report, in full mode, even if nothing changed there.
5. **Documentation:** if `docs/DECISIONS.md`, `docs/ROADMAP.md` or `docs/plans/` changed and no topic was touched, the touched topic is `docs-vs-code`.
6. **Supply chain:** add `supply-chain` when lockfiles, manifests, CI, Docker or `.env*` changed, or the scanners show something new. It counts against the review budget.

**Rotation and risk order:**
1. sync-convergence
2. library-access
3. permissions-on-receipt
4. lan-pairing
5. ipc-surface
6. data-files
7. migrations
8. signatures
9. flows-sharing
10. flows-catalog
11. webview-rendering
12. server
13. metadata-covers
14. spec-coverage
15. tests-quality
16. ux-sharing
17. ux-catalog
18. ux-book
19. ux-shell
20. ux-wording
21. quality-rust
22. quality-ts
23. docs-vs-code
24. supply-chain

After the last one, start again at the first. `threat-model` runs again only when security decisions change. `library-access` also runs, outside the rotation, whenever who receives a library changes: at the end of milestones 7.2, 7.5 and 7.6.

Topic names given as arguments replace the deep dive, in full mode, within the budget.

## 4 · Build the packets and launch

```bash
python3 "$REPO/.claude/skills/audit/bin/packet.py" review "$W" <topic> [--incremental]
python3 "$REPO/.claude/skills/audit/bin/packet.py" triage "$W" "<categories>" [--incremental]
```

- Each command prints the packet path and its size in lines.
- The topic file's header names its agent: `audit-reviewer` or `audit-reviewer-strong`. Triage packets go to `audit-triage`, 40 candidates each.
- **Never read a packet yourself.**
- If a packet warns it is over its limit but stays under 3,000 lines, launch it anyway. If it is larger, use `--incremental` or tell the maintainer.
- Launch the agents in the background, with `Packet: <path>` as the whole prompt: all in one message, or **in waves** when the maintainer wants to measure the usage. Waves, in order of cost: the triage packets and the `audit-reviewer` topics first, then the `audit-reviewer-strong` topic alone. Stop after each wave and report each agent's tokens.
- Each triage candidate comes with its whole function when it is under 60 lines; otherwise ±6 lines. Check the cited code yourself before accepting a «no concluyente».

## 5 · Verify and consolidate

An audit full of false positives is worse than none.

- **Triage:** accept `media` and `baja` rows as they are; "No concluyentes" become "por confirmar". For `crítica` and `alta`, read the cited lines ±15 in `$W/src` and confirm.
- **Reviews:**
  - For every `crítica` and `alta`, find the quoted code with `grep -n` and read ±20 lines. Confirm the scenario, or lower or dismiss it with the reason. If it depends on runtime behavior, keep it as `probable` and add "Cómo comprobarlo" with concrete steps on screen.
  - For `media` and `baja`, one `grep -n` to check that the quote exists.
- **Duplicates:** check across agents and against `docs/audits/hallazgos.md`.
- **Decisions:** behavior an accepted decision chooses becomes `aceptado (D<nn>)`. Anything needing a new decision goes to "Para decidir"; never write in `docs/DECISIONS.md`.
- **Open findings in changed files:** if their quoted code is gone, mark them `resuelto` with the commit range.
- **IDs:** the next free number per prefix, never reused:

| Prefix | For |
|---|---|
| `SEG` | Security: threat-model, library-access, permissions-on-receipt, lan-pairing, signatures, ipc-surface, webview-rendering, metadata-covers, server, supply-chain; triage `ts.sink`, `ts.mock-ipc-import`, `rust.sql-format`, `semgrep`, and `rust.panic` fed by peers |
| `DAT` | Data integrity: sync-convergence, data-files, migrations; triage `rust.swallow` that can lose data |
| `COS` | Seams: flows-*, ux-*, docs-vs-code; triage `ts.effect` |
| `CON` | Contracts: spec-coverage and the mechanical findings of step 2 |
| `PRU` | Tests: tests-quality |
| `CAL` | Quality: quality-rust, quality-ts, and every other triage category |

## 6 · Write (Spanish, short)

- **`docs/audits/AAAA-MM-DD.md`** (add `-2` if one exists for today):
  - header: its first bullet is exactly ``- **Commit auditado:** `<full sha>` `` (step 1 reads it); then base and number of commits, topics and triage groups run, agents with packet sizes, "Pendiente", "Siguiente a fondo";
  - "Resumen" in five lines;
  - a figures table (new and open, by severity);
  - "Hallazgos nuevos", 8 lines each at most, ordered by severity;
  - "Resueltos", "Siguen abiertos" (IDs only), "Descartados" (one line each, with the reason);
  - "Para decidir", "Escenarios de prueba manual propuestos", "Notas del proceso".
- **`docs/audits/hallazgos.md`**: one table sorted by ID with the columns `ID · Estado · Severidad · Título · Dónde · Auditor · Abierto · Cerrado · Informe`. States: `abierto`, `resuelto`, `aceptado (D<nn>)`, `descartado`.
- **`docs/audits/amenazas.md`**: from the "Modelo de amenazas" section of `threat-model`.

## 7 · Close

1. Tell the maintainer, briefly: the figures, each `crítica` and `alta` in one line, what needs a decision, and which findings you suggest for the development session. They decide what gets fixed and when.
2. Add to `docs/audits/descartes.md` one row per triage candidate you verified is not a problem (category, path, function and code exactly as in `candidates.tsv`, the reason, and the audited commit). Never add rows for something you did not check yourself.
3. `git status`, then `git add docs/audits/<each file you wrote>` and `git commit -m "Audit AAAA-MM-DD: <n> new findings"`. Never push.
4. `rm -rf "$W"`.
