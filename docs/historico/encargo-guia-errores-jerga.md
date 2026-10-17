# Encargo · Aplicar la revisión de jerga a «Mensajes de error» y «Novedades»

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. Sigue a [revision-jerga-4.md](revision-jerga-4.md). Solo `site/src/content/docs/es/using-the-app/error-messages.md` y `whats-new.md`. Mismo criterio que [encargo-guia-usuario-3-jerga.md](encargo-guia-usuario-3-jerga.md).

- **Los textos entre comillas que copian la app no cambian.**
- **«Aparato» sale dos veces** (líneas 89 y 92): pasa a «dispositivo», como en todas partes.
- «Servidor», «nube», «emparejar» y «segundo factor»: no los expliques de nuevo en cada fila. Enlaza la página de la guía que ya los explica, la primera vez que salgan: [La nube](../the-cloud/), [Compartir en casa](../share-at-home/), y lo que toque.
- `error-messages.md` es muy larga (unas 2.900 palabras). Si al aplicar ves filas que repiten la misma explicación, júntalas.
- Aplica las filas con criterio y di en una línea por qué no aplicas una.

## Al terminar

`npm run build && npm run check:links`. El informe: filas aplicadas y no aplicadas, y las palabras finales de cada página.
