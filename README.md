# Kosmotheka

> Early development: the desktop catalogue and local-network sync with the phone work; several libraries per device are under way.

A free app to catalog what your household owns — books first, then comics, board games, films and series, video games, music and anything else. Know what you own, where it is, who has read it and who borrowed it. The name comes from Greek *kosmos* (an ordered world) and *theka* (a store): the cosmos of your things.

- **Local-first**: the app works fully offline, with no account and no server.
- **Libraries you share**: keep separate libraries (family, role-playing games, a child's video games) and choose which ones go to each device. Pair devices with a QR code and sync over your home network, or, later, through an optional server with roles and permissions.
- **Scan to check**: in a bookshop, scan a barcode and find out whether you already own it.

Desktop (Linux, Windows) and Android, built with [Tauri 2](https://tauri.app).

## Status

Phases 1 (desktop catalogue) and 2 (phone and local sync) are feature-complete, and phase 3 (interface design) is done:

- Add books by hand or by ISBN, with data and covers from Open Library and Google Books.
- Quick add: type or scan ISBNs one after another (USB barcode readers work too).
- Works, editions and copies: several formats and owners per book, locations, purchase dates.
- Edit and delete; every change is recorded as an operation, with a per-book history.
- Accent-insensitive full-text search and filters by format, owner and location.
- Backups (zip with covers) and CSV import/export.
- Sync between devices over the home network, with no server or account: pair by QR code, encrypted with a pinned certificate, conflict-free merging.
- On the phone: scan a barcode to check whether you already own a book, even offline.
- English and Spanish.

See the roadmap and the architecture decision log in [`docs/`](docs/) (currently in Spanish).

## Repository layout

| Path | Contents |
|---|---|
| `app/` | Tauri app: React UI in `src/`, Rust core in `src-tauri/` |
| `crates/domain/` | Domain model, validation, hybrid logical clock and operations |
| `spec/` | Operation format: JSON Schema and conformance cases (MIT) |
| `server/` | Optional sync server: Symfony, PostgreSQL, with its own Docker setup |
| `docs/` | Vision, decisions, roadmap, architecture, test plans and development setup |

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
cd app && npm run typecheck && npm run lint && npm run check:spec
```

Tests against the real online services are opt-in: `cargo test -p kosmotheka -- --ignored live_`.

The server runs in Docker; no PHP is needed on the host. See [`docs/development/server.md`](docs/development/server.md):

```bash
cd server
make up       # PostgreSQL, PHP-FPM and nginx on http://localhost:8095
make check    # code style, Rector, PHPStan and PHPUnit
```

## License

[AGPL-3.0-or-later](LICENSE).
