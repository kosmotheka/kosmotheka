# Plan de la Fase 8 — Publicación

> Plan de trabajo mientras dure la fase. Se basa en D03, D22, D63, D64, D72, D74, D75 y D90 ([DECISIONS.md](../DECISIONS.md)) y en la Fase 8 del [ROADMAP](../ROADMAP.md). Escrito el 16/10/2026, con la Fase 7 a falta de lo que depende del maintainer. Al cerrar la fase pasa a `historico/`.

**Hecho cuando:** cualquiera puede instalarla desde la tienda.

## De dónde se parte

- **Google Play:** prueba interna subida a mano, con la clave de subida guardada fuera del repositorio ([Publicar una versión](../../site/src/content/docs/es/contribute/release.md)). La ficha y el formulario de seguridad de los datos ([ficha-google-play.md](../publicacion/ficha-google-play.md)) son del 29/09/2026: **no cuentan la cuenta ni el servidor**.
- **Web:** landing en kosmotheka.com con la página de descargas en «Próximamente»; las dos políticas de privacidad, sin servidor y con él.
- **App:** bienvenida, «Novedades», «Licencias» y una Ayuda corta. Versión 0.2.0.
- **CI:** compila Linux, Windows y Android en cada push; el instalador de Windows queda como artefacto de 14 días. No publica nada.
- **Repositorio:** privado, con 60 commits sin subir a 16/10/2026.

## Lo que la Fase 7 tiene que dejar antes

La fase solo empieza de verdad cuando el servidor esté en producción (7.9). Sin él, la prueba cerrada probaría una app sin cuenta, y al llegar la cuenta habría que volver a pasar la revisión de Google con otra ficha. Lo que queda de la Fase 7 es del maintainer: el push, el despliegue, las pruebas en pantalla, la sesión de seguridad de 7.9 y la guía de autoalojamiento (hablando antes de D48).

Lo que **no** necesita producción puede ir adelantándose: 8.2, la investigación de 8.3 y la parte de 8.4 que no habla del servidor.

## Hitos

| Hito | Qué | Depende de |
|---|---|---|
| **8.1 Lo que pide Google Play** | Ficha y formulario de seguridad de los datos al día con la cuenta, el correo, el servidor y los proveedores a través de él. **Una página web para pedir que se borre la cuenta**: Google la exige a toda app que permite crear cuentas, aparte del botón de la app. La política de privacidad con servidor, publicada. Capturas nuevas. Después, la **prueba cerrada**: 12 personas durante 14 días | Servidor en producción. La consola de Play y las 12 personas son del maintainer |
| **8.2 Publicar versiones (D74)** | Un workflow que, al subir una etiqueta, compila Linux en Ubuntu 22.04 y Windows, crea un *release* en borrador con nombres sin versión y sumas de comprobación, y sube el `.aab` firmado a la pista de prueba de Play. Una lista de pasos para publicar: subir las tres versiones, renombrar `next` en `whats-new.json`, etiqueta | Las claves en los secretos de GitHub: sesión juntos, como la clave de subida. La descarga pública del escritorio necesita el repositorio abierto (8.5) |
| **8.3 Notificaciones con la app cerrada (D75)** | Primero, una nota de investigación: qué plugin de push de la comunidad sirve en Tauri 2, qué pide Firebase y qué ve Google. Después, el servidor (un aviso genérico tras una petición nueva, una petición resuelta y una invitación aceptada), la app (permiso de Android 13, registrar el dispositivo, Ajustes de avisos) y la línea en la política de privacidad | Un proyecto de Firebase, que crea el maintainer. Servidor en producción para probarlo de verdad |
| **8.4 Ayuda y documentación pública (D90)** | La web docs.kosmotheka.com con Starlight, en español e inglés, empezando por las páginas «A» del [índice](documentacion-publica.md) (las que hacen falta antes de la prueba cerrada). La Ayuda de la app enlaza con ellas | Nada para empezar; el despliegue automático, de que se decida cómo (D90) |
| **8.5 Abrir el repositorio (D64, D90, D72)** | Cerrar D64. Escanear secretos de todo el historial, ensayar `git filter-repo` en una copia, mover la documentación interna a su carpeta ignorada, quitar los hallazgos concretos del método de auditoría, LICENSE y README con la exclusión de la marca, TRADEMARKS.md, aviso privado de vulnerabilidades en GitHub. Subirlo a `kosmotheka/kosmotheka` | Todo lo hace el maintainer o se hace con él: reescribe el historial |
| **8.6 Linux para todos (D74)** | Snap Store (Ubuntu 20.04 y cualquier Ubuntu) y después Flathub; la página de descargas con los enlaces de verdad | 8.2 y 8.5 |
| **8.7 Lo pendiente de uso** | La lista de [ux-escanear-y-editar.md](../testing/ux-escanear-y-editar.md#pendiente): el botón atrás de Android al editar, los selectores nativos, el orden de los resultados de búsqueda… Se cogen los que molesten en la prueba cerrada | Lo que digan las 12 personas |

**Por estudiar, sin hito:** actualizaciones automáticas en escritorio (plugin `updater`) y la firma de código en Windows (SignPath Foundation es gratis para proyectos libres, pero pide el repositorio abierto). La marca (D72): buscar que el logo esté libre y registrarlo es un trámite del maintainer, sin código.

## Orden

1. **8.1 en cuanto el servidor esté en producción.** Los 14 días de la prueba cerrada son lo que más tarda y no se pueden acortar, así que conviene que empiecen pronto. Lo demás puede llegar durante la prueba como actualizaciones.
2. **8.2 antes de la prueba cerrada**, para que cada versión de prueba salga de una etiqueta y no de una compilación a mano. La parte de Android funciona con el repositorio privado.
3. **8.3 durante la prueba cerrada.** Es la pieza más grande de código de la fase y la más nueva.
4. **8.4 en paralelo**, porque no depende de nada. Las páginas «A» tienen que estar antes de pasar a producción.
5. **8.5 al final**, y con él 8.6. Abrir el repositorio es lo único de la fase que no tiene vuelta atrás.

## Decidido al revisar el plan (16/10/2026)

- **Sin UnifiedPush:** solo FCM. D75 queda revisada y UnifiedPush pasa a «Ideas para el futuro».
- **La guía de instalación, con dos caminos** (D48 precisada): sin Docker, como el servidor oficial, con sus requisitos; y con Docker, con el `Dockerfile` y un `docker-compose` de producción. Es parte de 7.9, pero se escribe junto a la documentación pública (8.4).
- **Dejar todo ordenado para abrir el repositorio, y la documentación publicada, en paralelo a la prueba cerrada.** El maintainer dedica ese tiempo a la ficha de Google Play, que lleva muchos datos; la sesión coordinadora y la de hitos, a ordenar el repositorio y la documentación.
- **D64 cerrada** como reemplazada por D90: un solo repositorio, todo junto.
- **Inventario y orden de trabajo para abrir el repositorio:** [repositorio-publico.md](repositorio-publico.md). Primer encargo: los ficheros de la raíz ([encargo](../historico/encargo-ficheros-raiz.md)).

## Recordatorios para el maintainer (17/10/2026)

- ~~**El centro de datos del VPS**~~: confirmado por el maintainer el 17/10/2026, es Ámsterdam, como dice la política.
- **Cuenta de revisión de Google Play:** hecha el 17/10/2026, con su biblioteca en la nube.
- **El registro de accesos de nginx en producción:** comprobar que no apunta `/v1/metadata/`. Hacer una búsqueda desde la app y después `grep -c "metadata"` sobre el registro de la API: tiene que dar 0. La política lo promete.
- **Las copias automáticas cifradas** (sesión 9.1 de las notas privadas de producción) y una restauración de prueba.

## Pendiente técnico apuntado (17/10/2026)

- **Despliegues sin rsync.** El maintainer no quiere depender de subir la web y el servidor a mano. Hoy se asume; antes de abrir el repositorio, no. Opciones a proponer: un único script en el VPS que actualiza (ya está escrito en sus notas privadas, sesión 9.2) y se lanza con una orden; después, un workflow de GitHub que lo dispara con una clave de despliegue limitada a ese script. La web y la documentación, igual (D90).
- **Hecho (17/10/2026): `#[RateLimit]`, `#[CurrentUser]`, sin `#[IsGranted('ROLE_USER')]` repetidos y el reloj inyectado** ([encargo](../historico/encargo-symfony-atributos.md)). Los límites que se gastan por llegar son atributos con su clave en `RateLimitKey`; los que dependen del flujo (fallos de inicio de sesión, correos, llamadas reales a los proveedores) siguen en `RequestLimiter`. La respuesta 429 sigue siendo `too_many_requests`. Dos cambios pequeños: un cuerpo mal formado ya no gasta el límite, y en metadatos un ISBN inválido o un proveedor sin configurar sí lo gastan (solo se nota pasando de 600 al día).

## Lo que se puede hacer ya, sin ti

- ✅ La primera nota de investigación de 8.3: [notificaciones.md](../research/notificaciones.md) (16/10/2026). Trae dos cosas nuevas: un servidor propio no puede mandar avisos a la app de Google Play, y Play pide además una página web para pedir que se borre la cuenta.
- ✅ La página web para pedir que se borre la cuenta: `web/es/borrar-cuenta.html` y `web/en/delete-account.html` (16/10/2026), sin subir.
- Revisar la ficha de Play y el formulario de seguridad de los datos contra lo que hace hoy la app, y preparar el texto nuevo (8.1), sin subir nada.
- El workflow de 8.2, escrito y comprobado en local hasta donde se pueda. Sin los secretos no publica nada, y solo corre cuando tú hagas push.
- Montar el esqueleto de Starlight y las primeras páginas «A» que no hablan del servidor (8.4).
