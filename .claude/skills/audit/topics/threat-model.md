---
agent: audit-reviewer-strong
code:
  - crates/domain/src/signature.rs::@prod
  - crates/domain/src/permission.rs::@prod
  - app/src-tauri/src/lan/identity.rs::@prod
  - app/src-tauri/src/lan/pairing.rs::@prod
context:
  - docs/architecture/sync.md
  - docs/architecture/operations.md
  - docs/development/server.md
  - facts/surface.md#tauri.conf.json
  - facts/surface.md#Capabilities
  - facts/surface.md#AndroidManifest.xml
decisions: D05 D09 D13 D34 D37 D38 D43 D47 D49 D50 D51 D54 D55 D56 D57 D58 D62 D66 D67 D71 D73
max_lines: 2600
---
Write the threat model of Kosmotheka from the design (decisions and architecture) and check it against the code shown. This topic produces a document more than findings.

1. **Activos**: the catalog; personal data (members, people who borrow, accounts, emails, device names); device keys; account credentials and tokens; backups and exports; the server database.
2. **Atacantes**, each with what they have and what they want: A1 a library member with the reader role acting in bad faith; A2 an editor who wants more power; A3 a former member or a device whose access was withdrawn and keeps its copy; A4 a device on the same LAN that was never paired; A5 someone with a stolen phone, locked and unlocked; A6 an internet attacker against a self-hosted server; A7 a hostile or tampered metadata provider response; A8 a malicious dependency; A9 the person who runs the server.
3. **Fronteras de confianza**: webview ↔ Rust core (IPC); device ↔ device (LAN); device ↔ server; app ↔ metadata providers; server ↔ database; repository ↔ CI.
4. **STRIDE por frontera**: for each boundary, the threats that apply, the control that stops each one (file and line, or decision), or "sin control".
5. **Riesgos aceptados**: risks a decision accepts on purpose, naming the decision.

Report as findings only the gaps where the code contradicts a decision or a threat has no control and no decision accepts it. Add a final section `## Modelo de amenazas` with points 1 to 5, in Spanish, ready to be saved as `docs/audits/amenazas.md`.
