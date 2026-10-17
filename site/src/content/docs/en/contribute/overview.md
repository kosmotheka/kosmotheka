---
title: "The parts and how they fit together"
description: What Kosmotheka is made of and what happens, step by step, when a book is added.
sidebar:
  order: 1
---

Kosmotheka is a shared catalogue for what a home has: books first, and then comics, board games, films, video games and music. This page is the map. If you want to touch the code, start here.

Three ideas explain almost everything:

- **Local first.** The app always reads and writes its own SQLite database and works completely without a server or an account. *Why:* a household catalogue has to open in the shop, with no signal.
- **Optional server, and in charge when it exists.** Whoever wants it connects the app to a server. In that mode the server checks the permissions and is the source of truth.
- **Local sync.** Two devices pair with a QR code and pass changes to each other over the home network.

## The parts

| Part | Folder | What it is |
|---|---|---|
| The app | [`app/`](https://github.com/kosmotheka/kosmotheka/tree/main/app) | A [Tauri 2](https://tauri.app) app for desktop and Android. The interface (React, TypeScript, Tailwind and shadcn/ui) is in `src/`; the core, in Rust, is in `src-tauri/` |
| The domain | [`crates/domain/`](https://github.com/kosmotheka/kosmotheka/tree/main/crates/domain) | Domain types, validation, the hybrid logical clock and the operations. It knows nothing about databases or screens |
| The specification | [`spec/`](https://github.com/kosmotheka/kosmotheka/tree/main/spec) | The format of the operations: a JSON Schema and conformance cases. MIT licence |
| The server | [`server/`](https://github.com/kosmotheka/kosmotheka/tree/main/server) | Optional. PHP 8.5, Symfony 8, Doctrine and PostgreSQL, in Docker |
| The website and the documentation | [`web/`](https://github.com/kosmotheka/kosmotheka/tree/main/web) and [`site/`](https://github.com/kosmotheka/kosmotheka/tree/main/site) | The kosmotheka.com page and this site (Starlight) |

Each part has its own tools. The only thing the app and the server share is `spec/`.

### Where each thing goes

- **The domain logic lives in Rust**, never in the interface. React calls *Tauri commands* (Rust functions that the interface invokes) and does not run SQL. *Why:* there is only one implementation of each rule, and it can be tested without opening a window.
- **One catalogue, two implementations of the rules.** The rules of the operations are written in `spec/` and two programs follow them: the Rust core of the app and the PHP server. The same conformance cases are run by both, so that they cannot drift apart.
- **External services are always optional.** The data catalogues for books, films or games, and any future AI, sit behind interfaces that can be swapped. If they do not answer, the app keeps working.

## What happens when a book is added

Let's follow a book from the button to another device.

1. **The interface calls a command.** You tap "Save" on the book's page. React calls a Tauri command with the data.
2. **The core builds an operation.** An **operation** is an atomic batch of changes to the catalogue's entities, with an intent label (for example `book.add`). Adding a book creates, at the same time, the work, the edition, its details and the copy. *Why:* if every change is an operation, syncing is just swapping operations, and the server can check the permissions over its changes. The history of each book comes for free from the log.
3. **It is saved in the local database, in one transaction.** `ops::record` writes the operation to the log and applies its changes to the tables, all together: either everything is done, or nothing is. The operation carries a mark from a **hybrid logical clock** (a time that never goes backwards and that orders the changes of several devices even when their clocks do not match) and the device's signature.
4. **Paired devices ask for it.** When two devices are on the same network, each tells the other which operations it has and receives the ones the other is missing. The receiver checks them (signature, permissions) and applies them with `sync::merge`, which always ends in the same state whatever the order of arrival.
5. **With the cloud, the server too.** If the library is uploaded to a server, the device sends it the operation. The server checks the signature and the permissions of whoever made it and applies it again on PostgreSQL. What a device could not do according to its role is not lost: it stays as a **request** that the administrator accepts or rejects.

Someone who does not use an account stops at steps 1 to 4. The operations in full detail, the sync engine and the permissions are topics for the next parts of this documentation.

## Where to go next

- [Setting up the environment: the app](../setup-app/) and [the server](../setup-server/).
- [How to contribute](../how-to-contribute/).
- [Glossary](../glossary/), for the names that repeat in the code.
- [`AGENTS.md`](https://github.com/kosmotheka/kosmotheka/blob/main/AGENTS.md) says the same for a programming assistant; it is useful for a person too.
