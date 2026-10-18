# Encargo · Salir de una biblioteca que está en la nube, y volver a bajarla

> Encargo cerrado para la sesión de hitos, escrito el 18/10/2026. Sin commit, sin push. Aprobado por el maintainer el mismo día. Sale de tu informe del encargo de ajustes (punto 1). **Toca quién recibe una biblioteca:** regla de AGENTS.md, **el test del caso hostil va primero y se ve fallar**.

## El problema

1. **No se puede salir** de una biblioteca que solo está en este dispositivo y en la nube: `libraries::leave_library` solo cuenta los pares de la red local (`lan::devices_sharing`) y devuelve `OnlyOnThisDevice`.
2. **Una vez fuera, no vuelve nunca por la nube:** `forget_here` la apunta en `forgotten_libraries` y nada la borra de ahí. La cuenta sigue teniendo acceso, pero no hay forma de traerla otra vez a ese dispositivo.

## Lo que no puede romperse: SEG-017

`forgotten_libraries` existe por SEG-017 (`docs/audits/2026-10-03-3.md`): **un dispositivo al que se le retiró el acceso** (`remove_revoked`, D71) no puede volver a bajarla de la nube con la sesión de la cuenta. Hoy, dejar y que te la retiren usan la misma tabla y no se distinguen. **Antes de tocar nada, léelo**, junto con D71, D98 (acceso retirado de una persona) y lo que hace hoy el servidor con un dispositivo retirado: si la nube ya se lo niega por su cuenta o si solo lo para la app. **Cuéntamelo en el informe.**

## Qué

1. **Salir con la nube:** si la biblioteca está subida a la nube de la cuenta con la que este dispositivo tiene la sesión abierta (mira cómo sabe hoy la app que una biblioteca está en la nube; `useCloudRows` en la UI, y lo que lo respalde en Rust), cuenta como «sigue en otro sitio», y se puede salir.
   - El diálogo (`remove-library-dialogs.tsx`) dice que sigue en la nube, con una frase sin jerga, como «Seguirá en tu cuenta de la nube».
2. **Distinguir por qué se olvidó:** dejarla por decisión propia y que te la retiren no son lo mismo. Una migración que guarde el motivo en `forgotten_libraries` (por ejemplo `reason TEXT NOT NULL`, `left` o `revoked`). Las filas que ya existen quedan como `revoked`, que es lo seguro.
   - Es tabla local, así que no hay `schemaVersion`, `spec/` ni servidor. Si me equivoco y sí los toca, para y dímelo.
3. **Volver a bajarla:** solo una biblioteca olvidada con `left` vuelve, y solo cuando la persona lo pide.
   - Lo automático (`sync_all`) sigue sin bajarla: `DeletedHere`.
   - **La forma de pedirlo:** mira qué ofrece hoy la pantalla de la cuenta o de la nube con las bibliotecas que tiene la cuenta («Bajar de la nube» o como se llame), y ponle la olvidada con «Volver a bajar». Si no hay un sitio natural, dímelo antes de inventarlo.
   - Al bajarla se borra su fila de `forgotten_libraries` y se baja como cualquier biblioteca de la nube.
   - Una con `revoked` no ofrece nada.

## Tests, en este orden

En Rust, junto a los de SEG-017 (busca `DeletedHere` y `library_forgotten`).
1. **Hostil, primero y fallando:** un dispositivo al que se le retiró la biblioteca no la recupera de la nube, ni con `sync_all` ni pidiendo «volver a bajar». Sigue igual con una fila antigua sin motivo, después de la migración.
2. Se puede salir de una biblioteca que solo está aquí y en la nube; la que solo está aquí sigue dando `OnlyOnThisDevice`.
3. Después de salir, `sync_all` no la baja (sigue `DeletedHere`).
4. Al pedir «volver a bajar», vuelve, con sus datos, y la fila de `forgotten_libraries` desaparece.
5. Si hay un test de convergencia que toque esto, que siga pasando (`assert_converged`).

## Novedades y guía

- Una línea bajo `"0.3.0"` en `app/src/whats-new.json`: se puede salir de una biblioteca que está en tu nube y volver a bajarla después. Sin `switch`, porque no cambia lo que sale del dispositivo; si crees que sí, dilo.
- La guía: `several-libraries.md` (lo de salir) y `the-cloud.md` (bajar de nuevo), en `es/` y `en/`.
- `docs/architecture/cloud.md` y `sync.md`, donde hablen de `deleted_here` y de `forgotten_libraries`.

## Al terminar

- En `app/`: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:i18n` y `check:whats-new`.
- `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings` y `cargo test`.
- En `site/`: `npm run build && npm run check:links`.

El informe:
- qué hace hoy el servidor con un dispositivo retirado;
- la migración;
- los tests, y que el hostil falló antes;
- dónde está «Volver a bajar» en la pantalla.
