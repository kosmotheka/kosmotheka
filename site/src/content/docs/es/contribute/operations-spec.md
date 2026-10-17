---
title: "Las operaciones: la especificación y los casos de conformidad"
description: El formato de una operación, las reglas que comprueba quien la recibe y los casos de prueba que comparten Rust y PHP, para que cualquiera pueda escribir un cliente o un servidor compatible.
sidebar:
  order: 7
---

Todo cambio en el catálogo es una **operación**. La carpeta [`spec/`](https://github.com/kosmotheka/kosmotheka/tree/main/spec) define cómo es una operación y qué reglas cumple, en un formato que no pertenece a ningún lenguaje. Esta página explica qué hay en ella, cómo la usan la app (Rust) y el servidor (PHP) y cómo añadir un caso de prueba. Para entender por qué existen las operaciones, empieza por [El motor de sincronización](../sync-engine/).

**`spec/` tiene licencia MIT**, distinta de la AGPL del resto del proyecto. *Por qué:* para que cualquiera pueda escribir un cliente o un servidor compatible sin heredar la AGPL. Nada de lo que hay en esa carpeta depende de nuestro código.

## Qué hay en `spec/`

| Archivo | Qué contiene |
|---|---|
| `operation.schema.json` | El JSON Schema de una operación: el sobre, los cambios y los campos que acepta cada entidad |
| `sync.json` | Lo que todos tienen que compartir para sincronizar: la versión del esquema (`schemaVersion`). Dispositivos y servidores con otra versión no sincronizan |
| `README.md` | Las reglas que un JSON Schema no puede expresar, con sus códigos de error |
| `conformance/operation-validation.json` | Operaciones con el resultado de validarlas: `valid`, o `invalid` con un código de error, el índice del cambio y el campo |
| `conformance/signatures.json` | Operaciones firmadas, con los bytes canónicos exactos y si la firma vale |
| `conformance/merge.json` | Cómo se fusionan operaciones dentro de una biblioteca (campo a campo, el borrado gana, entregas repetidas…) |
| `conformance/library-merge.json` | Lo mismo con varias bibliotecas |
| `conformance/permissions.json` | Operaciones de dispositivos con distintos roles, entregadas en órdenes distintos: cuáles se aplican y cuáles quedan como petición |

## El formato de una operación

Una operación es un objeto JSON con estos campos. El JSON Schema exige todos menos `signature`, pero **una operación sin firma la rechaza siempre quien la recibe**:

| Campo | Qué es |
|---|---|
| `id` | Un UUID v7 |
| `library` | La biblioteca a la que pertenecen **todos** sus cambios. Un cambio no puede nombrar otra |
| `hlc` | La marca del [reloj lógico híbrido](../sync-engine/#2-el-reloj-lógico-híbrido-poner-orden-sin-un-reloj-común): 16 dígitos, 8 caracteres hexadecimales y el id del dispositivo (que es un UUID en minúsculas), separados por guiones |
| `intent` | La etiqueta de intención: dos o más palabras en minúscula separadas por puntos (`book.add`). **Solo informa**: los permisos y la fusión miran los cambios, nunca la etiqueta |
| `previous` | El id de la operación anterior del mismo dispositivo en esa biblioteca, o `null` si es la primera. **La clave siempre está**, aunque sea nula. Va firmada |
| `changes` | Una lista de uno o más cambios |
| `signature` | La firma Ed25519 del dispositivo que la hizo (el del `hlc`) |

Cada cambio es `{ entity, id, action, fields }`: la entidad que toca, el id de su fila, la acción (`create`, `update` o `delete`) y los campos. Un `delete` no lleva campos; un `update` lleva al menos uno.

Un ejemplo, simplificado: añadir un libro crea cuatro filas en una sola operación.

```json
{
  "id": "0199a1b2-…",
  "library": "0199a0aa-…",
  "hlc": "0001727222222222-00000000-0199a0ff-…",
  "intent": "book.add",
  "previous": "0199a1a0-…",
  "changes": [
    { "entity": "work", "id": "0199…11", "action": "create",
      "fields": { "kind": "book", "title": "El nombre del viento" } },
    { "entity": "edition", "id": "0199…12", "action": "create",
      "fields": { "work_id": "0199…11", "title": "El nombre del viento" } },
    { "entity": "book_details", "id": "0199…12", "action": "create",
      "fields": { "isbn13": "9788401352188" } },
    { "entity": "copy", "id": "0199…13", "action": "create",
      "fields": { "edition_id": "0199…12", "format": "hardcover", "quantity": 1 } }
  ],
  "signature": "…"
}
```

### Entidades

Las filas que una operación puede tocar (`entity`) son: `library`, `work`, `person`, `work_contributor`, `edition`, `book_details`, `comic_details`, `boardgame_details`, `film_details`, `tv_series_details`, `member`, `copy`, `publisher`, `tag`, `work_tag`, `work_genre`, `series`, `series_entry`, `work_relation`, `revocation`, `device`, `change_request` y `mark`. Qué campos acepta cada una está en `operation.schema.json`, y debe coincidir con `crates/domain/src/schema.rs` y con `server/src/Enum/CatalogEntity.php`. Los nombres están explicados en el [glosario](../glossary/).

### Intenciones

La etiqueta dice qué quería hacer la persona, para el historial y para mostrarlo. Son cosas como `book.add`, `copy.add`, `copy.edit`, `copy.remove`, `book.remove`, `edition.edit`, `series.add`, `mark.set`, `library.found`, `library.revoke`, `device.assign`, `member.withdraw` o `request.accept`. **No hay una lista cerrada**: la regla del formato es solo «palabras en minúscula separadas por puntos». Que no cuente para los permisos es deliberado: así nadie puede colar un cambio prohibido bajo una etiqueta inocente.

## Las reglas que comprueba quien recibe

Quien recibe una operación (la app o el servidor) hace dos comprobaciones, en este orden.

**1. Es una operación bien formada (validación).** Se comprueba el sobre y cada cambio. Estas son las reglas más importantes:

- el `id` es un UUID; la `library` y el `previous` tienen forma válida y `previous` nunca es el id de la propia operación;
- la intención tiene el formato correcto y hay al menos un cambio;
- un cambio no puede nombrar otra biblioteca (un `library_id` en un cambio es un campo desconocido), ni crear la fila `device` de otro dispositivo;
- los campos existen para esa entidad y tienen el tipo correcto; los obligatorios están al crear y **nunca son nulos**;
- las fechas existen en el calendario (`2026-02-30` no vale); un ISBN son trece dígitos; la valoración va de 1 a 5; la cantidad de un ejemplar es 1 o más;
- ningún texto lleva el carácter nulo (PostgreSQL no puede guardarlo).

Solo se informa del **primer error**, en un orden fijo (id, biblioteca, `previous`, intención, cambios vacíos y después cada cambio), con un código estable: `invalid_id`, `invalid_library`, `invalid_previous`, `invalid_intent`, `no_changes`, `other_library`, `other_device`, `unknown_field`, `missing_field`, `null_required`, `wrong_type`, `empty_update`, `fields_on_delete`, `wrong_id` y `undeletable`. *Por qué solo uno y en orden:* para que Rust y PHP den **exactamente el mismo resultado**, caso a caso.

**2. Está permitida y es de fiar (firma, permisos, cadena).**

- La **firma** vale. Se calcula sobre la operación sin el campo `signature`, escrita en JSON canónico ([RFC 8785](https://www.rfc-editor.org/rfc/rfc8785): claves ordenadas, sin espacios), con la clave pública de su autor, que viaja con la biblioteca en una fila `device`. La primera clave registrada para un dispositivo es la que vale.
- Los **permisos**: cada cambio se juzga con el rol del autor en ese momento. Un administrador puede todo; un editor crea y edita, y solo borra lo que creó su persona; un lector no cambia nada del catálogo. Cada persona apunta lo suyo (sus marcas) sea cual sea su rol. Nadie da más rol del que tiene. Si algún cambio no está permitido, **toda la operación queda como petición**. Las reglas completas están en `spec/README.md`, sección «Roles and permissions».
- La **cadena**: si la operación anterior del autor no se conoce, la operación espera. Una operación de más de una hora en el futuro se rechaza. Dos operaciones con la misma marca: la segunda se rechaza.

Más detalle de las reglas de fusión, en la sección «Merging operations within a library» de `spec/README.md`.

## Los casos de conformidad

Un **caso de conformidad** es una operación (o varias) de ejemplo, con el resultado que **toda** implementación tiene que dar. Son la forma de comprobar que la app y el servidor no se han separado, y lo que permite a un tercero comprobar que su cliente o su servidor es compatible. Hay cinco archivos, y tienen formas distintas.

- **`operation-validation.json`:** una lista de `{ name, operation, expect }`, donde `expect` es `{ "result": "valid" }` o `{ "result": "invalid", "code": …, … }`.
- **`signatures.json`:** la operación, la clave pública, el texto canónico exacto y el resultado (`valid`, o `invalid` con `missing`, `bad_key`, `malformed` o `mismatch`). Las claves de estos casos salen de bytes de prueba fijos y no deben usarse para nada más.
- **`merge.json`, `library-merge.json` y `permissions.json`:** una lista de `operations`, una lista de `deliveries` (órdenes en los que se entregan, como listas de índices) y el resultado esperado. **Cada orden de entrega tiene que acabar igual.** `merge` y `library-merge` dicen qué filas deben existir al final; `permissions` dice qué operaciones quedan aplicadas, cuáles como petición y, opcionalmente, cuáles se rechazan.

### Cómo los ejecutan Rust y PHP

| Casos | Rust | PHP |
|---|---|---|
| Validación | `crates/domain/src/operation.rs` | `server/tests/Validator/ValidOperationValidatorTest.php` |
| Firmas | `crates/domain/src/signature.rs` | `server/tests/Sync/OperationSignatureTest.php` |
| Fusión | `app/src-tauri/src/sync.rs` (`merge_conformance_cases`) | `server/tests/Sync/Merge/MergeConformanceTest.php` |
| Permisos | `app/src-tauri/src/sync.rs` (`permission_conformance_cases`) | `server/tests/Sync/Permission/PermissionConformanceTest.php` |

Cada implementación **carga los archivos de `conformance/`** (no los de `conformance/draft/`, que son para la próxima versión del formato) y comprueba que cada caso da lo esperado. Si una diverge, falla su prueba. Además, `npm run check:spec` (en `app/`) comprueba que el JSON Schema está de acuerdo con los casos de validación.

## Añadir un caso

Cuando cambies una regla, o encuentres un caso que nadie había probado, el caso va **primero**, antes que el cambio en el código (y se ve fallar, si es un arreglo).

1. **Elige el archivo.** Una regla de forma o de tipos va en `operation-validation.json`; una de fusión, en `merge.json` (o `library-merge.json` si intervienen varias bibliotecas); una de roles, en `permissions.json`; una de firmas, en `signatures.json`.
2. **Escribe el caso** siguiendo los que ya hay en ese archivo, con un `name` que diga lo que prueba. Usa ids de prueba reconocibles, como los que ya hay (`01997c7e-0000-7000-8000-…`).
3. **Escribe el resultado esperado** razonando desde la regla, no desde lo que hoy hace el código. Si es un caso de fusión o de permisos, escribe varios órdenes de entrega: es lo que demuestra que converge.
4. **Ejecuta las pruebas de los dos lados**:
   - `npm run check:spec` y `cargo test` (la app);
   - `make test` en `server/` (PHP).
5. **Si cambió el formato**, actualiza también `operation.schema.json`, `schema.rs` y `CatalogEntity.php`, la versión del esquema en `sync.json` (`schemaVersion`) y las reglas de `spec/README.md`. Está explicado paso a paso en [Añadir un campo o una entidad](../how-to-contribute/#añadir-un-campo-o-una-entidad-paso-a-paso).

Una regla que cambia **va primero a `spec/`**, con sus casos, y después a las dos implementaciones.

## Escribir un cliente o un servidor compatible

Si quieres hacer otra implementación, esto es lo que necesitas:

1. Aceptar y producir operaciones con el formato de `operation.schema.json` y las reglas de `spec/README.md`.
2. Firmar y comprobar firmas con Ed25519 sobre el JSON canónico.
3. Calcular marcas de reloj lógico híbrido y comparar con el orden de texto.
4. Aplicar los cambios con las reglas de fusión (campo a campo, el borrado gana, las lápidas).
5. Juzgar los permisos con las reglas de roles.
6. Pasar los casos de `conformance/`.

Los casos están pensados para que puedas empezar por ellos: el conjunto completo de reglas es lo que hacen cumplir.
