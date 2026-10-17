---
title: Contribute
description: How to help with Kosmotheka and how it is built.
sidebar:
  order: 0
---

Contributions follow the usual procedure: open an *issue* if the change is big, *fork* and make a branch, pass the checks and open a *pull request*. This part explains how the project is built, so that you can understand it, fix something or add something.

It is written for people who program and do not know the project yet. Some technical terms are used (operation, hybrid logical clock, conformance), but each one is explained the first time it appears, and it is always said why things were done that way.

## To begin with

1. [The parts and how they fit together](./overview/): the app, the domain, the specification, the server and what happens when a book is added.
2. [Setting up the environment: the app](./setup-app/): Rust, Node, Android and the checks.
3. [Setting up the environment: the server](./setup-server/): Docker, the databases, the fixtures and `make check`.
4. [How to contribute: style, tests and commits](./how-to-contribute/): the rules that matter and how to add a field or an entity.
5. [Glossary](./glossary/): what each thing is called in Spanish and in the code.

## How it works inside

6. [The sync engine, explained](./sync-engine/): how several devices reach the same result without anyone in charge.
7. [The operations: the specification and the conformance cases](./operations-spec/): the format, the rules, and how to write a compatible client or server.
8. [Security: the threat model](./security/): who it protects against, with which pieces, and what it does not protect.
9. [How we audit, so you can repeat it yourself](./audit-method/): the method, not the reports.
10. [Report a security problem](./report-a-vulnerability/): how to do it in private.

## Data, accounts and the cloud

11. [The data model](./data-model/): the entities of the catalog and of sharing, and why the work and the edition are kept apart.
12. [Accounts and sign-in](./accounts/): sign-up, Google, the second factor and the device as a session.
13. [The cloud: the API and the access rules](./cloud-api/): a map of the routes and of who reaches which library.

## Publishing

14. [Publishing a release](./release/): the number, the what's-new notes, the licences and the signed Android package.

## Repository documents

- [CONTRIBUTING.md](https://github.com/kosmotheka/kosmotheka/blob/main/CONTRIBUTING.md), the procedure and the rules that matter (in English).
- [SECURITY.md](https://github.com/kosmotheka/kosmotheka/blob/main/SECURITY.md), how to report a security problem privately.
- [CODE_OF_CONDUCT.md](https://github.com/kosmotheka/kosmotheka/blob/main/CODE_OF_CONDUCT.md).
- [The decision log](https://github.com/kosmotheka/kosmotheka/blob/main/docs/DECISIONS.md) and [the architecture](https://github.com/kosmotheka/kosmotheka/blob/main/docs/architecture), in Spanish.
