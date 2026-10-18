---
title: Glosario
description: Los nombres del dominio en español y en el código, para que una misma cosa se llame igual en Rust, PHP y TypeScript.
sidebar:
  order: 5
---

La documentación de trabajo del proyecto está en español y el código en inglés. Esta tabla fija cómo se llama cada cosa en cada sitio. **Mírala antes de poner un nombre nuevo en el dominio**, en Rust, PHP o TypeScript: un término que se vuelve `copy` en un archivo y `item` en otro nunca llega a salir del repositorio. Si falta un término, añádelo a [`AGENTS.md`](https://github.com/kosmotheka/kosmotheka/blob/main/AGENTS.md) antes de usarlo.

`library` no es el producto: el producto es **Kosmotheka**. En el código, `library` es la entidad del dominio, una biblioteca que comparten personas. No lo renombres junto con el producto.

## El catálogo

| En español | En el código | Qué es |
|---|---|---|
| obra | `work` | El libro en abstracto: «Dune», sin decir qué edición |
| edición | `edition` | Una edición publicada de una obra |
| ejemplar | `copy` | Lo que hay en la estantería: formato, de quién es, dónde está |
| persona (autor) | `person` | Quien contribuyó a una obra |
| editorial | `publisher` | |
| género | `genre` | Una clave de la lista cerrada de la app para cada tipo; no es una fila |
| etiqueta | `tag` | Palabra libre de una biblioteca, enlazada a obras |
| serie (agrupación de obras) | `series` | No es la de televisión, que es `tv_series` |
| obra en una serie | `series_entry` | |
| «necesita» | `work_relation` con `requires` | Una expansión necesita su juego base |

## Las personas y lo que apuntan

| En español | En el código | Qué es |
|---|---|---|
| miembro de la familia | `member` | Una fila del catálogo que posee ejemplares. **No** es una cuenta, aunque puede estar enlazada a una |
| marca (de una persona sobre una obra) | `mark` | El estado, la valoración y el favorito de una persona sobre una obra |
| estado · valoración · favorito | `status` · `rating` · `favorite` | Campos de una `mark` |
| lo deseo | `wished` | Campo de una `mark` |
| dispositivo | `device` | |
| biblioteca | `library` | En el servidor, la unidad que posee los datos |
| rol: administrador · editor · lector | `admin` · `editor` · `reader` | |
| salir de la biblioteca | `leave_for_good` | Una persona se retira el acceso a sí misma y corta sus propios dispositivos (`member.leave`). No es `leave_library`, que quita la biblioteca de un dispositivo y deja el acceso |
| acceso retirado (de una persona) | `access_withdrawn` | Campo de un `member`: la persona ya no puede llegar a la biblioteca, desde ningún dispositivo ni por su cuenta, pero sigue en el catálogo. No es una `revocation`, que va de un dispositivo |

## Operaciones y sincronización

| En español | En el código | Qué es |
|---|---|---|
| operación · cambio · intención | `operation` · `change` · `intent` | Una **operación** es un lote atómico de **cambios** de entidades, con una **intención** (`book.add`) que solo informa |
| operación anterior (del mismo autor) | `previous` | Campo de una operación: el id de la anterior de su autor en esa biblioteca, o nulo. Forma la cadena de cada autor |
| reloj lógico híbrido | `hlc` | La marca que ordena operaciones de varios dispositivos sin depender de que sus relojes coincidan |
| lápida | `tombstone` | Marca una fila borrada para que no pueda volver |
| estado de una operación: aplicada · petición | `applied` · `request` | Una `request` es una operación que su autor no podía hacer: espera a que quien administra la resuelva |
| solicitud | `change_request` / `ChangeRequest` | La fila que resuelve una petición. No es `request`, que choca con la petición HTTP |
| retirada de acceso | `revocation` | Un dispositivo que ya no puede recibir una biblioteca. Se sincroniza con ella |
| valla (sobre un dispositivo) | `fence` | La pone la operación que le quita poder a un dispositivo: lo que ese dispositivo fechó antes, más allá de la operación `trusted_through`, no se cree |
| sello | `seal` | Una operación aplicada que pone una valla al dispositivo de su propio autor (a todos, se nombren o no) |
| clave avalada | `vouched_key` | Campo de un `device`: la clave pública que quien dijo de quién es el dispositivo había registrado para él |
| fundación (de una biblioteca) | `founding` | Su primera operación: la fila `library`, su primera persona como `admin` y el dispositivo de su autor asignado a ella |

## La nube

| En español | En el código | Qué es |
|---|---|---|
| dispositivo de la nube | `cloud_device_id` | Campo de la fila `library`: el dispositivo que es la nube en esa biblioteca, el único con el poder de la nube |
| cuenta de una persona | `account_id` | Campo de un `member`: un id aleatorio de la cuenta enlazada a esa persona; solo lo escribe la nube |
| usuario (cuenta) | `user` | Una cuenta en el servidor |
| membresía | `membership` | En el servidor, enlaza una cuenta con un `member` de una biblioteca. El rol es siempre el del miembro, nunca se copia aquí. No es `member` |
| clave del servidor (de una fila del catálogo) | `key` | Solo en el servidor: la clave primaria de una fila del catálogo. El `id` de la app es único dentro de su biblioteca y es lo que llevan las operaciones; la `key` no sale del servidor |
| invitación | `invitation` | |

## Por qué hay tantos nombres de seguridad

Vallas, sellos, claves avaladas, fundación: salen de una sola pregunta. Si una operación puede cambiar quién tiene poder en una biblioteca, ¿qué pasa con lo que un dispositivo ya había hecho antes de que se lo quitaran? La respuesta se explicará en la parte sobre sincronización y seguridad de esta documentación. Si solo vas a añadir un campo o arreglar una pantalla, puedes ignorarlos.
