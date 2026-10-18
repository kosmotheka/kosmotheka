# Encargo · Tres ajustes de la app para la prueba cerrada

> Encargo cerrado para la sesión de hitos, escrito el 18/10/2026. Sin commit, sin push. Aprobado por el maintainer el mismo día. Salen de las diferencias apuntadas al escribir la guía (`docs/plans/repositorio-publico.md`, «Guía de usuario, tercera tanda»). **Mover una obra a otra biblioteca no se toca:** que no lleve las marcas ni la serie es lo esperado (decidido por el maintainer).

**La versión ya tiene número: `0.3.0`.** Las líneas de novedades van bajo la clave `"0.3.0"` de `app/src/whats-new.json`, no bajo `next`.

## 1. «Borrar biblioteca», solo para quien administra

Hoy se ve con cualquier rol. La regla (D73) es que solo administra quien la borra para todos (`libraries::delete_library` es una operación que viaja, D65).

- Lee cómo decide la app hoy qué ve cada rol (busca cómo se esconden otras acciones de administración en la UI, por ejemplo en Personas) y usa lo mismo; no inventes otra forma.
- Los dos sitios que abren el diálogo: `app/src/pages/library-page.tsx` (línea ~289) y `app/src/components/libraries/remove-library-dialogs.tsx` (línea ~251, «Ir a borrar» cuando la biblioteca solo está en este dispositivo).
- **Quien no administra** ve «Salir de esta biblioteca» (`libraries::leave_library`: la quita de este dispositivo, los demás la conservan). En el diálogo de salir, el caso «solo está en este dispositivo» hoy manda a borrar: para quien no administra, ¿qué tiene que ver? **Dímelo antes de decidirlo:** cuéntame qué pasa hoy, con y sin cuenta en la nube.
- **¿Y en Rust?** Mira si `delete_library` comprueba el rol al grabar o si la operación de quien no administra acaba como petición (D73) al recibirla. Si en local se graba sin comprobar, dilo: **no lo cambies sin hablarlo**, que toca operaciones y sincronización.
- **Una pregunta del maintainer:** al salir, ¿la biblioteca desaparece también de su cuenta en la nube, o vuelve a bajar? Mira `forgotten_libraries` y lo que hace la app al sincronizar con la nube, y explícalo en el informe. **No cambies nada de esto.**

## 2. Las series cuentan lo mismo en la lista y dentro

La lista de series dice «tienes 3 de 5» y la serie, por dentro, «3 de 6», porque dentro se cuentan los huecos.

- Busca las dos cuentas (Rust o la UI, donde estén) y di cuál es la buena. Lo razonable: el total incluye los huecos, en los dos sitios. Si hay un motivo para lo contrario, dilo.
- La cuenta va en Rust si hoy se calcula en la UI (la lógica del dominio va en Rust, AGENTS.md), con un test.
- Una línea en novedades si se nota.

## 3. El botón «Escanear» del escritorio

En el escritorio, el botón «Escanear» lleva a «¿Lo tengo?» en vez de escanear.

- Busca qué abre y qué debería abrir. El ordenador puede no tener cámara: mira cómo lo resuelve el móvil y qué hace «Añadir» en el escritorio.
- Si escanear en el escritorio no tiene sentido (sin cámara), dilo antes y propón: quitar el botón, cambiarle el nombre o llevarlo a otro sitio.
- Una línea en novedades.

## Al terminar

- En `app/`: `npm run build`, `typecheck`, `lint`, `format:check`, `knip`, `check:i18n` y `check:whats-new`.
- `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings` y `cargo test`.
- Si la guía describe algo que cambia (`site/src/content/docs/*/using-the-app/series.md`, `several-libraries.md`, `people-and-permissions.md`), corrígela en los dos idiomas, y después `npm run build && npm run check:links` en `site/`.

El informe:
- Cada punto, con lo que cambiaste.
- Las dos respuestas pendientes del punto 1: qué ve en el diálogo de salir quien no administra, y qué pasa con la nube al salir.
