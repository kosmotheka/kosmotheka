# Rules for audit agents

Every `audit-*` agent reads this file before doing anything else. If your own instructions and this file disagree, this file wins.

## Inputs

The coordinator's launch prompt gives you these values. If one is missing, stop and say so in your report instead of guessing.

| Input | Meaning |
|---|---|
| `REPO` | The live repository. Use it **only** for read-only git commands with explicit commits: `git -C "$REPO" log`, `show`, `diff`, `ls-tree`, `blame`, `grep`. |
| `SNAPSHOT` | A copy of the audited commit. **Read all code and docs from here**, never from `REPO`. Every path you report is relative to `SNAPSHOT`. |
| `COMMIT` | The audited commit. |
| `BASE` | The previously audited commit, or `none` in an initial audit. |
| `MODE` | `initial`: audit your whole scope. `incremental`: audit what changed in `BASE..COMMIT` inside your scope, plus the code that calls or is called by it. |
| `CHANGED_FILES` | Files changed in `BASE..COMMIT` (incremental only). |
| `KNOWN_FINDINGS` | Open, accepted and dismissed findings for your perspective, with their IDs and locations. |
| `TOOLS` | Paths to tool outputs you must triage, or `none`. |

## Hard rules

1. **Read-only.** Never create, edit or delete files. Never run git commands that change anything (`checkout`, `switch`, `commit`, `add`, `stash`, `reset`, `restore`, `fetch`, `pull`, `push`, `worktree`, `gc`).
2. **Never read code from the live working tree.** Another Claude session is editing it right now; its half-finished work is not what we audit. Use `SNAPSHOT`. Start every shell command with `cd "<SNAPSHOT>" && `: the commands in your instructions use paths relative to it, and the shell may not keep the directory between calls.
3. **No builds, package managers, network or containers.** Never run `cargo`, `npm`, `npx`, `node_modules/.bin/*`, `composer`, `php`, `make`, `docker`, `curl` or `wget`. CI already builds, formats, lints and tests every commit (`.github/workflows/ci.yml`). You may use `grep`, `find`, `sed -n`, `wc`, `sort`, `uniq`, `diff`, `awk` and `python3` (for parsing JSON only).
4. **Skip these paths** unless your own instructions list them: `docs/historico/` (superseded), `docs/research/`, `app/src-tauri/gen/android/` (generated), `app/src/components/ui/` (generated shadcn/ui components), lockfiles, `brand/`, `ds-bundle/`, `.design-sync/`, `.ds-sync/`, images.
5. **Decisions are the project's intent.** Before you report that a behavior is wrong, search `docs/DECISIONS.md` for a decision about it (`grep -n -i "<keyword>" docs/DECISIONS.md`, several keywords, in Spanish: the docs are in Spanish). If an accepted decision chooses that behavior, it is not a finding. Report it only if the code contradicts the decision, or if it carries a data-loss or security risk the decision does not mention; then write "D<nn> no contempla …".
6. **Do not report what CI enforces**: formatting, clippy warnings, ESLint rules, TypeScript errors, PHPStan errors, php-cs-fixer, Rector.
7. **Do not re-report `KNOWN_FINDINGS`.** For each open one inside your scope, say in the "Hallazgos conocidos" section whether it is still present at `COMMIT` (quote the line) or fixed (name the commit or the new code). Never report a dismissed finding again unless the code around it changed; then explain what changed.
8. **Never invent.** Every finding quotes the exact code from `SNAPSHOT`, with its path and line numbers. Get them with `grep -n` or `sed -n '<from>,<to>p'`. If you cannot quote it, you cannot report it.
9. **"X is missing" needs proof.** List the searches you ran (the exact `grep` commands) and the files you read.
10. **One root cause, one finding.** If the same mistake appears in ten places, write one finding and list all the locations.
11. **Stay in your perspective.** If you notice something serious that belongs to another auditor, write one line under "Fuera de mi perspectiva" and move on.
12. **At most 25 findings.** If there are more, keep the most severe ones and say how many more you saw and of what kind.
13. **Keep the report under 5,000 words.** Evidence snippets of 15 lines or fewer.

## Severity

Pick the level from what can actually happen, not from how ugly the code is. If you hesitate between two levels, pick the lower one and write what would raise it.

| Level | Use it when |
|---|---|
| `crítica` | In normal use, or for an attacker in `docs/audits/amenazas.md` (if it exists), one of these happens: data loss or corruption the user cannot undo; devices that never converge again; someone reads or writes data their role or device must not reach; a real secret is in the repository or its history. |
| `alta` | A feature or a guarantee (convergence, permissions, retirada de acceso, backups, signatures) breaks in a realistic scenario. Or a security weakness that needs a plausible precondition: being on the same LAN, a stolen unlocked phone, a library member acting in bad faith. Or an AGENTS.md rule broken in a way that causes bugs, such as a direct `INSERT` into a catalog table. |
| `media` | Edge cases, degraded behavior, missing defense in depth, important rules without tests, or debt that will cause bugs in the next milestones. |
| `baja` | Readability, naming, small inconsistencies and cleanups. |

## Confidence

| Level | Meaning |
|---|---|
| `confirmada` | You traced the path through the code and the evidence proves the failure. |
| `probable` | The code strongly suggests it, but one step depends on runtime behavior (operating system, a library's internals, timing). |
| `posible` | A suspicion worth checking. A `posible` finding can never be `crítica` or `alta`: lower it and write how to confirm it. |

## Output

Write the report **in Spanish**. Identifiers, code and paths stay as they are. Return it as your final message, with these sections in this order and nothing before the title. If your own instructions add sections, put them at the end.

````markdown
# <agent name> · <first 8 characters of COMMIT>

## Resumen
Two to four sentences: what you checked and the most important result.

## Hallazgos

### <PREFIX>-<n>. <Short title, 80 characters or fewer>
- **Severidad:** crítica | alta | media | baja
- **Confianza:** confirmada | probable | posible
- **Dónde:** `path/to/file.rs:120` (one per line if there are several)
- **Evidencia:**
  ```rust
  exact code copied from SNAPSHOT
  ```
- **Qué falla:** A concrete scenario: who does what, in which state, and the wrong result. "Could be a problem" is not a scenario.
- **Arreglo propuesto:** The smallest change that fixes it.
- **Referencia:** The AGENTS.md rule, decision (D<nn>), spec rule, CWE or OWASP id, or "—".

## Hallazgos conocidos
- `<ID>`: sigue | resuelto — one line of evidence.
(or "—")

## Cobertura
| Punto | Resultado | Cómo lo comprobé |
|---|---|---|
| <checklist id> <name> | hallazgos <PREFIX>-1, <PREFIX>-4 · o · sin hallazgos | the searches you ran and files you read |

Every item of your checklist gets a row, including the ones with no findings.

## Fuera de mi perspectiva
- One line each, or "—".
````

`<PREFIX>` is given in your own instructions. Number your findings from 1; the coordinator assigns the final IDs.
