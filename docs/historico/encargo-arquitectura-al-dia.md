# Encargo · Los documentos de arquitectura, al día con el código

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. Sale de lo que viste al escribir «Colaborar» (apuntado en [repositorio-publico.md](../plans/repositorio-publico.md)). **Solo `docs/architecture/operations.md`, `security.md` y `sync.md`.** Son documentos de trabajo, en español, y se publicarán con el repositorio: tienen que decir lo que hace el código hoy.

## Qué

1. **`operations.md`:**
   - **La tabla de intenciones**, completa y sacada del código: busca cada `intent` que se escribe en `app/src-tauri/src` y lo que acepta el servidor. Una fila por intención, con qué cambia.
   - Las que ya no se escriben y solo quedan en migraciones o tests (`wish.add`, `wish.remove`): en un apartado «Ya no se usan», con la que las sustituyó.
   - **El apartado «Pendiente»:** lo que ya está hecho (la fusión campo a campo, los permisos) sale de ahí; lo que de verdad falte, se queda.
2. **`security.md`:**
   - Quita «Borrador» y las fechas de trabajo. Pon una línea arriba: «Puesto al día el 17/10/2026».
   - Lo que dice «en construcción» y ya existe (la clave avalada, las invitaciones en la app) pasa a decirse como hecho.
   - Fuera los préstamos, los números de hito y el dominio del servidor oficial.
   - **«Lo que no protege» se queda entero, con la debilidad de emparejar a través de alguien que engaña:** si sigue pendiente en el código, se dice así. El maintainer decide aparte si va también en la guía pública.
3. **`sync.md`:**
   - «Dónde está en el código»: el servidor ya pasa los casos de fusión y de permisos (`MergeConformanceTest`, `PermissionConformanceTest`); que lo diga en presente.
   - Mira el resto con el mismo criterio.

## Cómo

- **No reescribas lo que está bien:** cambia lo que no coincide con el código, y nada más. Estos documentos guardan el porqué; no lo quites.
- **Las decisiones (`DECISIONS.md`) no se tocan.** Si algo de arquitectura contradice una decisión aceptada, apúntalo en el informe.
- Ningún secreto ni nada del servidor oficial.

## Al terminar

El informe:
- qué cambiaste en cada documento;
- las intenciones de la tabla nueva;
- lo que no encaje con una decisión.
