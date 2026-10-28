---
title: "Security: the threat model"
description: Who Kosmotheka protects against, what each piece protects and, above all, what it does not protect.
sidebar:
  order: 8
---

This page describes **the model**: who we protect against, with which pieces and with which limits. It does not list specific flaws; to report one, see [Report a security problem](../report-a-vulnerability/). To understand the pieces it is best to have read [The sync engine](../sync-engine/) first. The names the code uses are in the [glossary](../glossary/).

## The idea, in four sentences

1. Your data lives on your devices. The app works completely without a server and without an account.
2. A library only reaches whoever a family member has given it to: with a QR code at home, or through the cloud if whoever administers it decided to upload it.
3. Inside a library there are roles (whoever administers, whoever edits, whoever only reads). **Each device checks on its own** that the others respect them; no server watching is needed.
4. Nothing is lost: what someone tries without having permission is not deleted; it stays as a **request** that whoever administers accepts or rejects.

## Who we protect against

There are two borders, and it is best not to mix them.

| Border | Example | What is at stake |
|---|---|---|
| **Outsiders** | A neighbour on the same Wi-Fi, someone on the internet, whoever finds a phone | Reading your library: what you have, where it is, whose it is |
| **Insiders** | A family member who only reads and installs a modified version of the app to get around their role | Changing or deleting the catalog, or making themselves an administrator. Reading, they could already do |

The first is the most important. The second needs more mechanisms, because whoever attacks **already has a copy of the library** and a device that the others know.

In more detail, these are the attackers we consider:

- **A reader** with their device and a full copy: wants to change or delete the catalog, touch others' things or share without permission.
- **An editor:** wants to become an administrator, delete what they did not create or link devices.
- **A former member, or a device whose access was withdrawn,** with its copy and its key: wants to keep reading or writing.
- **An equipment on the same network, never paired:** wants to read the traffic, impersonate someone or pair using a QR code seen from afar.
- **A stolen phone,** locked or unlocked.
- **Someone on the internet** against a server: taking over accounts, reading other people's libraries or abusing the email.
- **A hostile data service** (the one that gives us a book's data): injecting content or tracking through images.
- **A malicious dependency** (a package we use).
- **Whoever runs the server:** can read and edit what is uploaded to it.

And the trust borders where the defences apply: between the interface and the core of the app, between two devices, between a device and the server, between the app and the data services, between the server and its database, and between the repository and continuous integration.

## What each piece protects

Each piece says what problem it solves, with an example.

### 1. Every change is signed

**Problem:** someone makes a change pretending to be another person's phone.

Each device has a secret key that **never leaves it**, and signs each operation with it. The others know its public key and check the signature. An operation without a signature, or with one that does not match, is discarded. A device's key never changes: if a different one arrives for the same device, it is rejected.

### 2. Each device checks the permissions

**Problem:** a reader edits the catalog with a modified app.

When an operation arrives, each device looks at who signed it, which person that device belongs to and what role that person had **at that moment**. If they could not do it, it is not applied: it stays as a request. All devices, and the server too, follow the same rules and reach the same result, whatever order they receive the changes in. The rules are written only once, with [test cases](../operations-spec/) that the app and the server both pass.

### 3. A change cannot be slipped in "dated yesterday"

**Problem:** Hugo loses the editor role today. With a modified app, he writes a change and dates it last week, when he was still an editor.

Three defences:

- **Each device numbers its own.** Each operation says which was the previous one of that same device (`previous`), so a new one cannot be slipped in between the old ones without it being noticed.
- **Removing power notes how far the device is believed.** Whoever removes a device's role notes, in that same operation, which is the last operation of that device they have seen. Whatever appears afterwards with an earlier date and does not hang from it is not believed: it stays as a request. That mark is called a **fence**.
- **A library is born with its administrator.** There is no moment in which it goes from having no roles to having them, which could be dated backwards: the first operation is the **founding**, and it only counts if the library's id comes from the key of whoever signs it.

### 4. A library only reaches whoever it is given to

**Problem:** someone on the same network receives your library.

- **At home**, two devices pair with a QR code shown on screen. From then on they talk encrypted, and each recognises the other by its certificate, not by its network address. The code is single-use and expires after ten minutes.
- **Only whoever administers a library, or has the "Can share" permission, can give it to another device.**
- **Each device chooses what it sends:** a library travels only if both sides share it.
- **Withdrawing access:** whoever administers can say that a device no longer receives a library; the others stop handing it over and do not accept what it signs afterwards.
- **Removing a person:** whoever administers can say that a person no longer enters the library. Their devices stop counting as theirs and their access is withdrawn, so they delete the library when they connect. The person stays in the catalog, with their name and what they noted. The last administrator cannot be removed.

### 5. The cloud: the account and the access list

**Problem:** an account reads other people's libraries.

- The cloud is optional. **Only whoever administers a library can upload it**; signing up uploads nothing.
- The server keeps a list: which accounts can enter which library. **It is the only thing that decides access.** It never trusts an identifier that the app sends: it compares it with what it knows, and does not use it to look things up.
- For a library that is not yours, or that does not exist, it gives the same answer, so that nobody can find out which ones exist.
- It applies to everything it stores the same role rules as the devices.
- If a person is removed or a device withdrawn, the cloud **stops handing them the library immediately**, even if they enter from a new phone. It shows the cut-off device the signed proof from whoever administers, and the device only deletes the library if that proof is good: a server that lied could not make a library be lost.

### 6. The cloud is one more device, with counted powers

**Problem:** you buy a new phone. Nobody at home knows it, so it has no role until someone pairs it by hand. And if you were the only administrator and lost your only phone, nobody can.

When a library is uploaded, the cloud enters it as one more device, with its own key stored encrypted. Whoever administers gives it **a single power**: to say "this new device is Laura's" when Laura has entered it with her account. It cannot change roles, touch the catalog (except emptying the trash, below) or remove a device from anyone. The cloud's identifier in a library **comes from its key**, so nobody can present that identifier with another key. Whoever does not trust their server can remove that power; then each new device is paired by hand.

With the trash it has **one more, and nothing else over the catalog**: deleting for good what has been **60 days** in the trash, in case no device of the library connects in that time. A cloud that tries to delete earlier, or something that is not in the trash, has its operation turned into a request, in the app and on the server. Accepted risk: a compromised cloud could date its operation in its favour and delete early, but only what a person had already deleted.

### 7. Saying whose a device is includes its key

**Problem:** a device's identifier is not secret; everyone in the library sees it. Someone with a modified app could present the identifier of an administrator's new tablet, with a key of their own, before the real one arrives. If the administrator then says "that tablet is mine", the impostor's would be accepted.

That is why whoever says "this device is Laura's" also notes **the key they have seen** for that device (the **vouched key**). A device only counts as Laura's if its key is that one. It is the same idea as "linking a device" in messaging apps.

### 8. Inviting someone from outside

**Problem:** an invitation link travels through a chat or an email and may end up in hands that are not that person's.

- **Only whoever administers can invite**, and the server checks it in its own lists. If the inviter stops administering before it is accepted, the invitation is no longer valid.
- **An invitation is valid once, and for 24 hours.** The server does not keep the link, only a fingerprint of it. The link is a very long random number, and the server cuts off whoever tries many times.
- **There are two kinds.** *By email:* only an account registered with that same email can open it. *Link to copy:* whoever arrives first opens it, so **it only gives the reader role, and only to a new person**, so that a leaked link does not let a stranger take the place of a family member.
- **Everything is checked again when accepting**, not only when creating.
- **The server cannot be used to send email to strangers:** there are limits per account and per address, and the text of the email is fixed, with nothing that the inviter writes.

### 9. A record of what happened

**Problem:** someone says "I did not delete that" or "I was removed for no reason", and the library or the account is no longer there.

The server notes, in a table where entries are only added, what happened, which account and device it is due to, when, and from which address. It notes it **with what it knows**, never with what the app said. It does not store passwords, codes, links or emails (for a deleted account, only a fingerprint). It is kept for a year and then deleted automatically.

### 10. Signing in with Google

**Problem:** Google is the entry door for every account that uses it, and the return to the app travels through a link that any other app or page can launch.

- What travels through the browser **is no use on its own**: the app starts with a secret that never leaves it, and the code that comes back gives no access without that secret.
- **The app only accepts the return of an attempt it started** and is waiting for.
- The server ties the return to the browser that left, with a single-use value.
- Only an email that Google confirms counts, and the person is recognised by their Google identifier, not by the email, which can be changed.
- No Google token is stored.

### 11. A deletion can be undone

**Problem:** a wrong tap, or someone with the editor role deleting what they should not, and the entry disappears from every device.

What is deleted spends 30 days in the **trash** before it goes, with who deleted it and when. An editor cannot delete anything for good before those 30 days: only whoever administers can. And `trashed_by` must be the person of the device that signs, checked like everything else, so nobody can blame someone else for a deletion.

### Other protections

- **Backups:** they are encrypted with a password by default, and **none carries the signing key, the account token or the identity of the local network**.
- **Data services:** the app treats their responses as untrusted. Covers are only downloaded over `https` from known servers and are stored by their fingerprint; the text is shown without being interpreted as code.
- **Request limits** on the server, per connection and per account, and a temporary lock of an account after several failed attempts.

## What it does not protect against

Stated plainly, so that nobody takes it for granted:

- **Whoever administers the server can read the libraries uploaded to it.** There is no end-to-end encryption: the operations travel encrypted, but on the server they are stored so that it can check permissions. That is why the cloud is optional, you can install your own server, and uploading a library is a decision for whoever administers it. **Full privacy is using the app without an account.**
- **Whoever can read a library can copy it.** No role prevents it.
- **A withdrawn device keeps what it already had copied.** The cloud stops handing it anything more, and its app deletes the library when it sees the proof, but a modified app may refuse to delete it.
- **An unlocked phone in someone else's hands acts as its owner,** until a family member removes it or the account's password is changed.
- **A family member with a modified app can cause trouble:** for example, trying to stop a new device from entering. They gain no access or role, and whoever administers can remove them.
- **An editor with a modified app can fork their own chain** (two operations with the same previous one), and only the devices that were behind receive it. The chain makes this detectable, but it is not resolved: it is limited to whoever could already edit and to what they could already do.
- **Pairing through someone who wants to deceive.** When a new device is given access, if someone in the middle manages to get their key to the admin's device before the real one, that device would accept it. It shows, because the real device cannot sync with anyone, and then the fake one's access is removed. It is an accepted risk ([D96](https://github.com/kosmotheka/kosmotheka/blob/main/docs/DECISIONS.md)).
- **A badly set clock dates its changes wrongly,** although never more than an hour ahead of the receiver's.
- **A copy-to-share link is used by whoever opens it first.** That is why it only gives the lowest role, only to a new person, and it expires in one day; if more security is needed, invite by email.
- **Whoever knows your email can lock your account for a while:** five failed passwords in a quarter of an hour block sign-in attempts to that account, yours included, until that time passes. It is the price of nobody being able to guess a password from many places at once.
- **The activity record is only as reliable as the server:** whoever administers it can read it and, with access to the database, change it. It serves to answer a complaint, not for forensic work.
- **A sole administrator who loses their only device loses control** of that library, and nobody can change permissions. The last resort is to duplicate the library as a new one.

## How it is checked

- **The hostile case is tested before the defence is written**, and it is seen to fail: a modified app, someone who knows the identifiers, a forged notice.
- **The permission rules are shared cases:** the same files are run by the app (Rust) and the server (PHP).
- **Periodic audits** with a written method that anyone can repeat: [How we audit](../audit-method/).

The model is reviewed when the security decisions change. The method and the closed findings are published; open ones, once they are closed.
