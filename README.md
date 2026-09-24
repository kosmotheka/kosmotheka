# Library

> Working name. Early development: nothing usable yet.

A catalog for your household library — books first, then board games, comics, video games and films. Know what you own, where it is, who has read it and who borrowed it.

- **Local-first**: the app works fully offline, with no account and no server.
- **Family sharing**: pair devices with a QR code and sync over your home network, or connect to a server to share with the whole family, with roles and permissions.
- **Scan to check**: in a bookshop, scan a barcode and find out whether you already own it.

Desktop (Linux, Windows) and Android, built with [Tauri 2](https://tauri.app).

## Status

Phase 0 (skeleton). See the roadmap and the architecture decision log in [`docs/`](docs/) (currently in Spanish).

## Repository layout

| Path | Contents |
|---|---|
| `app/` | Tauri app: React UI in `src/`, Rust core in `src-tauri/` |
| `docs/` | Vision, decisions, roadmap, research and development setup |

## Development

See [`docs/development/setup.md`](docs/development/setup.md) to set up the toolchain. Then:

```bash
cd app
npm install
npm run tauri dev          # desktop app with hot reload
npm run tauri android dev  # on an Android device
```

Checks run in CI:

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cd app && npm run typecheck && npm run lint
```

## License

[AGPL-3.0-or-later](LICENSE).
