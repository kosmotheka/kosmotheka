---
title: Error messages and what to do
description: "The notices Kosmotheka may show when something does not work, what they mean and what to do in each case."
sidebar:
  order: 20
---

Almost all the app's notices are fixed in a minute. Here are the ones you may really come across, written as they appear on screen, with what they mean and what you can do. If you are looking for one, use your browser's search (Ctrl+F) with the first few words.

One rule that helps: **the app works without the internet and without an account**. Almost no notice on this page stops you from using it; they only tell you that something, such as looking up details or syncing, could not be done just now.

## Offline

| What you see | What it means | What to do |
|---|---|---|
| "The search failed. Check your connection." | The app could not ask any catalogue: there is no internet or the service does not answer. | Check that you have internet and try again. Meanwhile, fill in the details by hand: only the title is required. |
| "Could not reach *the source's name*." | That particular catalogue did not answer. The others did. | It is usually temporary. Try again in a few minutes. If the result you want appears through another catalogue, you do not need to do anything. |
| "Not found in the online catalogues. You can fill it in by hand." | No catalogue knows that book, record or game. It happens with unusual or very old editions. | Fill it in by hand. It is not a fault. |
| "Not connected to the cloud" | The app cannot reach your account's server. | Everything keeps working on your device and over your home Wi-Fi. Changes will be sent by themselves when the connection returns. |
| "Not live. Changes from the cloud arrive when the app syncs" | The server does not notify right away (or there is no connection). | Nothing to do: changes will arrive the next time the app syncs. If you like, open it from time to time. |
| "This server does not send live updates." | Your server does not have instant updates. | Nothing to do: changes arrive when the app catches up. |

## The server does not answer

| What you see | What it means | What to do |
|---|---|---|
| "The server isn't responding. It may be under maintenance, or the address is wrong." | The app cannot talk to the server. | Tap **Try again** in a few minutes. If you use your own server, check that the address is spelled correctly. Meanwhile, the app keeps working. |
| "This address isn't valid. Check that it's a Kosmotheka server's address." | The server address is spelled wrongly, or it is not one the app accepts. | Copy the full address, starting with `https://` and with nothing after it. |
| "This app version cannot sync with the server." | The app and the server are very different versions. | Update the app and try again. |
| "The app and the server run different versions. Update the app." | The same, when uploading or downloading a library. | Update the app. |
| "Something went wrong with the cloud. Try again." | A general cloud fault. | Try again. If it keeps happening, tell whoever runs the server. |
| "Something went wrong." (on the account screens) | The server refused something it did not expect. | Try again. If it keeps happening, tell someone. |
| "Too many requests. Wait *n* minutes and try again." | You have tried many times in a row and the server makes you wait. | Wait the time it says and try again. |

## The account

### Signing in and creating the account

| What you see | What it means | What to do |
|---|---|---|
| "The email or password is incorrect." | One of the two does not match. | Check how the email is written. If you do not remember the password, tap **Reset password**. |
| "Your email isn't confirmed yet." | You created the account but did not type the code that was sent to you. | Tap **Send another code** and type it. Accounts that are not confirmed are deleted the next day. |
| "Incorrect code." | The code is not the one we sent you. | Use the one in the latest email you received: if you asked for several, only the last one works. |
| "This code has expired." | Codes last 15 minutes. | Ask for another with **Send another code** and use it as soon as it arrives. |
| "Too many attempts with this code." | You failed several times and the code no longer works. | Ask for a new one with **Get a new code**. |
| "This server is not taking new accounts for now." | The server only lets some people create an account. | If you already have an account, sign in. If you expected to be able to create one, talk to whoever runs the server. |
| "Your password needs at least 8 characters." | It is too short. | Choose a longer one. |
| "This account is locked." | Whoever runs the server has locked it. | Write to them. If you think it is a mistake, they can sort it out. |
| "This device is signed in to another account." | This phone or computer is already in use with another account. | To use it with yours, go to **Settings › This device** and hand it to someone else: it is left as if newly installed. Careful: it deletes what was on it (make a backup first). |
| "Your account knows this device with another key." | The account remembers this device from before, from another installation of the app. | Remove it from your account on another device, in **Account › My devices**, and then sign in here again. |

### Signing in with Google

| What you see | What to do |
|---|---|
| "You cancelled the sign-in with Google." | Nothing. If it was by mistake, start again. |
| "The sign-in with Google took too long." | Start again from the app. |
| "Google has not confirmed your email." | Confirm the address in your Google account, or sign in with your email and password. |
| "This email belongs to another Google account." | Sign in with the Google account you used before, or with your email and password. |
| "Google sign-in did not work." | Try again in a moment, or sign in with your email and password. |

### The second factor (code from the authenticator app)

| What you see | What it means | What to do |
|---|---|---|
| "This account asks for the code from its app." | You have turned on the second factor. | Type the 6-digit code shown by your authenticator app. |
| "That code isn't right." | The code changes every 30 seconds, and the one you typed no longer works, or the phone's time is wrong. | Type the one showing now. Check that the phone's time is set automatically. If you do not have the app, use a **backup code**. |
| "That backup code doesn't work." | Each backup code works only once. | Try another one from your list. |
| "The second factor was already on." / "The second factor isn't on." | What you see on the screen is out of date. | Open the account screen again. |
| "There's no setup in progress." | The screen was closed halfway. | Tap **Turn on** again. |

If you lose your phone **and** your backup codes, see [I lost my phone or changed it](../lost-phone/): whoever runs the server can remove your second factor.

### When you are signed out

| What you see | What it means | What to do |
|---|---|---|
| "You've been signed out. Your account's password was changed on another device." | You changed the password somewhere else and, for safety, the other sessions were closed. | Sign in with the new password. Nothing is lost: what had not been uploaded is sent when you sign back in. |
| "This device was removed from an account." | Someone removed it from another device on the account. Its libraries have been deleted from here. | The app is left empty and ready to use, with or without an account. If it was your account, sign in again and what you had in the cloud will come back. |
| "Sign in again to continue." | The session was not open. | Tap **Sign in**. |

## Sharing and pairing

### Pairing two devices over Wi-Fi

| What you see | What it means | What to do |
|---|---|---|
| "That is not a pairing code." | What you scanned or pasted is not the code. | Scan the QR code or copy the whole code. |
| "That code belongs to this same device." | You are reading the code on the same device that shows it. | The code is shown on one device and read on **another**. |
| "The other device cannot be reached. Is it on the same Wi-Fi, with the app open?" | The two cannot see each other. | Check that they are on the same Wi-Fi (not mobile data or the guest network) and that the app is open with the code on screen. A VPN or the firewall can also get in the way. |
| "The other device refused: the code expired or was already used, or it forgot this device." | Codes work once and only for a short time. | On the other device, generate a new code. If it had forgotten this device, pair again. |
| "The device that answered is not the one in the code. Pairing was stopped for safety." | Another device on the network answered instead of the one you expected. | This is a protection. Check that you are on your own network, generate a new code and try again. |
| "The other device already has this one paired." | They were already paired. | To pair them again, first forget this device on the other one. |
| "The two devices run different versions of the app. Update both." | One of the two is out of date. | Update both. |
| "The other device had a problem. Try again." | A fault on the other device. | Try again; if it keeps happening, restart the app on both. |
| "This device is not paired." | The other one had forgotten it. | Pair again from **Devices**. |
| "This device is not connected to any network." | There is no Wi-Fi. | Connect to your home Wi-Fi. |

Below these notices the app says: **"Nothing was lost: they will sync on their own once it is solved."** That is true: nothing in your catalogue is lost because a pairing failed. More about this in [Share at home](../share-at-home/).

### Inviting and sharing a library

| What you see | What it means | What to do |
|---|---|---|
| "This server does not make invitations." | Your server does not have them turned on. | You can give access with the QR code. |
| "Only whoever administers the library can invite." | You do not have that role. | Ask whoever administers the library. |
| "This library cannot invite yet: it needs people with roles and the cloud allowed to link devices." | A setting in the library is missing. | Check that there are people with roles and that the option for the cloud to give access to new devices is turned on. |
| "This link was already used." / "It has expired: ask whoever administers for a new one." / "This invitation was cancelled." | The link no longer works. It works once and for 24 hours. | Ask for a new one. |
| "This invitation is for another account." | It was sent to another email. | Sign in with the account it was sent to. |
| "Your account is already another person in this library." | You are already in with another name. | Nothing to do. |
| "This link belongs to another server than your account's. It was not opened." | You are connected to one server and the link is from another. | Sign out and use the server from the invitation. |
| "That is not an invitation link." / "This link is not valid." | The pasted text is wrong or incomplete. | Paste the full link again. |
| "Only whoever administers this library can upload it. Meanwhile, it keeps sharing over your home network." | Uploading it to the cloud is up to whoever administers it. | Ask them, or keep sharing it over Wi-Fi. |
| "The cloud says you no longer have access to this library." | Whoever administers it removed you. | Talk to whoever administers it. |
| "This library was started with a very old version of the app, so it cannot be uploaded as it is. Tap "Duplicate as new": it makes another one with the same books, and that one can be uploaded." | It is a very old test library. | Tap **Duplicate as new**: another one is created with the same books, and that one can be uploaded. |

## Backups

| What you see | What it means | What to do |
|---|---|---|
| "This file is not a backup of this app." | You chose a file that is not a backup. | Choose a file created with Kosmotheka's backup. A CSV is loaded with **Import CSV**. |
| "The backup is damaged and cannot be read." | The file is broken, or the download was cut off. | Try another, older backup. If you sent it by email or through the cloud, download it again. |
| "This backup was made with a different version of the app." | The backup is from an incompatible version. | Update the app and try again. |
| "This backup is protected: enter its password." | The backup has a password. | Type it. |
| "Wrong password. Nothing was restored." | It does not match. | Try again. If you have lost it, **the backup cannot be opened**: there is no way to recover it. |
| "Enter a password for the backup." | You chose to protect the backup and did not type a password. | Type one, or untick the protection (the backup would then be readable by anyone who finds it). |
| "This backup comes from an earlier version and only restores into an empty library. Create a new library, open it and restore it there." | An old backup is not merged into a library that already has things in it. | Do what it says. |
| "This backup was made with a development version that used another format, and it cannot be opened." | It only happens with backups from the first tests. | There is no way to open it; make a new backup. |

More in [Backups, export and import](../backups/).

## When adding or editing a book

These notices appear in red under the field, and the form does not close.

| What you see | What to do |
|---|---|
| "This field is required." | Type at least the title. The rest can be filled in later. |
| "An ISBN has 10 or 13 digits." | Copy it from the back cover or the credits page. Hyphens and spaces do not matter. |
| "An ISBN may only contain digits (and a final X)." | Check that no letter has slipped in. |
| "This ISBN is not valid: check for a typo." | Check each digit, or scan it with the camera. |
| "This is not an ISBN: ISBNs start with 978 or 979. Look for the number next to the book's barcode." | Look for the small number next to the barcode. |
| "That was a shop label, not the ISBN. Type the small number that starts with 978 or 979, or scan again aiming at its code." | Type the small number that starts with 978 or 979, or scan again, pointing at its code. |
| "Another book in the catalog already has this ISBN." | If it is the same book, add a copy from its details page instead of changing this one. |
| "Another edition already has this barcode." | The same: you probably have it already. |
| "This value is out of range." / "This is not a valid date." | Check the year (between 1 and 2100), the units or the date. |
| "*The source's name* rejected the stored key. Check it in Settings → Data providers." | You put in your own key for a catalogue and it no longer works. Ask for a new one, or delete it to use the service without a key. |
| "Google Books refused the request (quota exhausted)." | Google limits searches without a key. Put in your own free key in Settings. |

## If the notice is not here

- **"Something went wrong: …"** with a technical text: try again. If it keeps happening, copy the text exactly and tell whoever runs the server, or open an issue in the project.
- Anything odd with your data: make a [backup](../backups/) first. Almost nothing you see on this page deletes anything; if a notice asks you to do something that does (for example handing a device to someone else), the backup covers you.
