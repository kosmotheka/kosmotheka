---
title: Before you start
description: What you need to know before hosting your own Kosmotheka server, and what it takes to do it.
sidebar:
  order: 1
---

The server is optional: the app works completely without it, with its database on your phone or computer. A server lets you have your libraries on all your devices without sharing Wi-Fi, and share them with other people by invitation. This guide is for those who want to host their own instead of using the official one.

## What you need to know

These are the limits of hosting your own server. They are here, at the start, so you can decide with them in front of you.

- **The official app can connect to your server.** Under "Other server" you choose the address of yours. You do not need to compile anything.
- **No notices while the app is closed.** The app updates when you open it. The server can notify the apps that are open right away (with an optional component, Mercure), but it does not send notifications to the phone.
- **Invitation links are pasted into the app.** Those from your own server do not open with a tap by themselves: the person who receives the link copies it and pastes it into the app.
- **Signing in with Google, email and the keys for the data search services are optional and set up separately.** Without email, nobody can confirm a new account with a code (see [the configuration](./configuration/#the-email)). Each search service (films, video games, comics…) needs a key that you request from its provider, under its conditions.
- **Whoever runs the server can read what is uploaded to it.** The libraries are not encrypted end to end: they are in the server's database. If you host the server for other people, tell them.
- **The name and the logo are not yours.** The code is free, but "Kosmotheka" and its logo cannot be used to make your server or your app pass for the official one (see [TRADEMARKS.md](https://github.com/kosmotheka/kosmotheka/blob/main/TRADEMARKS.md)). Saying that your server "works with Kosmotheka" is allowed.

## What you need

- **A domain with HTTPS.** The app only accepts a server without HTTPS on the home network (a private address, such as `192.168.…`); from outside it needs HTTPS, and in any case passwords and libraries must not travel unencrypted. You need a name (for example `kosmotheka.your-domain.example`) pointing to the machine, and a certificate (the guide uses Let's Encrypt).
- **A Linux server with modest resources.** For a family a small machine is enough: 1 CPU, 2 GB of memory and a few GB of disk. To give you an idea: in load tests, uploading a library of 2,000 books at once takes about a minute, a normal change arrives in less than a tenth of a second, and a normal request uses about 120 MB of PHP memory. If the machine shares space with other services, give PHP a limited `memory_limit` (the guide does it).
- **Knowing how to administer a Linux server:** installing packages, editing the nginx configuration, using systemd and reading an error log. You do not need to know Symfony or PHP.

## The two paths

The server can be set up in two ways with the same parts (PHP, PostgreSQL, nginx and a scheduled-task process):

1. **[Without Docker](./without-docker/)**, installing everything on the machine. This is how the official server works.
2. **[With Docker](./with-docker/)**, with Docker Compose. You build the image yourself: it is not published in any registry.

After setting it up:

- [The configuration, variable by variable](./configuration/).
- [Connecting the app to your server](./connect-the-app/).
- [Administering the server](./administration/): who can sign up and the console commands.
- [What your server keeps and what you can see](./what-the-server-keeps/), and your obligations if you host other people.
- [Backups and how to restore](./backups/).
- [Updating to a new version](./updating/).
- [Common problems](./troubleshooting/).
