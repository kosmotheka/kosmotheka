# Encargo · Documentación para quien colabora, primera tanda

> Encargo cerrado para la sesión de hitos, escrito el 17/10/2026. Sin commit, sin push. **Solo `site/src/content/docs/es/contribute/`.** No toques `using-the-app/` ni `run-your-server/`, ni nada de `en/`: otra sesión escribe a la vez en `en/using-the-app/`. Índice general en [docs/plans/documentacion-publica.md](../plans/documentacion-publica.md), parte 3.

## Para quién

Alguien que programa y quiere entender el proyecto, arreglar algo o añadir algo. **No es quien usa la app**, así que aquí sí valen los términos técnicos (operación, reloj lógico híbrido, conformidad). Pero cada uno se explica la primera vez que sale, con una frase, y siempre se dice **por qué** se hizo así. Frases claras y cortas, como en el resto del sitio.

Esta parte **no se escribe desde cero**: el material está en `docs/`, como documentos de trabajo. Hay que **ordenarlo y resumirlo para alguien de fuera**:
- sin el historial de cómo se llegó a cada cosa (eso queda en `DECISIONS.md`, que se enlaza);
- sin fechas de trabajo, nombres de hitos ni referencias a sesiones o agentes;
- sin nada del servidor oficial: ni su máquina, ni sus rutas, ni sus cuentas.

Español primero (como la guía de usuario); el inglés va después.

## Las páginas

Nombres de archivo en inglés, que se repetirán en `en/`.

1. **`overview.md` · Las piezas y cómo encajan.**
   - La app (Tauri: interfaz en React y núcleo en Rust), `crates/domain`, `spec/`, el servidor opcional (Symfony) y la web.
   - Qué pasa al añadir un libro, desde el botón hasta otro dispositivo: operación, base local, emparejados por la red de casa y, con la nube, el servidor.
   - Local primero, y el servidor como autoridad cuando lo hay.
   - Material: `AGENTS.md` (Project, Stack, Repository layout) y `README.md`.
2. **`setup-app.md` · Preparar el entorno: la app.**
   - Requisitos, el contenedor `library-dev`, arrancar en el escritorio y en Android, y las comprobaciones.
   - Material: `docs/development/setup.md` y la tabla de comandos de `AGENTS.md`.
   - Distrobox es como lo hace el maintainer, no la única forma: dilo, y da también qué hace falta tener instalado sin él.
3. **`setup-server.md` · Preparar el entorno: el servidor.**
   - Docker Compose, el `Makefile`, la base de pruebas, las fixtures, el correo de desarrollo y `make check`.
   - Material: `docs/development/server.md`, `docs/development/fixtures.md`, `docs/development/correo.md` y `server/AGENTS.md`.
   - Los servicios de fuera son opcionales: Google, las claves de los proveedores (`docs/development/servicios-externos.md`, `google-oauth.md`). Di que existen y qué variable activa cada uno, **sin ningún valor**.
4. **`how-to-contribute.md` · Cómo contribuir: estilo, pruebas y commits.**
   - Lo importante de `CONTRIBUTING.md` y de las reglas de `AGENTS.md` y `server/AGENTS.md`, ordenado para quien llega:
     - el código en inglés;
     - toda escritura es una operación;
     - la regla de los cuatro sitios al añadir un campo;
     - los casos de conformidad;
     - la prueba del caso hostil antes del arreglo;
     - el servidor nunca se fía de un id del cliente;
     - las pruebas sin red;
     - la línea de novedades;
     - ningún texto de interfaz escrito a mano;
     - nada de secretos.
   - **Una sección «Añadir un campo o una entidad», paso a paso**: migración, `schema.rs`, `operation.schema.json`, casos de conformidad y `CatalogEntity.php`.
   - Enlaza `CONTRIBUTING.md` en lugar de repetirlo entero.
5. **`glossary.md` · Glosario.** La tabla del glosario de `AGENTS.md`, con su columna «en el código», y una línea de explicación por término cuando haga falta.

Y **`index.md`**: ponlo al día con estas páginas, y deja dicho en una línea qué viene después (el motor de sincronización, las operaciones, la seguridad y el método de auditoría). Ordena el menú con `sidebar.order`.

## Lo que pesa

- **Todo comprobado en el repositorio de hoy.** Si un documento de trabajo no coincide con el código (un comando, una ruta, un nombre), manda el código, y apúntalo en el informe.
- **Ningún secreto, ni de ejemplo.** Ni valores de `.env.local` (no lo abras), ni claves de desarrollo de Mercure, ni contraseñas de fixtures que no estén ya en el repositorio. Si una variable hace falta, se nombra y se dice qué es.
- **Enlaces al código** con la URL pública: `https://github.com/kosmotheka/kosmotheka/blob/main/<ruta>`, como ya hace `index.md`.

## Al terminar

- `npm run build` en `site/`.
- **El informe:**
  - cada página con su título y su número de palabras;
  - lo que no coincidía entre los documentos de trabajo y el código;
  - lo que dejaste fuera por ser interno.
