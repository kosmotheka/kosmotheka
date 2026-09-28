---
agent: audit-reviewer
code:
  - server/src/**/*.php
  - server/public/index.php
  - server/config/packages/*.yaml
  - server/config/services.yaml
  - server/config/routes.yaml
  - server/docker/docker-compose.yaml
  - server/docker/php/Dockerfile
  - server/docker/nginx/Dockerfile
  - server/docker/nginx/*.conf
context:
  - server/AGENTS.md
  - docs/development/server.md
  - facts/surface.md#Environment files
decisions: D06 D09 D38 D41 D42 D43 D48 D49 D50 D51 D57 D62 D66
max_lines: 2600
---
The optional Symfony sync server: security (OWASP API Security Top 10 2023, ASVS 5.0 level 2) and code quality against `server/AGENTS.md`. It is at an early stage: say which parts do not exist yet.

1. **Surface**: routes, firewalls and `access_control`; the entry point; published ports; whether debug tools or `APP_DEBUG` can be active outside development.
2. **Authentication and authorization** (API1, API2, API5): the server must never trust a library, member, device or role sent by the client, and must check every change of an operation. If endpoints do not exist yet, check the building blocks they will use.
3. **Input** (API3, API4): unknown fields, strict types, limits on body size, number of changes, string length and JSON depth; enums parsed with `tryFrom` and a clean error, not `from` and an HTTP 500; the validator implements every rule of the spec.
4. **Configuration**: `APP_SECRET` and database passwords in committed `.env*` files (development values are fine only if production must override them and `docs/development/server.md` says so); trusted proxies and hosts; CORS; HTTPS and HSTS; security headers; nginx serves only `public/` and executes only `index.php`.
5. **Docker**: containers running as root; PostgreSQL or PHP-FPM published on all interfaces; default credentials; `spec/` mounted read-only.
6. **Quality**: each rule of `server/AGENTS.md` (types, DTOs, errors, where classes go, tests); dead code; names that break the glossary.

Add a final section `## Requisitos para lo que viene`: the controls the next server milestones must include, one line each with the reason.
