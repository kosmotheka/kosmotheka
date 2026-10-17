---
title: "The cloud: the API and the access rules"
description: "A map of the /v1 routes grouped by topic, where access to a library comes from, how operations are uploaded and downloaded, and how live notices work."
sidebar:
  order: 13
---

This page is a **map** of the server's API: which routes there are, what each one does in a line and which rules govern them. It is **not a reference for each field**: for that, the code is the authority, and each section says where to look. To understand what an operation is, read [The sync engine](../sync-engine/) first; for accounts, [Accounts and sign-in](../accounts/).

All routes hang from `/v1`, speak JSON and use `snake_case`. For the real and up-to-date list, run `make console C="debug:router"` in `server/`. Business errors answer `{"error": "<code>"}` (the cases of `AccountErrorCode`), with a `429` and `Retry-After` when a request limit is reached.

## The golden rule: the server does not trust any client id

It is the rule that decides everything else (decision D84). **The ids that arrive in a request are compared, never used to look things up or to decide access**:

- the library's local id,
- the operation ids,
- and the device ids.

*Why:* those ids are not secrets (everyone who shares a library sees them), so whoever knows them must not be able to take advantage of them. The case against which everything is measured is that of a hostile account that knows all the ids of another family: it must not receive anything from it, write to it, or get in its way.

Access comes only from three things, which the server already has:

1. **The authenticated account and device** (by the token, not by what the request says).
2. **The access table** (`membership`): which accounts may enter which library.
3. **The library's own cloud id** (`C`, `cloud_library_id`), which **the server creates**. It is different from the library's local id: the server never translates one into the other.

Two practical consequences:

- **A library that is not yours and one that does not exist answer the same**: `404 not_found`. That way nobody can find out which libraries exist.
- **The same local id in two accounts are two different cloud libraries**, with their own operations and keys.

### Where access to a library comes from

The gate is `CloudLibraryAccess` (in `server/src/Sync/`). An account reaches a library if:

- it has a **membership** in it, **active** (not withdrawn) and, if the library has roles, **linked to a person** of the library who does not have access withdrawn;
- and the **device** with which it calls is not cut off: there is no withdrawal applied in the library that names that device with the same key.

The cloud cuts access **at once**, reading the catalog it stores, in two cases: when a device is withdrawn and when a person is removed. And to a cut-off device it shows the signed proof from whoever administers; the device only deletes the library if that proof is good. More in [Security](../security/).

A cloud library has an **owner** (the account that uploaded it), but that only counts for the first link and for deletion: everyone's access, the owner's included, comes from `membership`. If the owner deletes their account, the library passes to another account with an active seat, and it is only deleted if nobody is left.

## The routes

All except those marked as public require a device with a session (the headers `X-Device-Id` and `Authorization: Bearer`).

### The server

| Route | What it does |
|---|---|
| `GET /v1/server` (public) | Says **what this server offers**: whether it admits registration and whether that is restricted, whether it signs in with Google, whether it has a second factor, live notices, invitations (and the base of their links) and which data services it searches. The app decides what to show from this, instead of assuming which features exist |

### Account and session

Explained in [Accounts and sign-in](../accounts/).

| Route | What it does |
|---|---|
| `POST /v1/account/register` (public) | Creates an unconfirmed account and sends a code. Always `202` |
| `POST /v1/account/confirm` (public) | Confirms the email with the code and sets the password |
| `POST /v1/account/code` (public) | Asks for another confirmation code |
| `POST /v1/account/code/action` | Sends the account a code for a sensitive action (accounts without a password) |
| `POST /v1/account/password/forgot` and `/reset` (public) | Recovers the password with an email code |
| `POST /v1/account/password` | Changes the password, knowing the current one |
| `GET` · `PATCH` · `DELETE /v1/account` | View, change name or language, and delete the account |
| `GET /v1/account/devices` | Lists the account's devices |
| `DELETE /v1/account/devices/{id}` · `POST …/{id}/restore` | Removes a device, and undoes it while the installation has not noticed |
| `POST /v1/auth/challenge` · `/login` (public) | Asks for the challenge and signs in by proving the device key |
| `POST /v1/auth/logout` | Signs this device out |
| `POST /v1/auth/google/start` · `/connect` · `/callback` · `/finish` (public) | Sign in with Google |
| `POST /v1/account/two-factor/setup` · `/enable` · `/backup-codes`; `DELETE /v1/account/two-factor` | The second factor |

### The cloud: libraries and sync

All under `/v1/cloud/libraries`.

| Route | What it does |
|---|---|
| `POST /v1/cloud/libraries` | Creates a cloud library for the account, with its membership and the cloud identity. Returns its `C` |
| `GET /v1/cloud/libraries` | Lists the ones **this device reaches** and, apart, those the account had and from which it has been cut off |
| `POST /v1/cloud/libraries/{C}/link` | Links `C` with the library's local id, with a signed operation. It is a separate step before uploading |
| `POST /v1/cloud/libraries/{C}/push` | **Uploads operations** (up to 1,000 per call) |
| `POST /v1/cloud/libraries/{C}/pull` | **Downloads the operations the device lacks**, according to its version vector |
| `POST /v1/cloud/libraries/claim` | Links the account with its person in each library where its device is already that person's and, on a new phone, links the device. **It receives no id** |
| `GET /v1/cloud/events` | Gives the token for live notices |

### Invitations

| Route | What it does |
|---|---|
| `POST /v1/cloud/libraries/{C}/invitations` | Creates an invitation (administrators only) |
| `GET /v1/cloud/libraries/{C}/invitations` | Lists the library's invitations (administrators only), without the link or the email |
| `DELETE /v1/cloud/libraries/{C}/invitations/{id}` | Cancels one (the id is compared with those of `C`) |
| `GET /v1/cloud/invitations/{token}` (public) | Says just enough for "You are invited to…": the library's name, the kind and when it expires |
| `POST /v1/cloud/invitations/{token}/accept` | Accepts, with the account that has signed in |

### Metadata: searching data through the server

The server searches, for whoever has an account, in data services with its own key. Only **what is searched** is sent; the response always has a small shape of its own, never the service's.

| Route | What it searches |
|---|---|
| `GET /v1/metadata/google-books/isbn/{isbn}` | A book by ISBN (Google Books) |
| `POST /v1/metadata/tmdb/search` | Films and series by title (TMDB) |
| `POST /v1/metadata/rawg/search` | Video games by title (RAWG) |
| `POST /v1/metadata/comic-vine/search` | Comics by title (Comic Vine) |
| `POST /v1/metadata/bgg/search` | Board games and role-playing books by title (BoardGameGeek) |

If the server does not have a service's key, its route answers `404 metadata_unavailable` and `GET /v1/server` does not list it. The server keeps the response for a few days, **without keeping who asked or the text searched** (the search is stored under a fingerprint), and keeps a cap on real calls per day to the service and per account.

## Uploading and downloading operations

The format of what travels is the one in [the specification](../operations-spec/). Here, the rules of the route.

### Uploading (`push`)

Each operation in the upload goes through these checks, in this order (`App\Sync\CloudPush`):

1. **Well formed:** the same validation as in the app, with the same test cases.
2. **Did we already have it?** The same operation in the same `C` counts as known and changes nothing.
3. **It belongs to this library:** its `library` must be the local id linked to `C`, and its **signature** must hold with that device's key **within this `C`** (that of its `device` row, or the one the operation itself registers for itself). A different key for an already registered device is rejected.
4. **Each author's chain:** if the author's previous operation is missing, the operation **waits** (it is rejected, and it blocks the later ones from that author in the call; the app resends it in full afterwards).
5. **One HLC per operation**, and nothing more than an hour in the future.
6. **It is judged:** permissions with the same rules as the devices. What is not allowed stays as a **request**: it is stored and distributed, but not applied.

The response counts how many were **received**, how many **were already known** and how many were **rejected**. A bad operation does **not** make the batch fail: it is counted as rejected. Everything goes in a transaction that locks the `C` row, so two uploads at once (two administrators) do not overwrite each other. The client's schema version must be the server's (`spec/sync.json`) or it answers `409 schema_version`.

**Linking is a step of its own.** The app first sends to `link` the signed operation that links the library (checked against a device of the owner's account), and then uploads the history. That way a library of more than a thousand operations enters in batches without trouble.

### Downloading (`pull`)

The client sends its **version vector** (the latest mark it has for each author) and the server answers with **all the operations of that library with a higher mark**, applied or as requests, ordered by mark, and with its own vector. It is the same protocol that two devices use over the local network. Each device judges for itself what it receives.

### The merge engine

What is uploaded is applied to the catalog with the same merge engine the app uses, written again in PHP (`Sync/Merge/`). **It is the only part that writes the catalog with DBAL instead of Doctrine's ORM**, and all its SQL comes from a single class (`CatalogSql`), built from the metadata of the entities.

## Request limits

Per account, unless stated otherwise: uploading and downloading, 600 per hour each; linking and `claim`, 30 per hour; the notices token, 120 per hour; invitations, 20 per hour to create them, 10 every 15 minutes to accept, and 30 every 15 minutes per IP to view them; metadata, 600 per hour. The account ones are in [Accounts and sign-in](../accounts/).

## Live notices

With the app open, the server can signal "there is something new in this library" so that it syncs at once. It is **optional**: without it, the app syncs when it opens, when it comes back to the foreground and after each change.

- **Mercure.** A separate hub (Mercure) keeps the connections open, because PHP cannot. Symfony publishes to it over the internal network; the app reaches it through the web server.
- **The notice carries no data**: it says *that* something changed, not *what*. What each device may see is still decided by `pull`, with the access table.
- **The topics** are strings, never the local id: `urn:kosmotheka:cloud-library:<C>` (all the accounts with access to that library, after an upload that brought something) and `urn:kosmotheka:account:<id>` (a particular account, when its list of libraries changes).
- **The token** (`GET /v1/cloud/events`) reads nothing from the client: the topics come from the account's memberships. It is a JWT signed with a **subscriber** key, different from the publisher's (a subscriber token can never publish), and it **expires after 10 minutes**. Each time a new one is requested, the access table is read again, so whoever loses access stops receiving the topic as soon as they renew, and the hub cuts their connection when the token expires.
- **Without Mercure**: the token route answers `409 events_unavailable` and `GET /v1/server` says `events: false`.
- **A notice never makes an upload fail**: it is published after the transaction is confirmed, and if the hub fails it is only noted in the log.

## Where to look

- The routes: `server/src/Controller/` (`CloudController`, `InvitationController`, `CloudEventsController`, `MetadataController`, `ServerController`).
- The access and `push` rules: `server/src/Sync/` (`CloudLibraryAccess`, `CloudPush`, `CloudPull`, `CloudLink`, `AccessCutter`, `AccountLinker`).
- The limits: `server/config/packages/rate_limiter.yaml`.
- For the detail of each route and its hostile cases, the tests: `server/tests/Controller/CloudSyncControllerTest.php` and the others in that folder.
