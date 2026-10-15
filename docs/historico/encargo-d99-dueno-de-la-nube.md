# Encargo · La biblioteca de la nube no se va con la cuenta de quien la subió (D99)

> Encargo cerrado para la sesión de hitos, escrito el 15/10/2026. **Se envía cuando termine el de cuentas** ([encargo](encargo-cuentas-d100.md)), porque los dos tocan `DELETE /v1/account`. Sin commit, sin push, sin cambiar de rama. Toca `server/`, una línea de `app/` y `docs/`. Nada en `spec/`.

## Para qué

Cada biblioteca de la nube es «de» la cuenta que la subió (`cloud_library.owner`), y la base de datos la borra en cascada con esa cuenta (`CloudLibrary.php:51`, `onDelete: 'CASCADE'`). Si quien la subió borra su cuenta, la biblioteca desaparece de la nube para todos (SEG-061, [informe](../audits/2026-10-15-2.md)).

Lee [D99](../DECISIONS.md) tal como quedó revisada el 15/10/2026, y D65, D76, D93 y D98. El dueño solo cuenta para el primer enlace (`CloudLink::signedByOwner`) y para ese borrado. El acceso de todos, el del dueño incluido, sale de `Membership`.

## Qué se construye

1. **Al borrar una cuenta** (`AccountController::delete`), antes de borrarla, cada biblioteca de la nube de la que es dueña pasa a otra cuenta con una membresía activa en ella (`withdrawnAt` nulo y su persona sin `access_withdrawn`):
   - primero, una cuya persona del catálogo de esa biblioteca tiene el rol `admin`;
   - si no hay ninguna, cualquier otra;
   - entre varias del mismo grupo, la de membresía más antigua, para que el resultado no dependa del orden de la base;
   - si no queda ninguna, la biblioteca se borra con la cuenta, como hoy.
   Todo en la misma transacción que el borrado de la cuenta, y bajo el cerrojo de cada biblioteca, como las demás escrituras que cambian quién la tiene.
2. **En la base, `owner_id` pasa a `ON DELETE RESTRICT`** (migración), para que ninguna biblioteca se borre en cascada sin pasar por esa regla. El paso 1 borra explícitamente las que se quedan sin nadie.
3. **El registro de actividad** apunta el cambio de dueño con un tipo nuevo de `ActivityKind`, con la biblioteca y las dos cuentas por su id, nunca por su correo (D66).
4. **App:** el aviso de borrar la cuenta añade una línea: las bibliotecas compartidas en la nube siguen para las demás personas; las que solo eran tuyas se borran de la nube. En `en` y `es`.

## Lo primero: el enlace ya guardado

`CloudLink` comprueba la firma del enlace contra los dispositivos **del dueño actual** (`signedByOwner`, también en la línea 136). Mira si esa comprobación se repite después del primer enlace: al reconstruir una biblioteca (`app:catalog:rebuild`), al volver a leer su registro o al aplicar una operación que llega más tarde. Si se repite, con otro dueño fallaría y la biblioteca quedaría sin enlace.

**Antes de escribir nada, dime qué has encontrado y qué propones.** Por ejemplo, que esa comprobación solo valga para el primer enlace, o guardar quién firmó el enlace. No lo decidas tú solo: toca D93.

## Casos hostiles (prueba antes de la defensa, vista fallar)

1. Ana sube una biblioteca, invita a Bea (administradora) y a Carlos (lector) y borra su cuenta: la biblioteca sigue en la nube, su dueña es Bea, y Bea y Carlos siguen bajando y subiendo.
2. Sin administradores con cuenta, pasa a Carlos.
3. Una persona con el acceso retirado (D98), o una membresía retirada, nunca la recibe.
4. Ana sola en su biblioteca: se borra con su cuenta, sin dejar filas.
5. Borrar una fila de usuario directamente en la base, sin pasar por la ruta, falla por `RESTRICT` si es dueña de alguna biblioteca.
6. Tras el cambio de dueño, la reconstrucción de la biblioteca (`app:catalog:rebuild`) da lo mismo que antes.

## Reglas

- Doctrine primero; consultas con parámetros. Sin dependencias nuevas.
- Si algo contradice una decisión aceptada, falta una pieza o el encargo crece, **para y dímelo**.
- No deshagas nada con git.

## Al terminar

Servidor: `make check` **después del último cambio**. App: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`; `cargo fmt --all --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` si tocas Rust. La salida real de cada uno.

Documentación: `docs/architecture/cloud.md`, `docs/architecture/accounts.md` y `docs/plans/fase-7.md` (el apunte «Echan a quien subió la biblioteca…»).

Informe: qué has hecho, qué decidiste tú y por qué, qué pruebas viste fallar antes y qué no has podido comprobar.
