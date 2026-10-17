---
title: What your server keeps, and what you can see
description: The data a Kosmotheka server keeps, what whoever runs it can see, and your obligations if you host other people.
sidebar:
  order: 7
---

This page is for deciding with the data in front of you, and for knowing what you owe to whoever keeps their libraries on your server.

## What it keeps

### Accounts

- **Email and name**, and the account's language.
- **The password**, only as a fingerprint computed with an algorithm designed for passwords. An account that signs in only with Google has no password.
- **With Google**: the identifier that Google gives the person (to recognise them next time). Nothing else from Google.
- **Second factor**, if they turn it on: the secret of their authenticator app, **encrypted** with a key derived from `APP_SECRET`, and their ten backup codes, as keyed fingerprints. Changing `APP_SECRET` invalidates them.
- **Email codes** (confirm, recover password) until they expire: minutes.

### Devices

For each device with a session: the **name** its owner gave it, an identifier, its **public key**, a token (only its fingerprint, not the token) and when it was last used. Removing a device from the account keeps its row, so that installation stays linked to the account.

### Uploaded libraries

Uploading a library to the cloud is a separate decision from having an account. When it is done, the server keeps **all of its content**: works, copies, where they are and who owns them, the people in the library with their name and role, what each one records (status, rating, favourites, wishes) and **the history of changes** with the device that made each one. The history is kept as it arrived, signed.

**There is no end-to-end encryption**: it travels encrypted to the server (HTTPS), but in the database it is readable. Libraries that are not uploaded never leave the devices.

For each library it also keeps its own key, with which the server signs as "the cloud", encrypted with `CLOUD_KEY_SECRET`.

### Invitations

Who invites, which person, and when it expires (24 hours). The code is stored only as a fingerprint; for invitations by email the **address is not stored**, but a keyed fingerprint that only lets you check that whoever accepts is the person who was invited. They are deleted a week after they expire.

### The activity log

An append-only table, so that you can account for what happens to an account or a library. It records:

- for accounts: creating the account, signing in and out, failed attempts, changing or resetting the password, turning the second factor on or off, blocking, unblocking and deleting the account;
- for cloud libraries: uploading them, changing the owner, turning the cloud's power on or off, changes of role or of the sharing permission, withdrawing or restoring a person's or a device's access, assigning a device, and invitations (created, cancelled, accepted).

For each line it stores what happened, when (the server's time), the **account** and the **device**, the library (its id and the name it had at the time), who was affected, a short detail (a role, the type of invitation…), the **IP address** and the user agent (the app version). **It does not store passwords, codes, tokens, emails or the content of the catalogue.** Accounts are identified by their id and a fingerprint of the email.

It is kept for **one year**, after which the periodic clean-up deletes it. It outlives a deleted account. The IP is the one nginx sees; for it to be the real one and not the proxy's, `TRUSTED_PROXIES` has to be set correctly ([the configuration](/en/run-your-server/configuration/)).

### The search cache

If you turn on search services ([the configuration](/en/run-your-server/configuration/#search-services)), the server keeps the **response** of the provider so as not to repeat the question: for a book, 30 days if it was found and 1 day if not; title searches (films, video games, comics, board games), 1 day on TMDB and 7 on the others. **It does not keep who asked or the text**: the search is stored under a fingerprint that does not allow it to be recovered. It only counts how many searches each account makes in an hour, for the limit (600), and does not keep that beyond the hour.

## What you can see as an administrator

Put plainly: **whoever has access to the database or to the server's console can read all of the above**, including the content of uploaded libraries and the names of the people, and can see who has signed in and from which IP. They cannot read the passwords or, without `APP_SECRET`, the second factors.

What **does not** exist is a tool that shows it: there is no panel, and no HTTP route that returns the log. What you have is:

- the database (`psql`);
- the [console commands](/en/run-your-server/administration/), especially `app:activity:list`;
- the system and nginx logs.

And you can **act** on accounts: block them, remove a second factor, and at database level almost anything. Bear in mind that all of that is your responsibility.

Tell the people you host: whoever administers the server can, technically, read their libraries. That they will not is a promise from you, not a guarantee of the technology.

## Your obligations if you host other people

If the server is only for you, this does not affect you. If the server holds data of other people (your family included, for this purpose), **you are the one who answers for it**:

- **You need your own privacy policy.** The project's ([`web/es/privacidad.html`](https://github.com/kosmotheka/kosmotheka/blob/main/web/es/privacidad.html), in Spanish) describes the official server and can serve as a model: change the controller, the contact, the hosting and the email provider to yours, and remove what your server does not do (for example, a search service you have not turned on, or Google if you did not set it up).
- **Handle their access and deletion requests.** For deletion: the person can do it in the app (see [deleting an account](/en/run-your-server/administration/#deleting-someones-account-when-they-ask-by-email)). For access, what there is are the queries of the activity log and what you pull out of the database.
- **Tell them the level of access**: that there is no end-to-end encryption and that you can read it.
- **Look after the backups**: they contain all of the above. Encrypt them, as in [the backups guide](/en/run-your-server/backups/).

This is not legal advice. What the law requires depends on where you live and on whom you host: if in doubt, check with someone who knows.

## The nginx logs

The nginx access log records every address requested. Two families of addresses carry information that must not be written down:

- **`/v1/metadata/…`**: carries what an account searches for (the ISBN, for example). Writing it to a log would break the promise above of not recording who searches for what.
- **`/v1/auth/google/connect` and `/callback`**: carry a secret in the address.

The installation guides already exclude them: a `map` in `/etc/nginx/conf.d/kosmotheka-log.conf` and `access_log off` in the `location` ([without Docker, step 7](/en/run-your-server/without-docker/#7-nginx); [with Docker](/en/run-your-server/with-docker/)). If you set up the proxy your own way, or you have a CDN or another proxy in front, **add the same there**: any system that logs addresses has to skip them.
