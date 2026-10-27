# Plan: la papelera (D105)

> Empezado el 27/10/2026 en la rama `feat/trash`. Lo decidido está en D105 ([DECISIONS.md](../DECISIONS.md)); aquí, el orden del trabajo. **Cada paso es un commit; al terminarlo se marca aquí**, para que otra sesión sepa por dónde va. La rama no se fusiona hasta tenerlo todo hecho y probado en dispositivos: va en una versión propia, porque todos los dispositivos de una biblioteca necesitan la nueva.

## Resumen para retomar

- Cuatro cosas van a la papelera: ficha (`editions`), ejemplar (`copies`), etiqueta (`tags`) y saga (`series`). Campos `trashed_at` (fecha y hora UTC) y `trashed_by` (id de `member`). Restaurar = los dos a `null`.
- Editores y administradores ven, mandan y restauran. Eliminar definitivamente: administrador siempre; editor, a los 30 días. La nube, a los 60 (cron con `CloudAuthor`), y solo eso.
- 30 días desde la más tardía de `trashed_at` y «cuándo lo vio este dispositivo por primera vez» (local). La nube igual, con «cuándo le llegó la fila».
- Nada de Gedmo ni filtros de Doctrine: en el servidor son columnas normales.

## 0. Decisión y documentación de partida

- [x] D105 en `DECISIONS.md`.
- [x] Este plan.
- [x] Glosario de `AGENTS.md` (`trash`, `trashed_at`/`trashed_by`, `restore`, `purge`).
- [x] `server/AGENTS.md`: nunca un filtro de borrado lógico.
- [x] ROADMAP: «Papelera» decidida y en curso; la selección múltiple enlazada con «Añadir varios resultados».

## 1. Especificación y dominio (los casos hostiles primero, vistos fallar)

- [x] Tipo de campo nuevo de fecha y hora en UTC (hoy solo hay `Date`): en `crates/domain/src/schema.rs`, su validación y su forma en `spec/operation.schema.json`. *Hecho (1a): `FieldType::Timestamp`, una sola escritura `YYYY-MM-DDTHH:MM:SS.mmmZ`.*
- [x] `trashed_at` y `trashed_by` en `editions`, `copies`, `tags` y `series`: `schema.rs`, `spec/operation.schema.json` y `schemaVersion` en `spec/sync.json` (un test lo mantiene igual a las migraciones). *Hecho (1a): `schema::TRASHABLE`, código `trash_pair` (los dos campos juntos), 11 casos en `operation-validation.json`, `schemaVersion` 36. `trashed_by` sin clave foránea, como `devices.member_id`.*
- [x] Reglas en `spec/` y casos en `spec/conformance/` (`permissions.json`, `operation-validation.json`), que corren Rust y PHP:
  - [x] mandar a la papelera y restaurar: quien puede borrar esa cosa (editor, administrador);
  - [x] restaurar pone los dos campos a `null` a la vez; poner uno sin el otro no vale;
  - [x] al mandar a la papelera, `trashed_by` es la persona del dispositivo que firma; al crear una fila ya en la papelera (`backup.restore`), una persona de la biblioteca;
  - [x] eliminar una de las cuatro: administrador siempre; editor, solo si `trashed_at` tiene 30 días respecto a la marca de la operación; la nube, solo si tiene 60; sin papelera, nadie: queda como petición;
  - [x] casos hostiles: editor que elimina sin papelera o antes de tiempo; `trashed_by` de otra persona; quien solo lee y restaura; la nube que elimina antes de 60 días o algo que no está en la papelera, o que intenta mandar a la papelera o restaurar.
- [x] Juez en Rust con esas reglas. *Hecho (1b): regla 16 en `spec/README.md` (y excepciones en las reglas 3 y 12), `LibraryView::trashed_since` (lo que cuelga de qué, en `permissions.rs`), 10 casos «trash:» en `permissions.json`. Dos casos antiguos de D45 pasan de etiquetas a editoriales, que se siguen borrando directamente.*
- [x] `docs/architecture/bateria-de-acceso.md` con cada test hostil.

## 2. App (Rust)

- [x] Migración SQLite: columnas (`0036_trash.sql`, hecha en 1a porque el test de `schemaVersion` la pide). Índices, si las consultas los piden, en el paso 2.
- [x] Operaciones: `*.trash`, `*.restore` y `*.purge` para ficha, ejemplar, etiqueta y saga (intenciones nuevas en `operations.md`); `*.purge` hace lo que hoy hace el borrado, con la misma cascada. *Hecho (2a): `trash.rs` (`send`, `restore`, `purge`, `list`); intenciones `book.trash`, `copy.trash`, `tag.trash`, `series.trash`, `trash.restore`, `trash.purge`. Mover una ficha a otra biblioteca la manda a la papelera en la de origen (`book.move_out`), y fusionar dos etiquetas al renombrar manda la vaciada.*
- [x] Ficha: mandarla esconde sus ejemplares, pero en la papelera sale solo la ficha; un ejemplar mandado antes por separado sigue aparte; la obra se esconde cuando todas sus ediciones están en la papelera. *Hecho (2a y 2b).*
- [x] Todas las consultas ignoran lo que está en la papelera, con un test cada una: catálogo, búsqueda, filtros, contadores, «¿Lo tengo?», «Me falta», sagas (barras y huecos), etiquetas de una obra y su lista, marcas, CSV, «Alta rápida», foto de balda, duplicados al añadir. *Hecho (2b): catálogo y búsqueda, filtros, ubicaciones, contadores de bibliotecas, ejemplares de la ficha, etiquetas y su lista, sagas (lista, detalle y cuentas; una obra con todas sus ediciones en la papelera no sale), «necesita», CSV, duplicados por ISBN y código, y buscar una etiqueta por nombre al escribirla. Las portadas, las copias de seguridad, el historial y la purga lo siguen viendo todo. «¿Lo tengo?», «Alta rápida» y la foto de balda pasan por las mismas funciones.*
- [x] Comandos: listar la papelera de la biblioteca activa (con quién, cuándo y días que quedan), restaurar varias, eliminar varias (una operación por acción, todo o nada). *Hecho (2a): `list_trash`, `restore_from_trash`, `purge_from_trash`.*
- [x] Duplicados: código o ISBN en la papelera → «Restaurar» (con lo que trae); nombre de etiqueta o saga igual → lo mismo; «Crear nueva» elimina la otra (solo quien administra). *Hecho en Rust (2c): `trashed_edition_id` junto a `existing_edition_id` en la búsqueda por título, el ISBN y el código; `trashed_named` para etiquetas y sagas. El diálogo es del paso 4. Una etiqueta escrita en el formulario con el nombre de una que está en la papelera crea una nueva (no la enlaza a la invisible); ofrecer restaurarla ahí, a decidir en el paso 4.*
- [x] Borrado automático a los 30 días: al abrir y al sincronizar, solo con permiso; tabla local de «visto en la papelera por primera vez» (nunca sincronizada, fuera de las copias). *Hecho (2c): tabla local `trash_seen` (en la migración 0036, aún sin publicar), `trash::expire` y el comando `expire_trash` (la pantalla lo llamará al abrir y al sincronizar). Solo si el juez lo permitiría: el dispositivo de un lector nunca crea peticiones por esto.*
- [x] Peticiones: borrar desde un lector, aceptado, va a la papelera. *Hecho (2a): el resumen dice «borrar», y una petición de mandar a la papelera lo que ya está deja de aplicar.*
- [x] Copias de seguridad: guardan la papelera; al restaurar, el resumen cuenta lo que trae en ella. *Hecho (2c): `RestoredLibrary.trashed`; los libros y ejemplares contados son los que están fuera. El juez ya no pide que `trashed_by` exista al crear filas a un administrador (al restaurar, las personas llegan en la misma operación); a un editor, sí.*
- [ ] Historial: «Movido a la papelera por…» y «Restaurado». *Las intenciones ya existen (`*.trash`, `trash.restore`, `trash.purge`, `trash.expire`, `book.move_out`); faltan sus textos en la interfaz (paso 4).*
- [x] Tests de sincronización con `assert_converged`: restaurar frente a editar; restaurar frente a eliminar (gana eliminar); dos que eliminan a la vez; mandar sin conexión y sincronizar; etiqueta eliminada mientras su libro está en la papelera; copia de seguridad antigua restaurada. *Hecho (2d), en `sync.rs`: editar sin conexión una ficha que otro mandó a la papelera (vuelve con la edición), restaurar frente a eliminar (gana eliminar), dos que eliminan a la vez, y una etiqueta eliminada mientras su libro estaba en la papelera (vuelve sin ella). La copia de seguridad antigua, en `backup.rs` (2c).*
- [x] Listas fijas de `AGENTS.md` (`ORDER`, `CONTENTS`, `LIBRARY_CONTENTS_CHILDREN_FIRST`): comprobar que siguen bien. *Comprobado (2d): ninguna entidad nueva, así que `ORDER`, `CONTENTS` y `LIBRARY_CONTENTS_CHILDREN_FIRST` no cambian. La tabla local `trash_seen` se borra al quitar una biblioteca (`ops::delete_library`), pero no al reconstruirla desde su registro, que no debe dar más días a la papelera.*

## 3. Servidor

> **Ojo:** desde el paso 1a, los casos nuevos de `spec/conformance/` fallan en los tests del servidor (no conoce los campos ni `trash_pair`) hasta que se haga este paso. Es esperado en esta rama; no se fusiona hasta tenerlo.

- [ ] Migración de Doctrine con las columnas (`timestamptz` y el id de la persona); entidades `Edition`, `Copy`, `Tag` y `Series` en `src/Entity/Catalog/`; `CatalogEntity.php`.
- [ ] `CatalogJudge` con las reglas nuevas; los mismos casos de `spec/conformance/`.
- [ ] Cron de los 60 días: un comando (`#[AsCommand]`) que firma las eliminaciones con `CloudAuthor`; guarda cuándo le llegó cada fila a la papelera.
- [ ] `make check` en verde.

## 4. Interfaz

- [ ] Pieza de selección múltiple reutilizable: casilla en el ordenador, pulsación larga en el móvil; barra con el número, «Seleccionar todo» y las acciones; atrás sale de la selección (`useBackGuard`).
- [ ] Pantalla «Papelera» en el menú de la biblioteca (editores y administradores): apartados Fichas · Ejemplares · Etiquetas · Sagas; cada fila con qué es, cuándo, quién y «Se eliminará en N días»; vacía, explica qué es.
- [ ] «Restaurar» y «Eliminar definitivamente» (con confirmación; solo sale para quien puede).
- [ ] Al borrar: aviso flotante «Movido a la papelera · Deshacer»; las confirmaciones de borrar dicen «Estará 30 días en la papelera».
- [ ] Diálogos de duplicados (punto 8 de D105).
- [ ] i18n es y en; versión simulada; Playwright en el ordenador y en el móvil (pulsación larga con toques de verdad).

## 5. Documentación

- [ ] Arquitectura: `operations.md` (intenciones), `sync.md` (la papelera frente al borrado de verdad, los relojes, la nube), `data-model.md` (los campos y la tabla local).
- [ ] Guía (es, en): página nueva «La papelera» (qué entra y qué no, 30 días, quién la ve, restaurar y eliminar, lo que no vuelve igual, las copias de seguridad que dan 30 días más, la nube a los 60) y las páginas que borran (añadir un libro, ejemplares, etiquetas, sagas, personas y permisos, copias de seguridad), más preguntas frecuentes.
- [ ] Guía para colaborar: `contribute/sync-engine.md` y `contribute/security.md`.
- [ ] La Ayuda de la app, si merece tema propio.
- [ ] Novedades: una línea para personas, que avisa de que todos los dispositivos de la biblioteca deben actualizar.
- [ ] Comprobado que no cambia lo que sale del dispositivo ni la política de privacidad.

## 6. Cierre

- [ ] Prueba en el móvil y el portátil del maintainer: borrar, deshacer, restaurar, eliminar, pulsación larga, pasar por la wifi y por la nube.
- [ ] ROADMAP: la papelera, hecha; este plan pasa a `docs/historico/` o se cierra aquí.
