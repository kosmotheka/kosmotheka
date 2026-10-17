# Encargo · Decir antes que la nube es por invitación, y la web al día

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. El maintainer lo pidió el mismo día: hoy, que la nube oficial es por invitación solo se descubre al intentar crear la cuenta. Es un ajuste de textos y de un aviso, **no una funcionalidad nueva**. Toca `app/`, `web/` y `docs/publicacion/`.

## 1. El aviso en la app

`GET /v1/server` ya dice `registration_restricted`, y la app solo lo usa al escribir el correo. **En la primera pantalla de «Iniciar sesión o crear cuenta», antes de escribir nada**, si el servidor está restringido, una línea a la vista:

> «Por ahora, la nube es por invitación. Si no tienes una, puedes usar la app sin cuenta: todo funciona igual en tus dispositivos.»

Con un enlace a «Tengo una invitación», si esa acción existe en la pantalla. Textos en `es` y `en`. En `?mock`, con el servidor restringido y sin restringir. Sin tocar el servidor.

## 2. La web: las preguntas frecuentes y el inicio, al día

`web/es/preguntas.html` y `web/en/faq.html` son de antes de la nube. Dicen:
- que la cuenta llegará «más adelante» y que la nube es «próximamente»;
- «No tenemos servidores»;
- «Dar acceso a otro dispositivo» para emparejar (en la app es «Compartir y dispositivos» › «Emparejar otro dispositivo»);
- los estados de lectura antiguos.

Ponlas al día con la app de hoy, en los dos idiomas, con la misma plantilla y el mismo tono:
- **La cuenta y la nube existen, son opcionales y, por ahora, por invitación.**
- **Dónde están tus datos:** sin cuenta, solo en tus dispositivos. Con la nube, también en el servidor, y quien lo administra puede leerlo (enlace a la política nueva).
- **Compartir con alguien de otra ciudad:** con la nube, invitando con un enlace.
- **Perder el móvil:** con cuenta, el móvil nuevo recibe sus bibliotecas al entrar.
- Lo que prometa algo que aún no existe (préstamos, por ejemplo) se quita o se dice como futuro.

Mira también `web/es/index.html` y `web/en/index.html`: corrige solo lo que hoy sea falso, sin rediseñar.

**Lo que cuenta sale de la app tal como es hoy**, como en la guía. No se sube nada: lo sube el maintainer.

## 3. La ficha de Google Play

En `docs/publicacion/ficha-google-play.md`:
- **descripción corta:** hoy dice «Sin cuenta ni nube». Que no mienta: la cuenta y la nube son opcionales (80 caracteres como mucho);
- **descripción larga:** una sección corta «LA NUBE, SI LA QUIERES», con la cuenta opcional, todos tus dispositivos sin compartir wifi, invitar a alguien de otra ciudad, y «por ahora, por invitación»;
- y lo que haya quedado viejo: BoardGameGeek, los estados de lectura.

Respeta los límites de Play (80 y 4.000 caracteres) y di cuántos ocupa cada texto.

## Al terminar

En `app/`, `npm run build`, `typecheck`, `lint`, `format:check` y `knip`. El informe: el aviso (con una captura de `?mock`), las preguntas cambiadas, la ficha nueva con sus cifras y lo que hayas visto falso en la web y no hayas tocado.
