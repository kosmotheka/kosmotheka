# Encargo · Los ficheros de la raíz para abrir el repositorio

> Encargo cerrado para la sesión de hitos, escrito el 16/10/2026. Sin commit, sin push, sin cambiar de rama. Es el paso 3 de [plans/repositorio-publico.md](../plans/repositorio-publico.md): léelo antes. Solo toca ficheros de la raíz (y quizá `.github/`). **Todo en inglés**, como el resto del repositorio fuera de `docs/`. No hace falta fase 0, pero si algo de aquí no encaja, para y dímelo.

## Para qué

Quien llegue al repositorio tiene que entender en un minuto qué es Kosmotheka, cómo instalarla, cómo montar su servidor y cómo colaborar. El maintainer lo quiere claro:
- **Colaborar** sigue el procedimiento habitual: *fork*, rama y *pull request*.
- **Montar el servidor propio** se puede, y las pocas limitaciones se dicen de frente.
- La **app oficial** es la nuestra.

## Qué hay que escribir

### 1. `README.md` (rehacer)

El actual se quedó en las fases 1 y 2. Corto, para alguien que llega por primera vez:

- Qué es, en dos frases (la de la landing: «Todo lo que hay en casa, catalogado y compartido»), y los tres puntos fuertes: local-first, bibliotecas compartidas con roles y escanear para saber si ya lo tienes.
- **Capturas:** una de escritorio y una de móvil. Usa las de la web (`web/assets/img/`), referenciadas por ruta relativa; no copies imágenes.
- **Descargar:** Google Play y la página de descargas de kosmotheka.com. Marca como «coming soon» lo que aún no existe; no inventes enlaces.
- **Documentación:** docs.kosmotheka.com, con sus tres partes (usar la app, montar tu servidor, colaborar). Aún no existe: di que llega pronto, sin enlace roto.
- **Montar tu propio servidor:** una línea y el enlace a la guía futura. Las limitaciones van en la guía, no aquí; solo «the official app can connect to any server».
- **Contributing:** enlace a `CONTRIBUTING.md`.
- **Licencia:** AGPL-3.0-or-later para la app y el servidor, MIT para `spec/` (D11). **El nombre y el logo no entran en la licencia**: enlace a `TRADEMARKS.md` (D72).
- La tabla de carpetas del README actual se queda, al día. Las instrucciones de desarrollo pasan a `CONTRIBUTING.md`.

### 2. `CONTRIBUTING.md` (nuevo)

Para alguien que sabe programar pero no conoce el proyecto. Sale de `AGENTS.md`, `server/AGENTS.md` y `docs/development/`; no lo inventes.

- **El procedimiento:**
  1. abrir antes un *issue* para algo grande o que cambie el comportamiento, y para un arreglo pequeño, directamente el PR;
  2. hacer *fork* y una rama desde `main`;
  3. hacer los cambios con las comprobaciones en verde;
  4. abrir un *pull request* que explique el porqué;
  5. pasar la revisión.

  Un PR por tema. Los commits, en inglés y en imperativo o descriptivos, como los del historial.
- **Montar el entorno:** la app (Rust, Node, Tauri; el maintainer usa Distrobox, pero no es obligatorio: di qué se necesita) y el servidor (solo Docker, `make up`, `make check`). Enlaza a `docs/development/setup.md` y `server.md` mientras no exista la documentación pública.
- **Las comprobaciones que pasa la CI**, por parte: Rust, la interfaz, `spec/` y el servidor. Toma los comandos de la tabla de `AGENTS.md`.
- **Las reglas que de verdad importan aquí**, en pocas líneas cada una y con el porqué:
  - todo cambio del catálogo es una operación;
  - un campo nuevo toca a la vez la migración, `schema.rs`, `spec/` y el PHP;
  - las reglas viven en `spec/` con casos que pasan Rust y PHP;
  - la sincronización tiene que converger, con un test;
  - lo que decide quién recibe una biblioteca se prueba primero con el caso hostil, visto fallar;
  - el servidor nunca se fía de un id del cliente;
  - ningún texto escrito a mano en la interfaz: i18n en inglés y español;
  - una línea en `whats-new.json` si una persona lo notaría;
  - los tests no usan la red;
  - nunca secretos en el repositorio.
- **Idioma:** el código, en inglés; `docs/` está en español mientras se traduce, y una pregunta en español o en inglés es bienvenida.
- **Asistentes de IA:** `AGENTS.md` tiene las instrucciones para quien colabore con uno. Se aceptan contribuciones hechas con ayuda de IA si quien abre el PR las entiende y responde de ellas.
- **Seguridad:** nunca en un *issue* público; ver `SECURITY.md`.
- **Licencia de las contribuciones:** lo que se aporta entra con la licencia de la parte que toca (AGPL o MIT en `spec/`). Sin CLA. **Marca esta frase para que la confirme el maintainer**, con un comentario HTML `<!-- TO CONFIRM: ... -->`.

### 3. `CODE_OF_CONDUCT.md` (nuevo)

Contributor Covenant 2.1, tal cual, con el contacto `info@kosmotheka.com` (D80). Coge el texto oficial; no lo reescribas. Si no lo puedes obtener sin red, dímelo y lo traigo yo.

### 4. `TRADEMARKS.md` (nuevo)

Política de marca corta, según D72 (que sigue en `propuesta`: **márcalo con `<!-- DRAFT: D72 not accepted yet -->` arriba**):
- El nombre «Kosmotheka» y el logo no entran en la AGPL ni en la MIT.
- **Se puede:** hablar del proyecto, enlazarlo, usar el nombre y el logo para referirse a él, decir que tu servidor «funciona con Kosmotheka».
- **No se puede:** publicar una app o un servicio modificado con ese nombre o ese logo, ni de forma que parezca el oficial. Un *fork* que se publique cambia el nombre y el icono.
- La app oficial es la de Google Play y las descargas de kosmotheka.com; el servidor oficial, `kosmotheka.app`.
- Preguntas: `info@kosmotheka.com`.

Inspírate en la de Rust o la de Mozilla, pero en corto (una pantalla).

### 5. `SECURITY.md` (revisar, sin reescribir)

Hoy enlaza `docs/audits/amenazas.md`, que se publicará (D90). Déjalo, y añade una línea que diga que el texto está en español y se traducirá. Nada más.

### 6. `.github/` (opcional)

Si cabe sin complicarse: una plantilla de PR corta (qué cambia, por qué, comprobaciones hechas, novedades sí o no) y dos plantillas de *issue* (fallo, propuesta). Propónmelo antes de escribirlo si dudas.

## Reglas

- **No inventes** comandos, URLs ni datos: sale del repositorio o se marca como pendiente.
- Nada de `docs/` cambia en este encargo, salvo que te lo pida.
- Frases cortas, tono cercano y sin vender humo. Sin emojis.
- No deshagas nada con git. El maintainer puede tener `tauri dev` abierto: no lo cierres.

## Al terminar

Los enlaces relativos de los ficheros nuevos apuntan a ficheros que existen: compruébalo con un script y pega su salida. `npm run format:check` (si Prettier mira la raíz). Mándame un informe con lo que has marcado para el maintainer.
