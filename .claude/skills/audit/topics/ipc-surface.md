---
agent: audit-reviewer
code:
  - app/src-tauri/src/lib.rs
  - app/src-tauri/src/main.rs
  - app/src-tauri/src/commands.rs::@prod
  - app/src-tauri/src/covers.rs::@prod
  - app/src-tauri/tauri.conf.json
  - app/src-tauri/capabilities/*.json
  - app/src-tauri/gen/android/app/src/main/AndroidManifest.xml
  - app/src-tauri/gen/android/app/src/main/java/app/kosmotheka/MainActivity.kt
context:
  - facts/contracts.md#Tauri commands
decisions: D18 D27 D35 D62
max_lines: 2400
---
The boundary between the webview and the Rust core, on desktop and Android. Any script running in the webview (through XSS, for example) can call every command the capabilities allow, with any arguments; Rust must validate command inputs as if they came from an attacker.

1. **Commands**: for each one, what it can reach (files, network, database writes, keys) and how it validates arguments. Commands taking a **path** or a **URL** (data folder, backups, covers, CSV): path traversal and writes outside the app's folders (CWE-22), reading arbitrary files, fetching arbitrary URLs including local network addresses (CWE-918).
2. **Capabilities**: least privilege of each plugin permission and scope (file system paths, http URLs, shell open), which windows get them, remote URLs allowed to use IPC.
3. **Webview configuration**: CSP (missing, `null`, `unsafe-inline` or `unsafe-eval`, broad `connect-src` or `img-src`), `assetProtocol` scope, `withGlobalTauri`, devtools in release builds, `dangerousDisableAssetCspModification`.
4. **Keys and secrets**: where the signing seed, sync keys and any API key live (settings table, files, OS keychain) and who can read them. An API key in the source is `alta`.
5. **Android**: exported components and intent filters (can another app send data into the webview or IPC?), `allowBackup` and `dataExtractionRules` (database and keys copied to cloud backups), cleartext traffic, requested permissions and whether each is needed.
6. **Debug leftovers**: development-only commands, logs that print keys, tokens or personal data, the updater (if any: its public key).
