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

## Repository documents

- [CONTRIBUTING.md](https://github.com/kosmotheka/kosmotheka/blob/main/CONTRIBUTING.md), the procedure and the rules that matter (in English).
- [SECURITY.md](https://github.com/kosmotheka/kosmotheka/blob/main/SECURITY.md), how to report a security problem privately.
- [CODE_OF_CONDUCT.md](https://github.com/kosmotheka/kosmotheka/blob/main/CODE_OF_CONDUCT.md).
- [The decision log](https://github.com/kosmotheka/kosmotheka/blob/main/docs/DECISIONS.md) and [the architecture](https://github.com/kosmotheka/kosmotheka/blob/main/docs/architecture), in Spanish.

## What comes next

The sync engine, the operations and the specification with its conformance cases, the security model and the audit method are still being put in order for this documentation. Until then, they are in the [`docs/architecture`](https://github.com/kosmotheka/kosmotheka/blob/main/docs/architecture) folder of the repository.
