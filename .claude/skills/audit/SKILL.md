---
name: audit
description: Run the project audit - code quality, contracts between layers, tests, functional seams, data integrity and security - with specialized read-only agents, and write the report in docs/audits/. Use when the maintainer asks for an audit or types /audit. Arguments - none (incremental since the last audit, or initial if there is none), "initial", or a list of agent names to review in full.
---

# Audit

You are the **coordinator**. Specialized agents look for problems; you pin the commit, run the scanners, launch the agents, **verify every finding against the code**, and write the report in Spanish. The method, for humans, is in `docs/audits/README.md`; keep both in sync.

## Ground rules

- The audit only reads. Never edit code, `docs/DECISIONS.md`, `docs/ROADMAP.md`, `docs/testing/` or anything outside `docs/audits/`. Another session develops the project in parallel, in the same working tree.
- Commit only your own files with explicit paths (`git add docs/audits/<file>`), never `git add -A`, `.` or a folder. Check `git status` first and leave out files you did not write. Never push, never touch branches, stash or worktrees.
- Talk to the maintainer in Spanish. Never message other sessions unless the maintainer asks.

## The agents

| Agent | Model | Temporary prefix | Register prefix |
|---|---|---|---|
| `audit-quality-rust` | light | `RUST` | `CAL` |
| `audit-quality-ts` | light | `TS` | `CAL` |
| `audit-quality-php` | light | `PHP` | `CAL` |
| `audit-contracts` | light | `CON` | `CON` |
| `audit-tests` | light | `QA` | `PRU` |
| `audit-supply-chain` | light | `SUP` | `SEG` |
| `audit-flows` | main | `FLOW` | `COS` |
| `audit-data-integrity` | main | `DATA` | `DAT` |
| `audit-security-design` | main | `SEC-D` | `SEG` |
| `audit-security-server` | main | `SEC-S` | `SEG` |
| `audit-security-client` | main | `SEC-C` | `SEG` |

Their instructions are in `.claude/agents/<name>.md`; the rules they all share are in `.claude/skills/audit/auditor-rules.md`.

## 1 · Decide what to audit

1. `REPO` is the repository root. `COMMIT=$(git -C "$REPO" rev-parse main)`. Uncommitted work belongs to the other session and is never audited.
2. Find the latest report: `ls docs/audits/20*.md | sort | tail -1`. If none exists, or the argument is `initial`, `MODE=initial` and `BASE=none`. Otherwise read `BASE` from its line "Commit auditado" and `MODE=incremental`.
3. If `BASE` equals `COMMIT` and no agent was named in the arguments, tell the maintainer there is nothing new and stop.
4. Incremental only: `git -C "$REPO" diff --name-only "$BASE" "$COMMIT"` gives `CHANGED_FILES`; `git -C "$REPO" log --oneline "$BASE..$COMMIT"` gives the commits, for the report.

## 2 · Snapshot

Use the session's scratchpad directory (`<scratch>`):

```bash
SHORT=${COMMIT:0:8}
AUDIT="<scratch>/audit-$SHORT"; SNAP="$AUDIT/src"; T="$AUDIT/tools"
mkdir -p "$SNAP" "$T"
git -C "$REPO" archive "$COMMIT" | tar -x -C "$SNAP"
```

## 3 · Scanners (Docker only, pinned versions)

Nothing is installed on the machine. Exit code 1 from osv-scanner or gitleaks means "found something", not failure; any other non-zero code is a failure: note it in the report and carry on.

```bash
U="$(id -u):$(id -g)"

# Vulnerable dependencies in Cargo.lock, package-lock.json and composer.lock.
# Sends package names and versions to osv.dev; no code leaves the machine.
docker run --rm --user "$U" -v "$SNAP:/src:ro" -v "$T:/out" ghcr.io/google/osv-scanner:v2.6.0 \
  scan source --recursive --format json --output-file /out/osv.json /src

# Dependency licenses outside a permissive allowlist (license data comes from deps.dev).
docker run --rm --user "$U" -v "$SNAP:/src:ro" -v "$T:/out" ghcr.io/google/osv-scanner:v2.6.0 \
  scan source --recursive \
  --licenses="MIT,Apache-2.0,BSD-2-Clause,BSD-3-Clause,ISC,0BSD,Zlib,Unicode-3.0,Unicode-DFS-2016,CC0-1.0,BSL-1.0,Unlicense" \
  --format json --output-file /out/osv-licenses.json /src

# Secrets in the git history: the real repository, mounted read-only; values redacted.
# Initial: whole history. Incremental: add --log-opts="$BASE..$COMMIT".
docker run --rm --user "$U" -v "$REPO:/repo:ro" -v "$T:/out" \
  -e GIT_CONFIG_COUNT=1 -e GIT_CONFIG_KEY_0=safe.directory -e GIT_CONFIG_VALUE_0='*' \
  ghcr.io/gitleaks/gitleaks:v8.30.1 git /repo --no-banner --redact \
  --report-format json --report-path /out/gitleaks.json

# Static analysis. Rules are downloaded from the Semgrep registry; code stays local.
docker run --rm --user "$U" -e HOME=/tmp -v "$SNAP:/src:ro" -v "$T:/out" semgrep/semgrep:1.177.0 \
  semgrep scan --metrics=off --disable-version-check --quiet \
  --config p/default --config p/owasp-top-ten --config p/security-audit --config p/secrets \
  --config p/php --config p/typescript --config p/react --config p/javascript --config p/rust \
  --config p/dockerfile --config p/docker-compose --config p/github-actions --config p/nginx \
  --exclude docs/historico --exclude docs/research \
  --json --output /out/semgrep.json /src
```

Incremental only: keep in `semgrep.json` only the results in `CHANGED_FILES` (earlier results were triaged in earlier audits):

```bash
git -C "$REPO" diff --name-only "$BASE" "$COMMIT" > "$T/changed.txt"
python3 - "$T/semgrep.json" "$T/changed.txt" <<'EOF'
import json, sys
path, changed = sys.argv[1], set(open(sys.argv[2]).read().split())
data = json.load(open(path))
data["results"] = [r for r in data["results"] if r["path"].removeprefix("/src/") in changed]
json.dump(data, open(path, "w"))
EOF
```

To upgrade a scanner, change its tag here and in `docs/audits/README.md`, and say so in the next report.

## 4 · Choose the agents

- `MODE=initial`: all eleven.
- Agents named in the arguments: those, in `initial` mode (a full review of their scope), plus `audit-supply-chain`.
- `MODE=incremental`: `audit-supply-chain` always, plus every agent whose paths appear in `CHANGED_FILES`:

| Changed path | Agents |
|---|---|
| `crates/domain/**` | quality-rust, contracts, tests, data-integrity, security-design |
| `app/src-tauri/src/lan/**` | quality-rust, tests, data-integrity, security-design, security-client |
| `app/src-tauri/src/{ops,sync,db,backup,csv_io,data_folder,libraries,requests,people}.rs` | quality-rust, contracts, tests, data-integrity, flows, security-client |
| other `app/src-tauri/src/**` | quality-rust, contracts, tests, flows, security-client |
| `app/src-tauri/migrations/**` | data-integrity, contracts, tests |
| `app/src-tauri/{tauri.conf.json,capabilities/**,gen/android/**}` | security-client |
| `app/src/i18n/**` | contracts |
| other `app/src/**` | quality-ts, contracts, flows, security-client |
| `server/src/**`, `server/config/**`, `server/public/**` | quality-php, contracts, tests, security-server |
| `server/tests/**` | tests, quality-php |
| `server/docker/**`, `server/.env*` | security-server |
| `spec/**` | contracts, tests, data-integrity, security-design |
| `docs/DECISIONS.md`, `docs/ROADMAP.md`, `docs/plans/**` | flows, security-design |
| `docs/architecture/**` | contracts, data-integrity |
| `docs/testing/**` | tests |

- **Deep dive**: in every incremental audit, one more agent runs in `initial` mode over its whole scope, so old code keeps being reviewed. Take it from the line "Siguiente a fondo" of the latest report and move the pointer to the next one in this rotation: data-integrity → security-client → security-design → flows → security-server → contracts → tests → quality-rust → quality-ts → quality-php → (back to data-integrity).

## 5 · Launch

Launch every chosen agent **in one message, in the background**, with `subagent_type` set to the agent name. The launch prompt, filled in:

```
Kosmotheka audit.
REPO: <absolute path>
SNAPSHOT: <SNAP>
COMMIT: <COMMIT>
BASE: <BASE or none>
MODE: <initial | incremental>
CHANGED_FILES:
<one path per line, or n/a>
KNOWN_FINDINGS:
<the register rows found by this agent, open, accepted or dismissed: ID | estado | severidad | título | dónde — or none>
TOOLS: <T, or none>
```

`TOOLS` goes to `audit-supply-chain`, `audit-security-server` and `audit-security-client`; the others get `none`.

**Fallback** if the agent types are not available (for example, a session started before the files existed): use `subagent_type: general-purpose`, pass `model` from the agent file's frontmatter (omit it when it says `inherit`), and start the prompt with "Read `<REPO>/.claude/agents/<name>.md` and follow it as your instructions; ignore its frontmatter."

If an agent fails or returns a report without the required sections, relaunch it once. If it fails again, record that in the report's process notes.

## 6 · Verify and consolidate

Nothing goes into the report without passing these checks. This is the most important step: an audit full of false positives is worse than no audit.

1. **Evidence.** For every finding, open the cited lines in `SNAP` (`sed -n`) and check that the quoted code is there. If not, discard it and count it in the process notes.
2. **Duplicates.** Merge findings with the same root cause, across agents and against the register: keep the most precise wording, list all locations, name every perspective that saw it.
3. **`crítica` and `alta`: confirm them yourself.** Read the code path end to end and check that the failure scenario really happens. If it does not, lower it or discard it with the reason. If it depends on runtime behavior, keep it as `probable` or `posible` and add "Cómo comprobarlo": concrete steps the maintainer can follow on screen.
4. **`media` and `baja`**: the evidence check plus a read of the surrounding function. If still in doubt, mark it "por confirmar" instead of dropping it.
5. **Decisions.** If an accepted decision chooses the behavior, the finding becomes `aceptado (D<nn>)` instead of open, unless the decision misses the risk. Anything that needs a new decision goes to "Para decidir"; you never write in `docs/DECISIONS.md`.
6. **Severity** follows the scale in `auditor-rules.md`, the same for every agent.
7. **IDs.** New findings take the next free number of their register prefix: `SEG-001`, `DAT-001`, `COS-001`, `CON-001`, `PRU-001`, `CAL-001`. IDs never change and are never reused.
8. **Known findings.** Use the agents' "Hallazgos conocidos" sections to move register rows to `resuelto` (with the commit that fixed them) or keep them open. An open finding in a scope that no agent reviewed this time stays as it is.

## 7 · Write

All in Spanish, in `docs/audits/`:

- **`AAAA-MM-DD.md`**, the report. Add `-2` if one exists for today. Template:

```markdown
# Auditoría AAAA-MM-DD

- **Commit auditado:** `<COMMIT>` (main)
- **Desde:** `<BASE>` (<n> commits) · o · auditoría inicial
- **Modo:** inicial · o · incremental, a fondo: <agent>
- **Auditores:** <list>
- **Herramientas:** osv-scanner v2.6.0 · gitleaks v8.30.1 · semgrep 1.177.0
- **Siguiente a fondo:** <agent>

## Resumen
Five to eight lines: the overall state, the most serious findings, the trend compared with the previous audit.

## Cifras
| | Crítica | Alta | Media | Baja |
|---|---|---|---|---|
| Nuevos | | | | |
| Abiertos en total | | | | |
Resueltos desde la anterior: <n>. Descartados en esta pasada: <n>.

## Hallazgos nuevos
### SEG-001 · <título>
- **Severidad:** · **Confianza:** · **Perspectiva:** <auditores>
- **Dónde:** `path:line`
- **Qué falla:** …
- **Arreglo propuesto:** …
- **Referencia:** …
- **Cómo comprobarlo:** (only when it needs a check on screen)
(ordered by severity; for `quality-rust` findings keep the "Por qué (Rust)" line)

## Resueltos desde la anterior
## Siguen abiertos
## Descartados en esta pasada
Short title and the reason, so they do not come back.
## Para decidir
## Escenarios de prueba manual propuestos
From `audit-flows`, for the maintainer to move into `docs/testing/` if they want.
## Notas del proceso
Agents relaunched or failed, scanner errors, evidence that did not match, limits.
```

- **`hallazgos.md`**, the register. Create it in the initial audit and update it in every audit. One table, sorted by ID:

```markdown
| ID | Estado | Severidad | Título | Dónde | Auditor | Abierto | Cerrado | Informe |
```

States: `abierto`, `resuelto`, `aceptado (D<nn>)`, `descartado`. The details stay in the report where the finding first appeared; the register links to it.

- **`amenazas.md`**, the threat model: written in the initial audit from `audit-security-design`'s section, updated when that agent proposes changes.

## 8 · Close

1. Tell the maintainer, in Spanish and briefly: the figures, every `crítica` and `alta` in one line each, what needs a decision, and which findings you suggest handing to the development session. The maintainer decides what gets fixed and when.
2. `git status`, then commit only your files: `git add docs/audits/<each file you wrote>` and `git commit -m "Audit AAAA-MM-DD: <n> new findings"`. Never push.
3. Delete the snapshot folder `$AUDIT`.
