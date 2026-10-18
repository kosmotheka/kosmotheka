---
title: Glossary
description: The domain names in Spanish and in the code, so that the same thing is called the same in Rust, PHP and TypeScript.
sidebar:
  order: 5
---

The project's working documents are in Spanish and the code is in English. This table fixes how each thing is called in each place. **Look at it before you choose a new name** in the domain, in Rust, PHP or TypeScript: a term that becomes `copy` in one file and `item` in another never leaves the repository. If a term is missing, add it to [`AGENTS.md`](https://github.com/kosmotheka/kosmotheka/blob/main/AGENTS.md) before using it.

`library` is not the product: the product is **Kosmotheka**. In the code, `library` is the domain entity, a library that people share. Do not rename it along with the product.

## The catalogue

| In Spanish | In the code | What it is |
|---|---|---|
| obra | `work` | The book in the abstract: "Dune", without saying which edition |
| edición | `edition` | A published edition of a work |
| ejemplar | `copy` | What is on the shelf: format, who owns it, where it is |
| persona (autor) | `person` | Someone who contributed to a work |
| editorial | `publisher` | |
| género | `genre` | A key from the app's closed list for each kind; it is not a row |
| etiqueta | `tag` | A free word of a library, linked to works |
| serie (agrupación de obras) | `series` | Not the TV kind, which is `tv_series` |
| obra en una serie | `series_entry` | |
| «necesita» | `work_relation` with `requires` | An expansion requires its base game |

## People and what they note

| In Spanish | In the code | What it is |
|---|---|---|
| miembro de la familia | `member` | A catalogue row that owns copies. **Not** an account, though it may be linked to one |
| marca (de una persona sobre una obra) | `mark` | The status, rating and favourite of one person about one work |
| estado · valoración · favorito | `status` · `rating` · `favorite` | Fields of a `mark` |
| lo deseo | `wished` | Field of a `mark` |
| dispositivo | `device` | |
| biblioteca | `library` | On the server, the unit that owns the data |
| rol: administrador · editor · lector | `admin` · `editor` · `reader` | |
| salir de la biblioteca | `leave_for_good` | A person withdraws their own access and cuts off their own devices (`member.leave`). Not `leave_library`, which removes the library from one device and keeps the access |
| acceso retirado (de una persona) | `access_withdrawn` | Field of a `member`: the person can no longer reach the library, from any device or through their account, but stays in the catalogue. It is not a `revocation`, which is about one device |

## Operations and sync

| In Spanish | In the code | What it is |
|---|---|---|
| operación · cambio · intención | `operation` · `change` · `intent` | An **operation** is an atomic batch of entity **changes**, with an **intent** (`book.add`) that only informs |
| operación anterior (del mismo autor) | `previous` | Field of an operation: the id of the previous one by the same author in that library, or null. It forms each author's chain |
| reloj lógico híbrido | `hlc` | The mark that orders operations from several devices without depending on their clocks matching |
| lápida | `tombstone` | Marks a deleted row so that it cannot come back |
| estado de una operación: aplicada · petición | `applied` · `request` | A `request` is an operation its author could not make: it waits for whoever administers to resolve it |
| solicitud | `change_request` / `ChangeRequest` | The row that resolves a request. It is not `request`, which clashes with the HTTP request |
| retirada de acceso | `revocation` | A device that can no longer receive a library. It is synced with the library |
| valla (sobre un dispositivo) | `fence` | Set by the operation that takes power away from a device: what that device dated beyond the `trusted_through` operation is not believed |
| sello | `seal` | An applied operation that puts a fence on its own author's device (on every device, whether named or not) |
| clave avalada | `vouched_key` | Field of a `device`: the public key that whoever said whose the device is had registered for it |
| fundación (de una biblioteca) | `founding` | Its first operation: the `library` row, its first person as `admin` and the author's device assigned to that person |

## The cloud

| In Spanish | In the code | What it is |
|---|---|---|
| dispositivo de la nube | `cloud_device_id` | Field of the `library` row: the device that is the cloud in that library, the only one with the cloud's power |
| cuenta de una persona | `account_id` | Field of a `member`: a random id of the account linked to that person; only the cloud writes it |
| usuario (cuenta) | `user` | An account on the server |
| membresía | `membership` | On the server, links an account with a `member` of a library. The role is always the member's, never copied here. It is not `member` |
| clave del servidor (de una fila del catálogo) | `key` | Only on the server: the primary key of a catalogue row. The app's `id` is unique within its library and is what operations carry; the `key` never leaves the server |
| invitación | `invitation` | |

## Why there are so many security names

Fences, seals, vouched keys, founding: they all come from one question. If an operation can change who has power in a library, what happens to what a device had already done before it was taken away? The answer will be explained in the part about sync and security of this documentation. If you are only going to add a field or fix a screen, you can ignore them.
