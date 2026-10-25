---
title: "Cómo contribuir: estilo, pruebas y commits"
description: El procedimiento, las reglas que más afectan a un cambio y cómo añadir un campo o una entidad paso a paso.
sidebar:
  order: 4
---

Esta página ordena lo que hace falta saber para que un cambio se acepte a la primera. El procedimiento completo, en inglés, está en [`CONTRIBUTING.md`](https://github.com/kosmotheka/kosmotheka/blob/main/CONTRIBUTING.md), y las reglas completas en [`AGENTS.md`](https://github.com/kosmotheka/kosmotheka/blob/main/AGENTS.md) y [`server/AGENTS.md`](https://github.com/kosmotheka/kosmotheka/blob/main/server/AGENTS.md). Aquí no se repiten enteros: se explica el porqué de las que más tropiezos dan. Si algo de aquí es incorrecto o confuso, avísanos: sería una buena primera contribución.

## El procedimiento

1. **Abre un *issue* primero** si el cambio es grande o altera cómo se comporta algo, para acordarlo antes de que gastes tiempo. Para un arreglo pequeño, ve directo al *pull request*.
2. **Haz un *fork*** y una rama desde `main`.
3. **Haz el cambio** y ejecuta las comprobaciones hasta que pasen ([la app](../setup-app/#las-comprobaciones), [el servidor](../setup-server/#las-comprobaciones)).
4. **Abre el *pull request*** y explica **por qué** hace falta, no solo qué hace.
5. **Pasa la revisión.** Es posible que pidamos cambios.

Un *pull request* por tema.

**Ramas y *pull requests*:**

- **El nombre de la rama** lleva el tipo de cambio de [Conventional Commits](https://www.conventionalcommits.org/) y unas palabras en inglés, en minúsculas y con guiones: `feat/printed-isbn`, `fix/invitation-twice`, `docs/shelf-photo`. Tipos: `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`, `chore`.
- **El título del *pull request*** tiene la misma forma, con la parte que toca: `feat(app): read the ISBN printed on a back cover`. Partes: `app`, `ui`, `domain`, `spec`, `server`, `site`, `web`, `ci`.
- **Todo lo de un cambio va en su rama**, también su documentación: la guía, `docs/` y el registro de decisiones.
- La [plantilla del *pull request*](https://github.com/kosmotheka/kosmotheka/blob/main/.github/pull_request_template.md) tiene la lista de lo que hay que comprobar según la parte que tocas: marca lo que aplica y borra lo demás.
- Los *pull requests* se fusionan con «Rebase and merge»: cada commit llega a `main` tal cual, así que cada uno tiene que entenderse solo.

Quien participa acepta el [código de conducta](https://github.com/kosmotheka/kosmotheka/blob/main/CODE_OF_CONDUCT.md). Para un fallo de seguridad **no abras un issue público**: sigue [`SECURITY.md`](https://github.com/kosmotheka/kosmotheka/blob/main/SECURITY.md).

Si trabajas con un asistente de IA, se acepta el resultado si quien abre el *pull request* entiende el cambio y responde de él. `AGENTS.md` lleva las instrucciones para el asistente.

## Idioma, commits y licencia

- **Todo el repositorio está en inglés:** código, identificadores, comentarios, mensajes de commit, README, issues y los textos de la interfaz en su idioma original. La excepción es `docs/`, que está en español por ahora. Una pregunta en español o en inglés es bienvenida.
- **Commits:** en inglés, en imperativo o descriptivos, como el historial que ya hay. Explican el porqué cuando no es evidente.
- **Comentarios en el código:** cortos, y dicen **por qué**, nunca repiten lo que hace el código.
- **Licencia de lo que aportas:** la de la parte que tocas. AGPL-3.0 o posterior para la app y el servidor, y MIT para `spec/`. No hay que firmar ningún acuerdo.

## Las reglas que más afectan a un cambio

Cada una lleva su razón. Entenderla ayuda más que memorizarla.

- **Toda escritura en el catálogo es una operación.** Construye una lista de cambios y llama a `ops::record` dentro de una transacción; **nunca** hagas `INSERT`, `UPDATE` o `DELETE` directamente en las tablas del catálogo. *Por qué:* los dispositivos se intercambian operaciones, no filas, y eso es lo que hace que la sincronización converja. La única excepción es `sync::merge`, que aplica las que llegan de otros dispositivos.
- **La lógica del dominio va en Rust** (`crates/domain`), no en la interfaz. React llama a comandos de Tauri y no ejecuta SQL.
- **Un campo o una entidad nuevos tocan varios sitios a la vez.** Está explicado más abajo, paso a paso. *Por qué:* son dos implementaciones (Rust y PHP) de un mismo formato; si olvidas una, dejan de ponerse de acuerdo.
- **Las reglas de una operación se definen en `spec/`** y se cubren con **casos de conformidad** que ejecutan tanto la implementación en Rust como la de PHP. Un *caso de conformidad* es una operación de ejemplo con el resultado que **toda** implementación debe dar. *Por qué:* es lo que impide que las dos se separen, y lo que permite que alguien escriba un cliente o un servidor compatible. Un cambio de regla va primero a `spec/`, con sus casos, y luego a las dos implementaciones.
- **La sincronización tiene que converger.** Un cambio en cómo se aplican las operaciones necesita una prueba en `sync.rs` que simule varios dispositivos y compruebe, con `assert_converged`, que acaban iguales.
- **Quién recibe una biblioteca se prueba primero contra un par hostil.** Para cualquier regla que decida qué dispositivo o cuenta recibe, conserva o pierde datos de una biblioteca (emparejar, compartir, retirar el acceso, la tabla de acceso de la nube), escribe **antes del arreglo** la prueba del ataque y **comprueba que falla**: una app modificada, un par que conoce los ids, un aviso falsificado. *Por qué:* una revisión posterior es la segunda línea de defensa, no la primera.
- **El servidor nunca se fía de un id del cliente.** Los ids de biblioteca, operación y dispositivo de una petición se **comparan**, nunca sirven para buscar ni para decidir el acceso. El acceso sale solo de la cuenta, de la tabla de acceso y de los datos propios del servidor.
- **Los servicios de fuera son opcionales y van detrás de interfaces.** La app tiene que seguir funcionando cuando no están.
- **Las pruebas no usan la red.** Analizan respuestas grabadas en `app/src-tauri/tests/fixtures/`. Las que necesitan red son `#[ignore]` y se llaman `live_*`.
- **En Rust, no mantengas el acceso a la base abierto durante un `.await`.** El `MutexGuard` de la base no debe cruzar una espera: lee lo que necesites en un bloque, suelta el guardián y espera después (mira `commands::lookup_isbn`).
- **Ningún texto de la interfaz se escribe a mano.** Usa claves de i18n y añade la traducción en inglés y en español.
- **Un cambio que una persona notaría necesita su línea en `app/src/whats-new.json`.** Una línea en español y otra en inglés, bajo `next`, escrita para personas (nada de internos ni de refactorizaciones). Si el cambio modifica lo que sale del dispositivo, lleva además un interruptor ligado al ajuste que lo controla. `npm run check:whats-new` falla si la versión de la app no tiene entrada.
- **Ningún secreto en el repositorio:** ni claves, ni contraseñas, ni tokens, ni de ejemplo. El repositorio es público y su historial tiene que estar limpio.

### Reglas del servidor

Además de las anteriores, en `server/` (más en `server/AGENTS.md`):

- **Si Symfony ya tiene el mecanismo, usa el de Symfony.** Cuerpos de petición como DTO con `#[MapRequestPayload]`, límites de peticiones con `#[RateLimit]`, el usuario con `#[CurrentUser]`, el reloj con `ClockInterface`.
- **Un método devuelve su tipo o lanza**: nunca `Foo|string` ni un valor nulo para indicar un fallo. Y excepciones propias, no `\RuntimeException` a secas.
- **Cada clase va en una carpeta que nombra su papel** (`Controller/`, `Entity/`, `Dto/`, `Enum/`…). Ninguna en la raíz de `src/` salvo `Kernel.php`.
- **`make check` tiene que pasar**, con PHPStan en el nivel máximo y sin archivo de base.

## Añadir un campo o una entidad, paso a paso

Es el cambio que más veces se hace y donde más fácil es olvidar algo. Un campo nuevo (por ejemplo, un dato más de los juegos de mesa) pasa por estos sitios, **juntos, en el mismo cambio**:

1. **Una migración de SQLite** en `app/src-tauri/migrations/`: un archivo `NNNN_nombre.sql` con el número siguiente al último, y su línea `M::up(include_str!(…))` en `app/src-tauri/src/db.rs`. Sube también `schemaVersion` en `spec/sync.json`: una prueba comprueba que coincide con el número de migraciones, porque dos dispositivos con versiones distintas no sincronizan. La app aplica las migraciones al arrancar, y a las bases de los usuarios que ya existen también. *Por qué:* nadie va a empezar de cero; el esquema tiene que poder evolucionar sobre datos reales.
2. **`crates/domain/src/schema.rs`**: añade el campo a la lista de campos de su entidad con `field("nombre", FieldType::…, obligatorio)`. Aquí se declara qué tipo de dato es y si es obligatorio.
3. **`spec/operation.schema.json`**: el mismo campo, en el JSON Schema. *Por qué:* es lo que ve cualquier otro cliente o servidor, y el que valida `npm run check:spec`.
4. **Casos de conformidad** en `spec/conformance/`: una operación válida con el campo nuevo, y las inválidas que correspondan (tipo equivocado, un obligatorio a `null`…), en `operation-validation.json`. Si el campo cambia cómo se fusionan operaciones o los permisos, añade también un caso en `merge.json` o en `permissions.json`.
5. **`server/src/Enum/CatalogEntity.php`**: el espejo en PHP de `schema.rs`. Las pruebas del servidor ejecutan los mismos casos de conformidad y avisan de la mayoría de los desajustes.
6. **La clase Doctrine de esa entidad**, en `server/src/Entity/Catalog/`, con su columna, y **una migración del servidor** (`make migration` y luego `make migrate`; quita los comentarios que genera y rellena `getDescription()`). *Por qué:* el servidor guarda el catálogo en PostgreSQL con Doctrine, además de describirlo en `CatalogEntity`.

Después, lo que usa el campo: leerlo y escribirlo en `app/src-tauri/src/catalog.rs` (o donde toque), exponerlo en `app/src/lib/api.ts` y en el formulario, y los textos con claves de i18n en `es.json` y en `en.json`.

Para una **entidad nueva** (una tabla nueva), todo lo anterior y además:

- el caso nuevo en el `enum Entity` de `schema.rs` y en `CatalogEntity` del servidor;
- su sitio en las listas de entidades de la app, que tienen un tamaño fijo en el código: el orden de sincronización (`ORDER` en `sync.rs`, padres antes que hijos), el de las copias de seguridad (`CONTENTS` en `backup.rs`) y el de borrado de una biblioteca (`LIBRARY_CONTENTS_CHILDREN_FIRST` en `ops.rs`, hijos antes que padres);
- una prueba en `sync.rs` de que converge entre dispositivos.

Comprueba al final con `cargo test`, `npm run check:spec` y `make check`.

## Mejorar esta documentación

Esta guía está hecha con [Starlight](https://starlight.astro.build), un tema de documentación de Astro, a partir del Markdown de `site/src/content/docs/`. Es HTML estático: no se ejecuta nada en un servidor. Para verla mientras la editas:

```bash
cd site
npm ci
npm run dev        # http://localhost:4321, se recarga al guardar
npm run build && npm run check:links   # lo que comprueba la CI
```

Cada página se escribe primero en español (`es/`) y después en inglés (`en/`), con la misma ruta. Los enlaces entre páginas son `../página/`. Los detalles están en `site/README.md`. La carpeta `docs/` no es esta guía: es el diario de desarrollo, en español.

## Antes de abrir el *pull request*

- Ejecuta las comprobaciones que correspondan a lo que tocaste.
- Pasa por la lista: ¿lleva prueba? ¿una línea de novedades, si se nota? ¿textos con i18n en los dos idiomas? ¿algo de `docs/` que haya que actualizar?
- Si cambia una **decisión de arquitectura**, se apunta en el [registro de decisiones](https://github.com/kosmotheka/kosmotheka/blob/main/docs/DECISIONS.md), con su estado. Una decisión que se cambia **en silencio** es la manera de que nadie sepa, tres meses después, qué se decidió ni por qué.

Los nombres que se repiten en el código (obra, ejemplar, miembro…) están en el [glosario](../glossary/): mira allí antes de poner un nombre nuevo.
