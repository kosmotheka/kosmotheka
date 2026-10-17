# Encargo · Traducir dos páginas de «Colaborar» y revisar la jerga de dos de la guía

> Encargo cerrado para `library-haiku`, escrito el 17/10/2026. Sin commit, sin push. Dos partes, en este orden.

## Parte A · Traducir al inglés

Con las mismas reglas que [encargo-traduccion-colaborar-2.md](encargo-traduccion-colaborar-2.md): órdenes, rutas, variables y nombres de código iguales; ejemplos inventados traducidos; enlaces entre páginas con `../página/`.

| Español (origen) | Inglés (crear) |
|---|---|
| `site/src/content/docs/es/contribute/add-a-data-source.md` | `site/src/content/docs/en/contribute/add-a-data-source.md` |
| `site/src/content/docs/es/contribute/add-a-language.md` | `site/src/content/docs/en/contribute/add-a-language.md` |

Y en `en/contribute/index.md`, añádelas en el mismo sitio que en español.

## Parte B · Revisar la jerga, sin cambiar nada

Lee `site/src/content/docs/es/using-the-app/error-messages.md` y `whats-new.md` como en [encargo-revision-jerga-3.md](encargo-revision-jerga-3.md): una persona de 60 años que no sabe de informática. Apunta lo que no entendería.

**Los textos entre comillas que copian la app no se proponen cambiar** (son los de la pantalla): solo la explicación de alrededor.

El informe va en `docs/historico/revision-jerga-4.md`, con la misma tabla. Al final, una línea por página.

## Al terminar

`npm run build && npm run check:links` en `site/`, y un mensaje corto con:
- las palabras de las dos traducciones;
- los veredictos de la parte B.
