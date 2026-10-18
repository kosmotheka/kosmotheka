---
title: Connecting the app to your server
description: How to point the Kosmotheka app at your own server, which addresses it accepts, what happens when it does not answer, and how invitations work.
sidebar:
  order: 5
---

The official app connects to your server without compiling anything. You only need the public address (the same one you put in `DEFAULT_URI`) and `GET /v1/server` to answer.

## Step by step

1. In the app, open **Settings › Cloud and account** and tap **Sign in or create an account**.
2. On the sign-in screen you will see "On *the official server*" and, next to it, **Other server**. Tap it.
3. Type the full address in the **Server** field, with `https://` and no path: `https://kosmotheka.your-domain.example`. A final slash is ignored.
4. When you leave the field, the app calls `/v1/server`. If the server answers, a green dot appears with "Responds · email and password only" (that text is fixed: if the server offers Google, the button appears anyway). If not, you see the error below the field (further down, in [when the server does not answer](#when-the-server-does-not-answer)).
5. Sign in with an account that already exists there, or tap **Create account**.

**Use the official one** (top right of the field) takes you back to the official server.

### When creating the account

A new account is created in two steps:

1. Name, email and password (at least 8 characters). The server always answers the same way, whether the account already exists or not.
2. A 6-digit code that the server sends by email and that expires after 15 minutes. Until it is confirmed, the account does not work; **unconfirmed accounts are deleted after 24 hours**.

That is why you need [email set up](/en/run-your-server/configuration/#the-email): without it, nobody gets the code. If the server has a list of allowed emails (see [who can sign up](/en/run-your-server/administration/#who-can-sign-up)), the app warns that "this server only admits some addresses". Someone who is not on the list still reaches the code step, but **nothing arrives for them**.

When you sign in, the device registers with the server under its name, an identifier and its public key, and signs the entry for **that address**: a signature made for another server is not valid on yours. If you later change the server's public address (`DEFAULT_URI`), **nobody can sign in** until the app has the new one.

### The "not official" notice

Kosmotheka's app is the same for the official server (`api.kosmotheka.app`) and for yours, and it works with any server, but **whoever uses it has to know when their data is on a server that is not the official one**. So, with yours, people will see "This cloud is not the official one of Kosmotheka: *your-domain* runs it" when they type its address, in their account and in "Sharing and devices", when they open an invitation of yours, and in the libraries that live on it. It is a notice, not a block: nothing has to be set on the server, and it does not change with another name. "Official" is exactly `https://api.kosmotheka.app`: a test server on your network also shows as not official.

### Changing server

The app only lets you change server **while signed out**. Anyone signed in to another one must sign out first (**Settings › Cloud and account › Sign out**: it deletes nothing from the device).

## Which addresses it accepts

The app checks the address before calling anything:

- **`https`**, always. This is what you should use.
- **`http`** only if the address is on the home network or on the machine itself: `localhost`, a private IP (`10.x.x.x`, `172.16.x.x` to `172.31.x.x`, `192.168.x.x`), the IPv6 loopback address (`[::1]`) or `10.0.2.2` (the Android emulator). With a name such as `server.local` or a VPN IP (for example `100.x.x.x`), `http` is rejected.
- No user name or password in the address, no path (`/something`) and no query parameters.

An address that does not meet these rules gives "This address isn't valid. Check that it's a Kosmotheka server's address." The `http` option is for testing at home: the password and the libraries would travel unencrypted.

## When the server does not answer

If the app cannot talk to the server, it says **"The server isn't responding. It may be under maintenance, or the address is wrong."** and offers **Retry**. What to check, in order:

1. Open `https://your-domain/v1/server` in a browser. It should return JSON with what your server offers.
2. If it does not open, the problem is nginx, PHP-FPM or the certificate: the [common problems](/en/run-your-server/troubleshooting/) guide has the usual symptoms.
3. If it opens in the browser but the app fails, check that the address typed in the app is the same as the public one, and that the device's clock is correct (a certificate is rejected if the date is far from the real one).

## Invitations on your own server

Whoever administers a library can invite another person **by email** or with a **link** to copy. The link has the form `<INVITATION_BASE_URL>/i/<code>`, lasts 24 hours and works once.

**On your own server, the link does not open with a tap.** Links that open the app directly (on Android) are verified when the app is built, and only the links of the official domain are verified. Yours are **copied and pasted into the app**:

1. The person who receives the link copies it (from the message or the email; there is no need to open it).
2. In the app, tap **I have an invitation** (in the sign-in dialog, in **Settings** or on the account screen), paste the link and tap **Look**.
3. The app shows which library they have been invited to and with which account they will join. Nothing happens until you tap **Join “…”** (with the name of the library).

Things worth knowing:

- The app **only accepts links from the server it is connected to**, and only if they start with the address the server publishes as `invitation_base`. A link from anywhere else is rejected with "This link belongs to another server than your account's. It was not opened."
- Whoever invites copies the link once, when it is created: if they close the window without copying it, they have to cancel the invitation and make another.
- The invitation email contains the link, but **your server does not serve any page at `/i/<code>`**: opening it in a browser gives a 404 from your nginx. It works because the app reads the code from the text, not because it opens the link. Tell the people you invite: "copy the link and paste it into the app".
- For `INVITATION_BASE_URL` use your server's address (`https://your-domain`). An address on the official domain is discarded.
