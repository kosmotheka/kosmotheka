# Encargo · Guía de usuario, primera tanda

> Encargo cerrado para la sesión de hitos, escrito el 16/10/2026. Sin commit, sin push, sin cambiar de rama. Es el paso 4 de [plans/repositorio-publico.md](../plans/repositorio-publico.md). Lee antes la parte «Quien usa la app» de ese plan y la tabla 1 de [plans/documentacion-publica.md](../plans/documentacion-publica.md). Solo toca `site/src/content/docs/es/using-the-app/` y las imágenes de esas páginas. Si algo no encaja, para y dímelo.

## Para qué

Las primeras páginas de la guía para quien usa la app, en español. Son las que hacen falta antes de la prueba cerrada de Google Play. El maintainer lo quiere así: **clara, con un lenguaje que entienda cualquiera, sin jerga rara.**

## Las páginas de esta tanda

Todas son de lo que ya funciona sin cuenta, lo que menos va a cambiar:

1. **Qué es Kosmotheka y qué no es.** Explicación. Material: la landing (`web/es/`) y `VISION.md`.
2. **Primeros pasos: instalar y añadir tu primer libro.** Tutorial, de principio a fin. Material: la bienvenida de la app (D91) y `testing/hito-1.md`.
3. **Añadir escaneando, por ISBN o buscando por título.** Guía. Material: `testing/ux-escanear-y-editar.md` y la Ayuda de la app (clave «search» en `app/src/i18n/locales/es.json`).
4. **Ejemplares, ubicaciones y de quién es cada cosa.** Guía. Material: el glosario de `AGENTS.md` y la ficha de la app.
5. **Compartir en casa: emparejar dispositivos por wifi.** Tutorial. Material: la Ayuda («pairing», «share») y `testing/fase-2.md`.
6. **Copias de seguridad, exportar e importar.** Guía. Material: la Ayuda («backups»).
7. **Qué sale de tu dispositivo y qué no.** Explicación. Material: la política de privacidad de la web, D91 y la pantalla de Proveedores.

**Lo que cuenta cada página sale de la app tal como es hoy**, no de los planes. Si un documento de `docs/` dice una cosa y la app hace otra, manda la app: compruébalo en el código o en la interfaz con `?mock`, y apúntame la diferencia.

## Cómo se escribe

- **Una página responde a una pregunta** que alguien se hace con la app en la mano. El título es esa pregunta o su respuesta corta.
- **Palabras de la calle.** Las de la interfaz, tal cual, entre comillas angulares («Compartir y dispositivos»), para que se encuentren en pantalla. Nunca: operación, sincronizar el vector, HLC, valla, sello, firma, clave pública, entidad, *local-first*. Si hace falta la idea, se dice qué pasa: «los cambios llegan a los demás dispositivos cuando están en la misma wifi».
- **Frases cortas**, de tú, en presente. Pasos numerados para hacer algo; listas para enumerar.
- **Lo que no se puede hacer, también**, sin esconderlo. Por ejemplo: «el escáner necesita un móvil con Google Play».
- **Capturas** donde ayuden, no en cada paso: de la app en el navegador con `?mock` (el modo de prueba) y Playwright, en tema claro, a tamaño de móvil (390×844) o de escritorio según la página. Van junto a la página, en WebP.
- **Enlaces entre páginas** de esta tanda donde una lleve a la otra.

## La revisión de jerga

Al terminar cada página, **léela como alguien que no programa** y haz una lista de las palabras o frases que no entendería. Reescríbelas y pon la lista en el informe, con lo que cambiaste. Después la repasará otra sesión solo con esa mirada.

## Reglas

- Solo español en esta tanda. El inglés llega después; el sitio ya enseña la página española con su aviso.
- Ningún dato que no esté en la app o en el repositorio. Nada de promesas de lo que vendrá, salvo una línea si de verdad ayuda («los préstamos llegarán más adelante»).
- No toques la app ni `docs/`.
- No deshagas nada con git. El maintainer puede tener `tauri dev` abierto: no lo cierres.

## Al terminar

- `npm run build` en `site/`, con su salida.
- Para cada página: su título, cuántas palabras tiene y la lista de jerga encontrada y cambiada.
- Las diferencias entre `docs/` y la app que hayas visto.
- Una captura de una de las páginas en el sitio, para ver cómo queda.
