---
title: What's new in each version
description: "What has changed in each version of Kosmotheka, from the newest to the oldest."
sidebar:
  order: 23
---

Here is each version in **full detail**. The app shows you a summary when it is updated, once only, with a link to this page; to see it again, open **Settings › About › What's new**.

When a change alters what leaves your device, the app also shows you a **switch**, so that you can decide whether you want it. Here we call it a "switch".

## Version 0.7.0

- Photos: in the form, tap the cover to set one of your own, with a photo (on a phone) or an image, or to search for it again in the catalogues after adding the ISBN or correcting the title. Yours is kept smaller and without the photo's data, and reaches the cloud with the other covers. And for the first load, on a phone, “Photo of a shelf” in Add reads the spines on the phone itself, searches each title in the catalogues and lets you tick what to add at once. It does not read everything, but it saves a lot of typing.
- “Series” are now “Sagas and collections”, so they are not mixed up with TV series. They are in the menu, with the library's things, and you also reach them from the Catalog and Missing. The screen explains what they are for and shows how much of each you have, a work's page says “2 of 6” instead of “No. 2”, putting a work on a taken number moves the following ones up one, and “+ I have it” on a work you are missing goes straight to the copy to save it. On a phone, the menu is grouped as on a computer and shows your last three libraries, with “See all”.
- Pairing without a camera is much easier: a 6-digit number shows under the QR, and on the other device “Type the number” lists the ones on your Wi-Fi with pairing open; pick yours and type the number. If the other one does not have the app yet, “The other device does not have the app yet?” leads to the download. And “Invite” is also in Share and devices.
- If a catalogue does not answer or has run out of quota, the warning stays next to the results, in Do I have it? too, and takes you to Data providers when a key fixes it. And when you filter by genre or tag, the choice no longer shows twice in the bar.
- On a computer, the top bar no longer wraps or overflows in narrow windows: the sync status takes one line (“3h ago”) and Do I have it? and Quick add keep only their icon. Settings and What’s new are wider, the welcome and “Searches through your server” have their button next to the text, and People and permissions and Sharing and devices use the whole width.

## Version 0.6.0

- On a phone, if a book's barcode is a shop label, tap “Photo of the number” in the scanner: the app reads the printed ISBN in the photo, on the phone itself, without sending it anywhere. In Add, “Type the number” brings up the number keyboard and no longer searches titles while you type it.
- In People and permissions, each device says when it last changed anything. If you reinstalled the app, it offers to remove the earlier installs at once, which used to stay in your list for good.
- When you sign in, a screen tells you which catalogues your server asks for you, what is sent to each one, and lets you turn off any of them. It shows again if your server adds a new one.
- Help now also covers the cloud and people, permissions and requests, and each topic takes you to its page of the full guide.
- In «People and permissions», the cloud's server no longer shows as a device with no person that can be assigned, you can see which one is this device, and the only-administrator warning no longer repeats the whole list when you have many.

## Version 0.5.0

- Covers now go to your other devices when they sync, over your home Wi-Fi or through the cloud: you see the same cover on all of them, even if the item was added on another one.
- Covers take up much less space: they are stored smaller and in a lighter format, including the ones you already had. They look just as good.
- When pairing two devices that already had libraries, saying whose the other one is no longer fails with a permissions error or leaves it with no owner.
- Pairing with a computer is easier: the QR code is bigger and, on computers with Docker or virtual machines, much simpler, so the phone reads it first time.

## Version 0.4.0

- With an account, when you scan a board game the app asks GameUPC, through your account's server, which game it is: no key of your own is needed any more. If it is not sure, you pick yours among its suggestions or search for it by name, and when you save it we tell it which one it was so it learns; it receives the code and the game, with an identifier of your account that does not say who you are. You can turn it off in Data providers. *(With a switch.)*

## Version 0.3.1

- When a cloud is not the official one of Kosmotheka, the app says so: when you type another server address, in the account, in “Sharing and devices”, in the library, when you receive by QR a library that lives in another cloud, and when you open an invitation from another server, always with who runs it.

## Version 0.3.0

- “Leave this library” is now called “Remove from this device”: the library disappears only from that device, also when it is in your account's cloud (the app checks at that moment that the cloud has it), and afterwards you can bring it down again from “Sharing and devices”. New: “Leave the library”, with which you lose access on all your devices and in your account; your books stay in the library, in your name.
- On the computer, the button at the top is now called “Do I have it?” instead of “Scan”, because the computer does not scan with a camera: there you type or paste the book's number.
- “Delete library” now only shows for those who administer the library, and the series list and each series count what you have the same way, gaps included (“you have 3 of 6” in both places).
- Help links to the full guide, at docs.kosmotheka.com. Covers of any proportion now fill the same box in lists, and choosing a library in the menu takes you out of Help and to its list.
- The ways to remove a device have clearer names (“Unpair”, “Remove this device’s access”, “Remove from my account” and “Reset this device”), and the app tells you before you create an account that the cloud is, for now, by invitation.

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
