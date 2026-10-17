---
title: What's new in each version
description: "What has changed in each version of Kosmotheka, from the newest to the oldest."
sidebar:
  order: 21
---

The app shows you these notes when it is updated, once only. To see them again, open **Settings › About › What's new**.

When a change alters what leaves your device, the app also shows you a **switch**, so that you can decide whether you want it. Here we call it a "switch".

## Version 0.2.1

- The first time, the app asks for your name: your library is created in your name, as its administrator, and you can change it later in Settings. When you pair a device you must say whose it is and what role it has ("editor" by default).
- Test data must be deleted before using this version: on each device, "Hand this device to someone else" in Settings; and in the cloud, delete the account and sign up again. Also, restoring a backup on a new phone does not join the original library: it becomes a new library with the same books.
- Changing or resetting the password and deleting the account ask for the code from the authenticator app, if the account has one.
- When adding, an ISBN with hyphens or ten digits is looked up as an ISBN; if its check digit is wrong the app says so instead of asking what it is, and if no service knows it you can add the book by hand with the number already filled in.
- With an account, board games are searched by title on BoardGameGeek, and searching for books also finds role-playing books from RPGGeek. What you type goes to your account's server, which asks them; you can turn it off in Data providers. *(With a switch.)*

## Version 0.2.0

- The cloud arrives, optional and by invitation: with an account (email or Google) you keep your library on all your devices without sharing wifi. With no account, everything works as before.
- You can invite other people to a library, by email or with a link.
- With an account, looking a book up by ISBN fails less often.
- You can remove a person from a library and give their access back.

## More

- What leaves your device and what does not: [What leaves your device](../what-leaves-your-device/) and [Where the details come from](../data-sources/).
- If something does not work as expected after updating: [Error messages and what to do](../error-messages/).
