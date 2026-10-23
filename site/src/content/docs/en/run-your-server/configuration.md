---
title: The configuration, variable by variable
description: All the environment variables of the Kosmotheka server, what they are for, how they are generated and what happens if they are left empty.
sidebar:
  order: 4
---

All the configuration is environment variables. They go in `server/.env.local` (a file that is not in git and that only the server's user should be able to read), or in the environment of the service. **Do not edit `server/.env`**: it is the file of default values and it is overwritten when you update.

After changing a variable: `php8.5 bin/console cache:clear` and restart PHP-FPM (and the task service if it uses it).

A convention that repeats: **empty = that feature is not offered**, and the server keeps working. `GET /v1/server` (which you can open in the browser) says what your server offers, and it is the first thing the app checks.

## The basics

| Variable | Required | What it is |
|---|---|---|
| `APP_ENV` | yes | `prod` on a real server. The default is `dev`, which is slower and shows detailed errors |
| `APP_SECRET` | yes | The application's secret. Generated with `openssl rand -hex 32`. If empty, the server refuses to issue codes by email |
| `DATABASE_URL` | yes | The connection to PostgreSQL: `postgresql://user:password@127.0.0.1:5432/database?serverVersion=18&charset=utf8`. The `serverVersion` must be `18` |
| `DEFAULT_URI` | yes | The server's public address, `https://your-domain`, without a trailing slash. It must be exactly the one people see: the return address for Google and the server that signs each device when it signs in come from here. If it does not match the one the app has saved, **nobody can sign in** |
| `TRUSTED_PROXIES` | yes, if there is a proxy | Addresses of the trusted proxies in front of the server, to read the real IP of who calls (the request limits use it). With nginx on the same machine, `127.0.0.1`. If there is a CDN, add its ranges |
| `CLOUD_KEY_SECRET` | yes, for the cloud | 32 random bytes **in base64**: `openssl rand -base64 32`. It encrypts the private key with which each library is signed. If empty, the libraries have no cloud identity and the API returns nulls. **Losing it, or changing it, leaves the server unable to sign for the libraries already created.** Keep it in your backups, apart from the database |
| `INVITATION_BASE_URL` | yes, to invite | What the invitation links start with, without a trailing slash (`https://your-domain`; the link is `<this>/i/<code>`). If empty, the server does not do invitations |
| `REGISTRATION_ALLOWED_EMAILS` | no | Emails that can create an account, separated by commas. **If empty, anyone can register.** For a family server, set it: for everyone else, registration seems to work but nothing is created and no email is sent, and whoever already has an account signs in as usual. Signing in with Google also respects the list |
| `TWO_FACTOR_ENABLED` | no | `true` by default: new accounts can turn on a second factor (an authenticator app). Set to `false`, it is no longer offered; the accounts that had it still need it to sign in |
| `SERVER_EXTRA_ORIGINS` | no | **Leave it empty.** Other addresses that reach the same server; it is only useful in development. A device signs its sign-in for the server's address, and each address you add here is one more where that signature is also accepted |

Variables that already have a reasonable value and need no change: `APP_SHARE_DIR`, `LOCK_DSN` (`flock`, locks in the file system of a single machine) and `MESSENGER_TRANSPORT_DSN` (the task queue lives in the database itself).

## The email

Accounts send email: the code to confirm the sign-up, the one to recover the password and the invitation notices.

| Variable | Required | What it is |
|---|---|---|
| `MAILER_DSN` | yes, to register | How it is sent. By default it is `null://null`, which **sends nothing**: nobody can confirm a new account with a code. With an SMTP server, `smtp://user:password@server:587`. With Brevo (included as standard), `brevo+api://KEY@default`. Any other service needs its Symfony Mailer package |
| `MAILER_FROM` | yes | The address the email is sent from. By default it is the official service's: **put your own**, from a domain you can authenticate (SPF and DKIM) so that it does not land in spam |

With Google configured, an account can be created without the server's email (Google confirms the address), but password recovery and invitation notices do need it. So always set up email.

## Signing in with Google

Optional. You need an "OAuth client" in the Google Cloud console.

| Variable | What it is |
|---|---|
| `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET` | The identifier and the secret of that client. **Both empty, and Google is not offered.** The secret never goes to the repository |

The authorized return address you register with Google is `DEFAULT_URI` plus `/v1/auth/google/callback`. The addresses `/v1/auth/google/connect` and `/v1/auth/google/callback` carry a secret in the address and **must not be written to any access log** (nginx, a proxy, a CDN): the nginx configuration in the guide already avoids it.

## Search services

With an account, the app can search for books, films and so on through the server. Each search service is optional and needs **a key that you request yourself** from its provider, accepting its conditions. Without a key, the server does not offer that search (the app warns and keeps working, and searches on its own what it can).

| Variable | What it searches | Provider's conditions |
|---|---|---|
| `GOOGLE_BOOKS_API_KEY` | Books by ISBN (Google Books) | A Google Cloud API key, restricted to the Books API |
| `TMDB_API_TOKEN` | Films and series (TMDB) | The "API Read Access Token" of your TMDB account. Non-commercial use only, with its attribution |
| `RAWG_API_KEY` | Video games (RAWG) | A RAWG API key, accepting its conditions and its attribution |
| `COMIC_VINE_API_KEY` | Comics (Comic Vine) | A Comic Vine account; strictly non-commercial use. The server limits itself to 150 requests per hour and resource (the service allows 200) |
| `BGG_API_TOKEN` | Board games (BoardGameGeek) and role-playing books (RPGGeek) | The token of an application registered on BoardGameGeek, non-commercial. The "Powered by BGG" logo must be shown, and the app does it |
| `BGG_DAILY_CALL_CAP` | — | Cap on real calls per day to BoardGameGeek (1000 by default). The service does not publish figures: adjust it by looking at the "usage" of your registered application |
| `GAMEUPC_API_KEY` | Board games by barcode (GameUPC), and sending back the game a person picks when it is not sure | GameUPC gives the key by email to whoever makes the app. With each pick it receives a fingerprint of the account that does not say whose it is |

How they are kept: all of them travel in a header except RAWG and Comic Vine, which **only accept the key in the address of the call**. For that reason, with those two you must not put an outbound proxy or a log in front of the server that stores the addresses of `api.rawg.io` and `comicvine.gamespot.com`, nor lower the log level below `warning`. The addresses `/v1/metadata/` carry the ISBN that an account searches for: they are not written to the access log either (the guide already does it).

## Instant notices (Mercure)

Optional. See [the last step of setting it up without Docker](../without-docker/#10-instant-notices-optional).

| Variable | What it is |
|---|---|
| `MERCURE_URL` | The internal address of the hub, which is also the audience of its tokens (`http://127.0.0.1:3000/.well-known/mercure`). **Empty, there are no instant notices** and the server publishes nothing |
| `MERCURE_PUBLISHER_JWT_KEY` | Signs the tokens with which the server publishes to the hub. `openssl rand -hex 32` |
| `MERCURE_SUBSCRIBER_JWT_KEY` | Signs the subscription tokens given to the devices. **Different from the previous one**: so a subscriber token cannot publish. `openssl rand -hex 32` |

The two keys go with the same value in the server and in the hub.
