---
title: "Accounts and sign-in"
description: "How the server account works: sign-up with email and a code, signing in with Google, the second factor, the device as a session, request limits and deletion."
sidebar:
  order: 12
---

The account is **optional** and exists only on the server: the app works completely without it. This page explains how it is created, how you sign in, what each step protects and what the server stores. The detail of each route is in [The cloud: the API](../cloud-api/), and the security part in [the threat model](../security/). The code lives in `server/src/` (`Controller/`, `Security/`).

## What an account is, and what it is not

An **account** (`user`) is one person's access to the server: email, password (stored as a hash, argon2id when available), name, language (`en` or `es`) and server roles. **It is not a member** (`member`) of a library: the member is a person inside the catalog, and may or may not be linked to an account. The link is a random id, `account_id`, unique to each account, which is the **only part of the account that travels with a library**; it is not the internal id and does not come from the email, so whoever reads a library learns nobody's email.

An account starts **unconfirmed**, and it is deleted after 24 hours if nobody confirms the email.

## Sign-up with email and a code

1. **`POST /v1/account/register`** `{email, password, name, locale}`. It always answers **`202`** with an empty body, whatever happens inside: it does not say whether the email already has an account, whether it is on a closed server's list, nor whether it has used up its emails. *Why:* so that nobody can check which addresses are registered.
   - New email: creates the unconfirmed account and sends a code.
   - Email with an unconfirmed account: only sends a new code. It **changes nothing in the account**, not even the password.
   - Email with a confirmed account: changes nothing and sends a short notice ("someone tried to create an account with your address").
2. **`POST /v1/account/confirm`** `{email, code, password}` confirms the account and **sets the password**. The password is chosen here, with the code from the email, and not at sign-up: that way the one that remains is always the password of whoever controls the email, even if someone else had registered the same address before.

The **code** has 6 digits, expires after 15 minutes, allows 5 failed attempts (on the fifth it is deleted), is single-use, and there is only one live code per account and purpose. It is stored as an HMAC of the code, never in clear. It serves three things that are not mixed: confirming the email, recovering the password and confirming a sensitive action of an account without a password.

**The email is sent after answering** (in `kernel.terminate`), so that the time of the response does not reveal whether the address has an account.

**Closed server.** With `REGISTRATION_ALLOWED_EMAILS` (a list of emails), only those addresses can create an account; to the others the same `202` is answered without creating anything. `GET /v1/server` says `registration_restricted: true`, never the list.

### Forgotten password and change

- **`POST /v1/account/password/forgot`** sends a recovery code (always `202`, for the same reason).
- **`POST /v1/account/password/reset`** `{email, code, password, totp_code?}`: with the code (and the second factor, if it has one) it changes the password and **signs out the other devices**.
- **`POST /v1/account/password`** (with a session) changes the password by knowing the current one.

## The device as a session

There are no cookies and no browser session. Each installation of the app is a **device** (`device`) with an id and its own Ed25519 key pair, and **signing in is proving that you hold that private key**:

1. **`POST /v1/auth/challenge`** returns a challenge: 32 random bytes, valid for 5 minutes and **single-use** (it is consumed whether it is used well or badly).
2. **`POST /v1/auth/login`** `{email, password, device_id, device_name, public_key, challenge, signature, totp_code?}`. The app signs with the device key a text that includes the challenge, the device id, the **purpose** (`login` or `google`) and the **origin of the server**. *Why:* a signature made for another server, or for signing in with Google, is not valid here for signing in with a password, nor the other way round.
3. The server checks, **in this order**: the challenge, the signature, the account's failure limit, the credentials, the state of the account (unconfirmed or locked), the second factor, and finally the device. It answers with a **token** of 32 random bytes.

The server **stores only the hash of the token**. Each authenticated call carries `X-Device-Id` and `Authorization: Bearer <token>`.

**A device is its id together with its public key**, not the id alone. The id travels in each operation and everyone who shares a library sees it; if the id alone were enough, whoever knew it could sign in first with a key of their own and lock out the real phone. A device can be **signed in**, **signed out** (the same account can sign in with it again) or **removed**. A removed device does not sign in again with any account; the app, on learning that, creates a new identity.

If the server closes a session (for example, because the password changed on another device), the next call answers `401` with the reason, so that the app can explain it.

## Sign in with Google

It is optional (the server must have a Google client configured) and follows the "OAuth with PKCE" path, designed so that **what travels through the browser is no use on its own**:

1. The app generates a secret that **never leaves it** and sends its fingerprint to `POST /v1/auth/google/start`. The server stores an **attempt** (10 minutes) and returns the address to open.
2. The system browser goes to `GET /v1/auth/google/connect`. The server generates its own `state` and a cookie tied to that browser, and redirects to Google.
3. Google comes back to `GET /v1/auth/google/callback`. The server only continues if the `state` is its own, from an attempt that is waiting, **and the cookie is that of the same browser**. Each `state` is valid once.
4. The app finishes with `POST /v1/auth/google/finish`, with **the same device proof as the normal sign-in** and the secret from step 1. Without that secret, the code that came back through the browser gives nothing.

In addition, the app **only accepts the return of an attempt it started and is waiting for**, and only an email that Google confirms counts. The person is recognised by their **Google identifier**, not by the email. If an account registered with a password was never confirmed and the real owner signs in with Google, that account becomes theirs and **the other person's password stops working**.

**Second factor when signing in with Google:** the server **asks for it too** if the account has it turned on (`GoogleController::finish` checks the account's failure limit and then calls `requireTwoFactor`). Google does not replace the account's second factor.

## The second factor

Optional per account, and the server can turn it off altogether (`TWO_FACTOR_ENABLED`). It is a **TOTP** code (that of an authenticator app: 6 digits, 30 seconds) plus **ten backup codes**, single-use, for whoever loses the phone.

- **`POST /v1/account/two-factor/setup`** creates a pending secret and returns the secret and its `otpauth://` address.
- **`POST /v1/account/two-factor/enable`** `{code, …}` checks a code against the pending secret, turns the factor on and returns the ten codes **only once**.
- **`DELETE /v1/account/two-factor`** and **`POST /v1/account/two-factor/backup-codes`** remove it and regenerate the codes.
- The second-factor code can be the 6-digit one or a backup one (with or without the hyphen, in upper or lower case).
- **Anti-replay:** a code from a time step equal to or earlier than the last one accepted is rejected, even if it is valid.
- **The secrets are stored encrypted** (XChaCha20-Poly1305, with a key derived from `APP_SECRET` and the account id as associated data): a copy of the database alone is no use for generating codes. The backup codes, as hashes.

### The sensitive actions

A session lasts a long time, and an unlocked phone in other hands holds it. What changes **how you get into the account** asks for more than the session:

| Action | Asks for | And, with the second factor on |
|---|---|---|
| Change the password | the current password | a code |
| Reset it | the code from the email | a code |
| Delete the account | the password, or an email code if it has no password | a code |
| Turn on the second factor | the password, or an email code | its own code |
| Turn it off or regenerate the codes | the password, if it has one | a code |

An account without a password (Google only) proves who it is with an **email code** (`POST /v1/account/code/action`). An account that has a password always uses the password: **it cannot swap it for an email code**, which whoever had the email open would also have. The order is always: who is asking, then the second factor, and only at the end is the email code spent.

## Request limits

`symfony/rate-limiter`, with a sliding window. The ones a request spends on arrival are `#[RateLimit]` attributes on the action; the ones that depend on the flow are consumed by `App\Security\RequestLimiter`. They all end in the same response, `429 {"error":"too_many_requests"}` with `Retry-After`.

| Limit | How much | Key | What for |
|---|---|---|---|
| Sign-up | 5 per hour | IP | So that nobody creates accounts en masse |
| Sign-in (the challenge and the sign-in) | 30 every 15 minutes | IP | Wide on purpose: a family on the same Wi-Fi shares an IP |
| **Failures of an account** | 5 every 15 minutes | fingerprint of the email typed | Only counts **failed** attempts (wrong password or code). An email with no account counts the same, so as not to reveal which exist |
| Codes by email | 10 every 15 minutes | IP | |
| Emails per address | 3 every 15 minutes | fingerprint of the address | So that a third party cannot be flooded with emails |
| Second-factor code | 5 every 15 minutes | account | So that the 6 digits cannot be tried from many IPs |

A malformed body (`400` or `422`) **does not spend** the limit; a well-formed request with the wrong password does. The limits for the cloud, the invitations and the metadata are in [The cloud: the API](../cloud-api/).

## Deleting the account

**`DELETE /v1/account`** asks for the password (or an email code, and the second factor if it is on). A library **is not lost with the account of whoever uploaded it**: before deleting it, and in the same transaction, each cloud library of which it is the owner passes to another account with an active seat in it (administrator first), and it is only deleted with the account if nobody is left.

In addition, an hourly purge (`App\Scheduler\PurgeSchedule`, which needs a running *worker*) deletes the unconfirmed accounts older than 24 hours and the expired codes.

## What the server stores

- **Of the account:** email, password hash, name, language, the `account_id`, and if there is a second factor, its encrypted secret and the hashes of the backup codes.
- **Of each device:** its id, its name, its public key, the **hash** of the session token and when it was last used.
- **The codes** from the email, as HMACs, deleted when they expire.
- **From Google:** the Google identifier, the email and the name. **No Google token.**
- **The activity log:** an append-only table with what happened, which account and device it is due to, when, and from which IP and app version. It is kept for a year. It does not store passwords, codes or links; of a deleted account only a fingerprint of the email remains.

**What it does not store:** the password in clear, the token in clear, the codes in clear, or what an account searches for in the data services.

## Errors

Business errors answer with a body `{"error": "<code>"}` and the HTTP status that applies. The codes are cases of `AccountErrorCode`, and a single listener (`AccountExceptionListener`) turns them into a response, so that no controller builds it by hand. Validation errors of the body answer `422` in Symfony's format.
