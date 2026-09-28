## Instructions

You review **one topic** of the Kosmotheka audit. Kosmotheka is a local-first household library app: a Rust core with SQLite on each device, a React UI in a Tauri webview, devices that pair by QR and sync signed operations over the LAN, roles per shared library, and an optional Symfony server. Everything you need is in this packet: the questions, the conventions and decisions that apply, mechanical facts, and the exact code with line numbers.

1. **Work from this packet.** Answer every question from the code shown. Do not explore the repository.
2. **Follow-ups: at most 8 commands**, only to confirm or refute a concrete hypothesis (a caller, a definition, a test). Use `grep -n` or `sed -n 'A,Bp'` on files under SNAPSHOT, starting each command with `cd "<SNAPSHOT>" && `, or `Read` with offset and limit. Never list or read whole folders.
3. **Read-only.** Never modify files; never run builds, package managers, network tools, docker or git.
4. **Decisions are the project's intent.** Behavior an accepted decision chooses is not a finding, unless the code contradicts the decision or it carries a data-loss or security risk the decision does not mention; then write "D<nn> no contempla …".
5. **No quote, no finding.** Every finding quotes the exact code with its path and line numbers. "X is missing" needs proof: say which code you checked.
6. **One root cause, one finding.** At most 12 findings; keep the most severe.
7. Anything outside the topic: one line under "Fuera de este tema".

**Severity.** Judge by what can actually happen. If you hesitate between two levels, pick the lower one and say what would raise it.
- `crítica`: in normal use, or for a realistic attacker: data loss or corruption the user cannot undo; devices that never converge again; someone reads or writes what their role or their revoked device must not reach; a real secret exposed.
- `alta`: a feature or guarantee (convergence, permissions, retirada de acceso, backups, signatures) breaks in a realistic scenario; or a security weakness with a plausible precondition: same LAN, stolen unlocked phone, a member acting in bad faith, a modified app on a paired device.
- `media`: edge cases, degraded behavior, missing defense in depth, important rules without tests, debt that will cause bugs soon.
- `baja`: clarity, naming, small inconsistencies.

**Confidence.** `confirmada` (you traced the path in the code) · `probable` (one step depends on runtime behavior) · `posible` (a suspicion; never `crítica` or `alta`).

**Output.** In Spanish (code, identifiers and paths as they are), and nothing else:

```
# <topic> · <commit>
## Resumen
Two or three sentences.
## Hallazgos
### <n>. <title, 80 characters or fewer>
- **Severidad:** … · **Confianza:** …
- **Dónde:** `path:line`
- **Evidencia:** the quoted code, 10 lines or fewer
- **Qué falla:** a concrete scenario: who does what, in which state, and the wrong result
- **Arreglo propuesto:** the smallest change that fixes it
- **Referencia:** AGENTS.md rule, D<nn>, spec rule, CWE or OWASP id, or —
## Comprobado y correcto
One line per question that has no finding, naming the code that makes it correct.
## Fuera de este tema
One line each, or —.
```

Put any extra section a question asks for after "Fuera de este tema".
