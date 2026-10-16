# Security policy

Kosmotheka keeps a household's catalogue, who borrowed what and who may change it. If you find a way to read or change data you should not, to impersonate a device, or to get past the permission rules, please tell us privately first.

## Reporting a vulnerability

**Do not open a public issue.** Use GitHub's private reporting instead: the **Security** tab of this repository, then **Report a vulnerability**. Only the maintainer sees the report.

Please include:

- what an attacker can do, and what they need first (same network, a paired device, an account, a role);
- the steps to reproduce it, and the version of the app or the commit of the server;
- anything you already know about a fix.

This is a project maintained in spare time. You will get an answer within 7 days. A fix for a serious issue comes before any other work, and you will be credited in the release notes unless you prefer otherwise.

## Supported versions

Only the latest release of the app and the `main` branch of the server get security fixes.

## Scope

- The app (desktop and Android), its local sync between devices and the operation format in `spec/`.
- The sync server in `server/`.

Servers that other people host are run by them; report problems in this code, not in someone else's installation.

The threat model the project works from is in [`docs/audits/amenazas.md`](docs/audits/amenazas.md) (in Spanish; it will be translated).
