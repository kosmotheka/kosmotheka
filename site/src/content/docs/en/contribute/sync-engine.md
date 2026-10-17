---
title: "The sync engine, explained"
description: How several devices with the same books reach the same result without anyone in charge, and why it was designed this way.
sidebar:
  order: 6
---

This is the central page of the technical part. It explains how Kosmotheka makes sure that two devices that change the same book, without a connection, end up seeing **exactly the same thing**. If you do not know the project, read it from start to finish: it goes from the simple to the hard, and each term is explained the first time it appears. The names that repeat in the code are in the [glossary](../glossary/).

## The idea, in four sentences

1. Each device keeps **all the operations**: its own and the ones it has received. An operation is a batch of changes with an intent label, such as "add this book".
2. Syncing means **giving each other the operations the other is missing**.
3. Each device applies the operations with **rules that do not depend on the order** in which they arrive.
4. That is why they all end in the same state, **with no referee**: no device or server is needed to decide.

The rest of the page explains each piece and why it is needed.

## An example we will follow

Ana and Hugo share the library "Home". Ana has the phone; Hugo, the laptop. Both have *Dune*, with one copy in paperback (in the living room, one unit), and also *Rayuela*. Now both are **without a connection** (each in their own place) and they do things:

| When (by their clock) | Who | What they do |
|---|---|---|
| 10:00 | Ana | Changes the copy's location to "Shelf 2" and writes a note: "lent once" |
| 10:05 | Hugo | Changes the location of the same copy to "Study" and sets 2 units |
| 10:10 | Hugo | Deletes the edition of another book, *Rayuela* |
| 10:20 | Ana | Changes the year of *Rayuela*, not knowing that Hugo has already deleted it |

Later they connect. What should happen? Ana and Hugo must see the same thing. The reasonable outcome is:

- The location of the *Dune* copy is only one, not two. "Study" wins, because Hugo changed it later.
- But **nothing else is lost**: Ana's note and Hugo's two units stay, because they changed different fields.
- *Rayuela* is deleted for both, even though Ana edited it later.

Let's see how the pieces achieve this.

## 1. Operations and the log

No part of the app writes directly to the catalog tables. Each action ("save the details") becomes an **operation**: an atomic batch of changes (create, modify or delete a row of an entity) with an intent label such as `book.add`. The operation is written to a **log** (the `operations` table), and its changes are applied **in the same transaction**: either everything happens, or nothing does.

*Why this way:* with operations, syncing is just swapping operations, the server can check the permissions over its changes, and the history of each book comes for free from the log. The exact form is in [The operations](../operations-spec/).

In the example, Ana's operation carries two changes on a copy row: its location and its note. Hugo's carries two others: location and units.

## 2. The hybrid logical clock: putting things in order without a shared clock

To decide who changed something "later", the operations have to be put in order. But the devices' clocks do not match: one may be ahead, or someone may change the time. A **hybrid logical clock** (HLC) solves this. Each operation carries a mark with three parts:

- the **milliseconds** of the device's clock;
- a **counter**, which breaks ties if two operations fall in the same millisecond or the clock goes back;
- the **identifier of the device**, which breaks ties between different devices.

It is written with leading zeros, for example `0001727222222222-00000003-<device id>`, so that sorting the marks as text is the same as sorting them in time.

Two properties matter:

- **A device's mark never goes back**, even if the system clock does.
- **When it receives operations, the clock jumps ahead of what it received.** So what you do afterwards is always "later" than what you have already seen, even if the other device's clock was ahead.

And a safety limit: an operation whose time is **more than an hour ahead** of the receiver's clock is rejected. Without it, a device with its clock set to the year 2099 would win every conflict forever.

Since each mark includes the device identifier, **two different operations never have the same mark**. An honest app does not sign two with the same one; if two arrive, the second is rejected.

## 3. Merging: field by field, and the highest mark wins

When an operation arrives, each device applies its changes with these rules:

| Situation | What is done |
|---|---|
| The operation was already known (same id) | It is ignored. Syncing twice changes nothing |
| Modify a row | **Field by field**: each field is written only if the operation's mark is higher than the mark of whoever last wrote it |
| Create a row that already exists | It is treated as a modification |
| Modify a row that does not exist | It is ignored (it had been deleted) |
| Delete | The row is deleted and a **tombstone** is left (see below) |
| Change a row that has a tombstone | It is ignored: **the deletion wins** |

To be able to decide "field by field", each device keeps, for each field of each row, the mark of the operation that last wrote it (the `field_versions` table).

**In the example.** The copy has three touched fields: location (10:00 Ana, 10:05 Hugo), note (only Ana) and units (only Hugo). The location is won by the 10:05 mark: "Study". The note and the units, which nobody else touched, stay. That is what we wanted, and it is achieved **without the two devices having to talk** beyond swapping operations.

*Why "field by field" and not "the whole row":* if the whole row won, Hugo's operation would erase Ana's note, which he did not even know about.

### Why it always converges

This is the important part, and it has a short explanation. Each rule above does three things:

- the result **does not depend on the order** in which the operations arrive (for each field, the highest mark wins, among those that arrive first or later);
- **repeating an operation changes nothing** (it is ignored if already known);
- **there are no ties**, because each mark is unique.

A system like this ends up the same, whatever it receives and in whatever order. And it is not only reasoned: the convergence tests simulate several devices, deliver the same operations in different orders, and check with `assert_converged` that they all end up equal. They also check that **recalculating the whole library from the log** (see below) leaves it exactly the same.

## 4. Tombstones: a deletion must not come back to life

If Hugo deletes *Rayuela* and Ana, without knowing it, edits it afterwards, Ana's edition must not bring it back to life. That is why a deletion leaves a **tombstone**: a mark that says "this row was deleted" and with which mark. A later change to a row with a tombstone is ignored. **The deletion always wins.**

*Why this way:* the alternative is losing the deletion when another device touches the row at the same time. We accept that someone may lose an edit in exchange for a deletion that cannot undo itself.

Some cases involve dependents: when a work is deleted, its editions and the copies of those editions are deleted in a chain. An **optional** reference to something deleted (the publisher of an edition, the owner of a copy) is left empty, instead of rejecting the change. A **required** reference to something that no longer exists makes the change be ignored, because the row would have disappeared in cascade anyway.

**In the example.** The edition of *Rayuela* has a tombstone since 10:10; Ana's edit, from 10:20, is ignored. Both end up without *Rayuela*.

## 5. How they ask for what they are missing: the version vector

How does a device know which operations the other one lacks? For each library and each author of operations, each device computes **the latest mark it has**: its **version vector**. It is read from the log, with no extra tables. A sync between A and B goes like this:

1. A sends its vector to B.
2. B replies with the operations that A does not have (each author's operations after what A's vector says) and with its own vector.
3. A applies them.
4. A sends B the ones B is missing, according to B's vector.

Operations from third parties are also passed on: if A syncs with B and B with C, what A has reaches C through B. That way a phone can act as a bridge between another phone and the laptop.

**How they talk:** over the home network, after pairing with a QR code. The QR contains a single-use secret and the other device's certificate fingerprint; that way each recognises the other, and the connection is encrypted, with no passwords. The data does not travel in the QR (it does not fit): it travels over the local network.

## 6. Signatures and each author's chain

So far everything assumes honest devices. But a device may have been modified, or a peer may try to pass as another. There are two defences in the operation's own format.

**Signature.** Each device has a secret key that never leaves it, and **signs every operation** it makes. The others know its public key, which travels with the library, and check the signature when they receive it. An operation without a signature, or with one that does not match, is discarded. A device's key never changes: if a different one arrives for the same device, it is rejected.

**Chain.** Each operation carries a `previous` field: the id of the **previous operation of the same device in that library**, or null if it is the first. It is signed together with the rest. This does two things:

- Whoever receives an operation **knows whether an earlier one is missing**. If the `previous` is not in its log, the operation **waits**, together with the next ones from its author, and the next sync will ask for all the missing ones. An operation that is missing is never skipped, so the version vector cannot leave a gap that nobody sees.
- Nobody can **slip a new operation in between** a device's old ones without it being noticed (it will be seen below why this matters).

## 7. Permissions are checked on receipt

Each received operation is judged with the **role its author had at that moment**: administrator, editor or reader. If all its changes are allowed, it is applied. If any is not, **the whole operation is stored as a request**: it stays in the log and travels to the other devices, but it is not applied, until the administrator accepts or rejects it. It is all or nothing, and nothing is lost.

There is a hard case. If Hugo loses the editor role while his laptop is offline, what he does afterwards **stops being valid**. But that can reach another device *after* operations Hugo had already made. When something arrives with a mark older than what has already been applied, the device **recalculates the library from its log**: it empties its content, applies all the operations again in mark order, and judges each one with the roles of its moment. That way everyone reaches the same result, in whatever order.

One more protection is needed: the mark of an operation is set by whoever signs it. If Hugo loses his role today, with a modified app he could sign a change and date it last week, when he was still an editor. Against that, whoever removes power notes in that same operation **up to which operation of the device they believe** (the last one they had seen, through its `previous`). This is a **fence**: whatever appears from that device with an earlier date and does not hang from that operation is not believed; it stays as a request. And a library is born already with its administrator (the **founding**), so there is no earlier moment in which there were no roles and something could be dated backwards.

The full threat model is in [Security: the threat model](../security/).

## 8. What the server does, when there is one

If someone uploads a library to a server, **the server is one more device with a difference: it is the authority**. When an operation reaches it:

1. It checks that it is well formed (the same validation as the app, with the same test cases).
2. It checks the **signature** of the device that made it.
3. It checks the **permissions** with the same rules as the devices.
4. If everything is in order, it applies it to PostgreSQL with **the same merge engine**, written again in PHP, and notifies the other devices.

Each device asks the server for what it lacks with the same version vector. The server **never trusts an identifier sent by the client**: it compares it with what it knows, and access comes from the account and from its own list of who enters which library. The rules are written once, in `spec/`, and the same conformance cases run them in Rust and in PHP.

Without a server, everything above keeps working between paired devices. The server adds that the library gets updated even when the devices never share a network.

## Why this way and not another

This is the project's assessment, not a comparative study: it says why we discarded other ways for a family catalog with roles.

**A single central server.** It is the simplest: the server decides and the clients ask it. We discarded it as the only option because the app has to work **without signal** (in a shop, checking "do I have it already?") and **without an account**: for many people, the best privacy is that there is no server. That is why the app always writes to its own database, and the server is optional. But we kept from this idea what is really needed: when there is a server, it is the authority and checks the permissions.

**Syncing files** (copying the database or an exported file between devices, or using a shared-folder service). It is easy to understand, but when two devices change at the same time, **a whole file wins** and the other's changes are lost, or conflicting copies are created that must be merged by hand. An open database also cannot be copied safely while it is being written. And there is no way to say who may change what.

**Connecting the devices over the internet through a relay** (a service that carries the traffic between them, for example with WebRTC). It allows syncing outside home without opening ports, but it requires **someone to run that relay**, who sees who talks to whom and when. At home it is not needed: two devices on the same network talk directly, with a QR and no one in the middle. Outside home there is the optional server, which also understands the data and can check permissions.

**Document CRDTs** (structures such as Automerge or Yjs, designed so that several people can edit the same text at once). They guarantee convergence without a referee, and the idea that a structure converges by construction is the one we followed. But they are designed for documents, not for a relational catalog with roles, and it is hard to impose permissions ("a reader does not delete") when each copy can decide for itself. We took from them what is useful, which is convergence, with smaller pieces: marks per field, tombstones and explicit operations.

**An off-the-shelf sync engine** (there are several that sync PostgreSQL with SQLite on the clients). Two things made us choose our own: those engines start from a server (the writes go through your own server), and we wanted two phones to sync with none; and one of the project's goals is to understand how it works. We did learn from them: keeping pending operations in a local table, and asking for "what comes after my point" instead of copying everything again.

**Operations with their own names** (one operation for "lend", another for "delete"…), instead of batches of changes with a label. Each type would define its own merge, and it would have to be written twice (Rust and PHP). With generic changes, the merge is written once for everything, and permissions are checked over the **changes**, not over the label: nobody can smuggle a change under an innocent name.

The decisions behind all this, with their alternatives and dates, are in the [decision log](https://github.com/kosmotheka/kosmotheka/blob/main/docs/DECISIONS.md): D05 (local first, authoritative server), D13 (local sync by QR), D25 (shape of the operations), D31 (format versions), D84 (access to the cloud), D87 (each author's chain).

## What it does not do

So that nobody takes it for granted:

- **Two changes to the same field at the same time: one wins.** The other stays in the log, but stops being seen. It is the "last one to write wins" rule, without warning.
- **Duplicates do not merge by themselves.** If two devices add, offline, the same author, the same publisher or the same ISBN, both are kept: rejecting one would lose data, and merging them blindly could join different things.
- **Two devices with very different versions of the app do not sync.** The app asks for an update first.
- **What has already been handed over cannot be recovered.** If a device's access is removed, it keeps what it already had copied.
- **An editor with a modified app can fork their own chain** (two operations with the same previous one), and only the devices that were behind receive it. The chain makes this detectable, but it is not resolved: it is an accepted risk, limited to whoever could already edit and to what they could already do.

## Where it is in the code

- `app/src-tauri/src/sync.rs`: the version vector (`version_vectors`), what the other lacks (`operations_since`) and the merge (`merge`, `apply_change`), with the tests that simulate devices.
- `app/src-tauri/src/ops.rs`: the local operations, which use the same rules.
- `crates/domain/src/hlc.rs`, `operation.rs`, `permission.rs`, `signature.rs`: the clock, the validation, the permissions and the signatures, without a database.
- `app/src-tauri/src/lan/`: pairing, certificate and transport over the local network.
- `server/src/Sync/`: the server; `Sync/Merge/` is the merge engine.
- `spec/conformance/`: the cases that Rust and PHP share.

Next: [the operations and the specification](../operations-spec/).
