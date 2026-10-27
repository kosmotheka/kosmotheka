---
title: Administering the server
description: Who can sign up on your Kosmotheka server, and the console commands to block accounts, remove a second factor, read the activity log and keep the data in order.
sidebar:
  order: 6
---

The server has no web panel. It is administered with environment variables ([the configuration](/en/run-your-server/configuration/)) and with console commands. This page gathers the two things you will need day to day.

## How the commands are run

From the `server/` directory, as the user that runs the server (so the cache does not end up owned by another user):

| Installation | Command |
|---|---|
| Without Docker | `php8.5 bin/console <command>` |
| With Docker | `docker compose run --rm php bin/console <command>` |

The examples below use the short form, `bin/console …`. `bin/console list app` shows the project's commands.

## Who can sign up

`REGISTRATION_ALLOWED_EMAILS` is a list of emails separated by commas, spaces or semicolons (not case-sensitive).

- **Empty** (the default): anyone can create an account.
- **With addresses**: only those can. For a family server, set it before opening the server to the internet.

What someone who is **not** on the list sees:

- **With email and password**: the app treats it like any sign-up, warns that the server only admits some addresses and asks for the 6-digit code. That code **never arrives**: the server does not create the account or send anything. The person trying cannot tell whether it is the list or an email fault.
- **With Google**: they see "This server is not taking new accounts for now."

Someone who already has an account signs in as usual, even if their email is not on the list. To add another person, add their email to the variable, run `bin/console cache:clear` and restart PHP-FPM (see [the configuration](/en/run-your-server/configuration/)).

## The console commands

### `app:user:block` · block an account

```bash
bin/console app:user:block someone@example.org
bin/console app:user:block someone@example.org --undo
```

Blocks the account and **signs out all of its devices**. With `--undo` it unblocks it. The blocked account cannot sign in and sees "This account is locked. If you think this is a mistake, contact whoever runs the server." It deletes nothing: the account, its libraries and its data stay. It is recorded in the [activity log](/en/run-your-server/what-the-server-keeps/#the-activity-log).

### `app:user:delete` · delete an account

It is further down, in [deleting someone's account when they ask by email](#deleting-someones-account-when-they-ask-by-email).

### `app:user:two-factor-disable` · remove the second factor

```bash
bin/console app:user:two-factor-disable someone@example.org
```

For someone who has lost their phone **and** their ten backup codes and cannot get in. It removes the account's second factor; it fails if the account had none. Afterwards, that person signs in with their password and can turn the second factor on again.

**Make sure it is who they say they are before running it**: this command is what turns "I have the password" into "I get in". Write to the account's email, or confirm it by another means you know. It is recorded in the activity log.

### `app:user:super-admin` · the super administrator role

```bash
bin/console app:user:super-admin someone@example.org
bin/console app:user:super-admin someone@example.org --revoke
```

Grants or removes `ROLE_SUPER_ADMIN`. It can only be given from the console, never through the API. **Today that role opens nothing**: the server does not yet have a panel or administration routes. It exists for when they do, and you do not need to use it.

### `app:activity:list` · read the activity log

```bash
bin/console app:activity:list --since=2026-10-01 --limit=100
bin/console app:activity:list --account=<account id> --kind=login_failed
```

It is the **only** way to read the log (there is no HTTP route). It shows a table with the most recent first: when (UTC), what, the account, the device, who uploaded it, the library, the person affected, the detail, the IP and the user agent.

| Option | Filters by |
|---|---|
| `--account` | One account (its id): what it did or what its session uploaded |
| `--library` | One cloud library (its id) |
| `--kind` | One type of event, for example `login_failed`, `password_changed`, `account_blocked`, `invitation_accepted`. If you get it wrong, the command lists the valid ones |
| `--since` · `--until` | From and until (UTC; `--until` not included) |
| `--limit` | Maximum number of lines (50 by default, 1000 at most) |

Accounts are identified by their id, not by their email: the log **does not store the email**, only a keyed fingerprint that lets you check "was it this address?" without having it written down. What is recorded is explained in [what your server keeps](/en/run-your-server/what-the-server-keeps/#the-activity-log).

### `app:accounts:purge` · the periodic clean-up

```bash
bin/console app:accounts:purge
```

In one pass, it deletes:

- **unconfirmed accounts older than 24 hours**;
- expired email codes;
- invitations **that expired more than a week ago**;
- activity log lines **older than a year**;
- expired attempts to sign in with Google;
- saved searches that have passed their time limit (see [the search cache](/en/run-your-server/what-the-server-keeps/#the-search-cache));
- covers no work of their library uses any more, a day later, and the cover folders of libraries that no longer exist.

**You do not need to run it by hand**: the scheduled process runs it every hour (the `worker` service of Docker, or `kosmotheka-worker` of systemd, which you set up in the installation guide). Without that process running, nothing is deleted. Each step runs separately: if one fails, the others still happen and the command ends with a warning listing which ones failed.

### `app:trash:expire` · empty what is old in the trash

What somebody deletes in the app goes to the trash of their library and can be restored for 30 days. After that, the devices themselves delete it. In case no device of a cloud library is opened for a long time, the server deletes what has been **60 days** in the trash, signing as the cloud device of that library. The 60 days count from when the row reached the server, so a device with its clock wrong brings nothing forward.

The same scheduled-tasks `worker` runs it once a day. To run it by hand:

```bash
bin/console app:trash:expire
```

It says how many things it deleted. It only acts in libraries whose cloud has its power; in the others it does nothing.

### `app:catalog:rebuild` · rebuild the catalogue

```bash
bin/console app:catalog:rebuild <cloud library id>
bin/console app:catalog:rebuild --all
```

For each library, the server keeps the **operation log** (the truth) and, apart from it, a catalogue derived from it that is used for queries. This command deletes a library's catalogue and calculates it again from the log, judging each operation again. It prints how many were applied and how many were left as requests.

You rarely need it: when a server update changes how operations are judged and the release notes ask for it, or if you suspect the catalogue does not match the log. It is safe (the log is not touched) and takes the library's lock, so while an upload of that library is running it waits. With `--all` it rebuilds one by one, each in its own transaction; if one fails, it carries on with the others and ends with an error. The id is that of the **cloud** library (the server's), the same one that `app:activity:list --library` shows.

## Deleting someone's account when they ask by email

Whoever asks has the right to have the account deleted, and the project's privacy policy promises to do it within 30 days at most (yours will say what you promise). With `app:user:delete` it takes a minute:

1. **Check that whoever writes is the owner of the email.** The request has to come **from the account's address**. If it comes from another, ask them to send it from theirs, or confirm it by another means you know. Never delete an account on a request you cannot verify: it cannot be undone.
2. **If they can sign in to the app**, it is cleanest for them to do it: **Settings › Cloud and account › Delete account**, with their password (or an email code if they sign in with Google) and their second factor.
3. **If they cannot sign in, or you prefer to**, run the command with the account's email.
4. **Reply to them** when it is done.

### `app:user:delete` · delete an account

```bash
bin/console app:user:delete someone@example.org
bin/console app:user:delete someone@example.org --force
```

It does **the same as the app's button**, with the same code: in a single transaction, the libraries the account uploaded to the cloud pass to another account that has a place in them (first whoever administers, and among equals the oldest), or they are deleted with the account if nobody else is left; one line is written to the activity log, and the account is deleted, with its devices and its codes.

Without `--force`, before deleting it **shows what will happen and asks for confirmation** (the default is "no"):

```
Account: someone@example.org (0196b2f1-…)
It uploaded 2 cloud libraries:
  - 0196b2f3-…: passes to account 0196b2f0-…
  - 0196b2f4-…: is deleted with the account (nobody else has a place in it)
Delete this account? It cannot be undone. (yes/no) [no]:
```

With `--force` it does not ask, for use in a script. If the email has no account, it fails with an error message and a non-zero exit code, without touching anything. The activity log gets **one `account_deleted` line** with the account's id and a fingerprint of its email, **without the email**, and with the channel `console`; it is kept until its year is up.

**Do not delete the database rows by hand**: a direct `DELETE` skips the handover of libraries and could take the library away from everyone.
