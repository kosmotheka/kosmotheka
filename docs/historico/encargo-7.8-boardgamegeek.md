# Encargo · 7.8, BoardGameGeek a través del servidor

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push, sin cambiar de rama. Sigue el camino de TMDB y RAWG ([encargo-7.8-tmdb.md](encargo-7.8-tmdb.md), [encargo-7.8-rawg.md](encargo-7.8-rawg.md)): léelos antes, y la parte de 7.8 en `docs/plans/fase-7.md`. **Empieza por la fase 0 y espera mi respuesta antes de escribir código.**

## Para qué

Datos de juegos de mesa (y de rol, con RPGGeek, del mismo dueño y la misma API) a través del servidor, que pone el token (D62). El maintainer recibió la aprobación de BoardGameGeek el 17/10/2026.

## Lo que ya se sabe

- **Desde julio de 2025, la XML API pide una aplicación registrada.** Cada petición lleva `Authorization: Bearer <token>`; sin él, BGG responde 401.
- **El token va en `BGG_API_TOKEN`**, en `.env.local` del servidor, como `TMDB_API_TOKEN`. Lo pone el maintainer: **nunca lo leas, lo imprimas ni lo copies.** En `.env` va vacía y, vacía, la ruta responde que el servicio no está disponible, como las demás.
- **Condiciones conocidas:**
  - hay que nombrar a BoardGameGeek como fuente de los datos;
  - el logo «Powered by BGG», enlazado a BGG, debe salir en los usos públicos;
  - **está prohibido usar sus datos para entrenar una IA**;
  - hay registro comercial y no comercial.

  La página de condiciones (`boardgamegeek.com/wiki/page/XML_API_Terms_of_Use`) devuelve 403 sin sesión. **El maintainer la lee y confirma** lo que cuenta para la caché y los límites.

## Fase 0 · Propuesta (sin código)

1. **Qué rutas de la XML API2** usarías: buscar por nombre (`/xmlapi2/search`, tipos `boardgame` y `boardgameexpansion`; y en RPGGeek, `rpgitem`) y el detalle (`/xmlapi2/thing`, varios ids a la vez). Qué campos pasan a la ficha de la app: título, año, autoría y su papel, editorial, jugadores, duración, edad, portada y si es una expansión de otro juego (D61, relación «necesita»). Compáralo con lo que la app ya guarda para un juego de mesa (`schema.rs`).
2. **XML, no JSON:** cómo lo lees en PHP sin dependencias nuevas (`SimpleXML` o `DOMDocument`, con las entidades externas desactivadas) y los cuidados frente a un XML hostil.
3. **La caché y los límites:** igual que TMDB (sin quién preguntó, la búsqueda guardada como huella) y un ritmo prudente hacia BGG. BGG responde a veces 202 «en cola» o 429: cómo lo tratas, sin bloquear el `push` ni a otras cuentas.
4. **La atribución en la app:** la línea de créditos y el logo «Powered by BGG» enlazado, en «Créditos» y en la ficha de un juego que venga de BGG, como pide su condición. Dónde está el logo oficial y con qué licencia; si no se puede bajar sin sesión, dímelo.
5. **El orden con los demás proveedores** de juegos de mesa en Proveedores, y si «Todas» lo incluye.
6. **Novedades:** el límite de cinco líneas en `next` ya está lleno. Propón cuál fundir.
7. **GameUPC no entra en este encargo:** no hay clave de producción. Dime solo si algo de lo que hagas aquí lo complica después.

## Reglas

- Las de los otros proveedores: el token nunca en un log, en una excepción, en una respuesta ni en un test (test con token centinela, como RAWG); los tests no usan la red (respuestas XML grabadas a mano según la documentación, y dilo); nada de la búsqueda en una dirección del servidor.
- Ningún texto de la interfaz escrito a mano en el código: i18n en `en` y `es`.
- Servidor solo con `make`; `make check` una sola vez al final, en segundo plano.
- No deshagas nada con git. El maintainer puede tener `tauri dev` abierto: no lo cierres.

## Al terminar (después de mi visto bueno)

Las comprobaciones de la app y del servidor con su salida real. `docs/plans/fase-7.md` (7.8), `docs/development/servicios-externos.md` y `docs/development/server.md` (la variable). Una prueba real con el token la hace el maintainer: déjale escritos los pasos.
